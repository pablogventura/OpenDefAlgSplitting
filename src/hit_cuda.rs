//! Cálculo de information gain en GPU (CUDA). Opcional con feature "cuda".
//! Solo soporta operaciones binarias en la ruta GPU; si hay aridad distinta se hace fallback a CPU.

use crate::first_order::relops::Operation;
use crate::hit::{information_gain_from_counts, TupleHistory};
use std::collections::HashMap;

const KERNEL_SRC: &str = r#"
extern "C" __global__ void ig_counts(
    const int* histories,
    const int* history_len,
    const int* index_map,
    const int* in_target,
    const int* op_tables,
    const int* ti,
    int n_tuples,
    int max_history,
    int universe_len,
    int n_candidates,
    int* count_flat
) {
    int cand_id = blockIdx.x * blockDim.x + threadIdx.x;
    int tuple_id = blockIdx.y * blockDim.y + threadIdx.y;
    if (cand_id >= n_candidates || tuple_id >= n_tuples) return;

    int ti0 = ti[cand_id * 2];
    int ti1 = ti[cand_id * 2 + 1];
    int idx0 = histories[tuple_id * max_history + ti0];
    int idx1 = histories[tuple_id * max_history + ti1];
    int table_base = cand_id * universe_len * universe_len;
    int result = op_tables[table_base + idx0 * universe_len + idx1];

    int hist_len = history_len[tuple_id];
    int map_idx = tuple_id * universe_len + result;
    int pos = index_map[map_idx];
    int group_idx = (pos >= 0) ? pos : hist_len;

    int tgt = in_target[tuple_id];
    int count_idx = cand_id * (max_history + 1) * 2 + group_idx * 2 + tgt;
    atomicAdd(count_flat + count_idx, 1);
}
"#;

/// Intenta obtener el mejor (op, ti) por information gain usando GPU.
/// Solo se usa si todos los candidatos son operaciones binarias y los datos caben en i32.
/// Devuelve None si no hay GPU, falla, o hay candidatos no binarios.
#[cfg(feature = "cuda")]
pub fn best_candidate_ig_cuda(
    tuples: &[TupleHistory],
    cand_list: &[(Operation, Vec<usize>)],
    n: usize,
    in_total: usize,
) -> Option<((Operation, Vec<usize>), f64)> {
    use cudarc::driver::safe::CudaDevice;
    use cudarc::driver::LaunchAsync;
    use cudarc::driver::LaunchConfig;
    use cudarc::nvrtc::compile_ptx;
    use std::sync::Arc;

    if tuples.is_empty() || cand_list.is_empty() {
        return None;
    }
    // Solo soportamos aridad 2 en GPU
    if cand_list.iter().any(|(op, ti)| op.arity != 2 || ti.len() != 2) {
        return None;
    }

    let dev = Arc::new(CudaDevice::new(0).ok()?);

    // Construir universo: todos los i64 que aparecen en histories y en op tables
    let mut universe_set = std::collections::HashSet::new();
    for th in tuples {
        for &x in &th.history {
            universe_set.insert(x);
        }
    }
    for (op, _) in cand_list {
        for (args, &v) in &op.op {
            for &x in args {
                universe_set.insert(x);
            }
            universe_set.insert(v);
        }
    }
    let mut universe: Vec<i64> = universe_set.into_iter().collect();
    universe.sort_unstable();
    let universe_len = universe.len();
    if universe_len == 0 || universe_len > (i32::MAX as usize) {
        return None;
    }
    let u_len = universe_len as i32;
    let val_to_idx: HashMap<i64, i32> = universe
        .into_iter()
        .enumerate()
        .map(|(i, v)| (v, i as i32))
        .collect();

    let max_history = tuples
        .iter()
        .map(|th| th.history.len())
        .max()
        .unwrap_or(0);
    if max_history == 0 || max_history > (i32::MAX as usize) {
        return None;
    }
    let max_h = max_history as i32;
    let n_tuples = tuples.len() as i32;
    let n_candidates = cand_list.len() as i32;

    // histories [n_tuples][max_history] rellenado con -1
    let mut histories: Vec<i32> = vec![-1; (n_tuples as usize) * (max_history as usize)];
    let mut history_len: Vec<i32> = vec![0; n_tuples as usize];
    let mut index_map: Vec<i32> = vec![-1; (n_tuples as usize) * universe_len];
    let mut in_target: Vec<i32> = vec![0; n_tuples as usize];

    for (t, th) in tuples.iter().enumerate() {
        history_len[t] = th.history.len() as i32;
        in_target[t] = if th.in_target { 1 } else { 0 };
        for (i, &val) in th.history.iter().enumerate() {
            if let Some(&idx) = val_to_idx.get(&val) {
                histories[t * (max_history as usize) + i] = idx;
                index_map[t * universe_len + (idx as usize)] = i as i32;
            }
        }
    }

    // op_tables [n_candidates][universe_len*universe_len], ti [n_candidates][2]
    let mut op_tables: Vec<i32> = vec![-1; (n_candidates as usize) * universe_len * universe_len];
    let mut ti: Vec<i32> = vec![0; (n_candidates as usize) * 2];
    for (c, (op, indices)) in cand_list.iter().enumerate() {
        ti[c * 2] = indices[0] as i32;
        ti[c * 2 + 1] = indices[1] as i32;
        let base = c * universe_len * universe_len;
        for (args, &result) in &op.op {
            if args.len() != 2 {
                continue;
            }
            let i0 = val_to_idx.get(&args[0]).copied().unwrap_or(-1);
            let i1 = val_to_idx.get(&args[1]).copied().unwrap_or(-1);
            let r = val_to_idx.get(&result).copied().unwrap_or(-1);
            if i0 >= 0 && i1 >= 0 && r >= 0 {
                op_tables[base + (i0 as usize) * universe_len + (i1 as usize)] = r;
            }
        }
    }

    let count_len = (n_candidates as usize) * (max_history as usize + 1) * 2;

    let ptx = compile_ptx(KERNEL_SRC).ok()?;
    dev.load_ptx(ptx, "ig", &["ig_counts"]).ok()?;
    let f = dev.get_func("ig", "ig_counts").expect("ig_counts loaded");

    let d_histories = dev.htod_sync_copy(&histories).ok()?;
    let d_history_len = dev.htod_sync_copy(&history_len).ok()?;
    let d_index_map = dev.htod_sync_copy(&index_map).ok()?;
    let d_in_target = dev.htod_sync_copy(&in_target).ok()?;
    let d_op_tables = dev.htod_sync_copy(&op_tables).ok()?;
    let d_ti = dev.htod_sync_copy(&ti).ok()?;
    let d_count = dev.alloc_zeros::<i32>(count_len).ok()?;

    let cfg = LaunchConfig {
        grid_dim: (n_candidates as u32, n_tuples as u32, 1),
        block_dim: (1, 1, 1),
        shared_mem_bytes: 0,
    };
    unsafe {
        f.launch(
            cfg,
            (
                &d_histories,
                &d_history_len,
                &d_index_map,
                &d_in_target,
                &d_op_tables,
                &d_ti,
                n_tuples,
                max_h,
                u_len,
                n_candidates,
                &d_count,
            ),
        )
    }
    .ok()?;

    let count_flat = dev.dtoh_sync_copy(&d_count).ok()?;
    dev.synchronize().ok()?;

    let mut best_ig = -1.0f64;
    let mut best_idx = 0usize;
    let stride = (max_history as usize + 1) * 2;
    for c in 0..(n_candidates as usize) {
        let mut part: HashMap<(usize, bool), (usize, usize)> = HashMap::new();
        for g in 0..=(max_history as usize) {
            let in_c = count_flat[c * stride + g * 2] as usize;
            let out_c = count_flat[c * stride + g * 2 + 1] as usize;
            if in_c > 0 {
                part.insert((g, true), (in_c, 0));
            }
            if out_c > 0 {
                part.insert((g, false), (0, out_c));
            }
        }
        let ig = information_gain_from_counts(n, in_total, &part);
        if ig > best_ig {
            best_ig = ig;
            best_idx = c;
        }
    }

    let (op, ti) = cand_list[best_idx].clone();
    Some(((op, ti), best_ig))
}

#[cfg(not(feature = "cuda"))]
pub fn best_candidate_ig_cuda(
    _tuples: &[TupleHistory],
    _cand_list: &[(Operation, Vec<usize>)],
    _n: usize,
    _in_total: usize,
) -> Option<((Operation, Vec<usize>), f64)> {
    None
}
