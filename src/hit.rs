use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use rayon::prelude::*;

use crate::first_order::formulas::{self, Formula, OpSym, Term, Variable};
use crate::first_order::models::Model;
use crate::first_order::relops::{Operation, Relation};
use crate::preprocessing::Pattern;

/// Shared operation table for a HIT run (cloned as Arc across blocks).
type OpsMap = BTreeMap<usize, Vec<Operation>>;

/// Orden de exploración del árbol de bloques.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExploreOrder {
    /// DFS paralelo (Rayon), comportamiento histórico.
    Dfs,
    /// BFS iterativo con cola.
    Bfs,
}

/// Configuración del algoritmo HIT / splitting.
#[derive(Clone, Copy, Debug)]
pub struct HitConfig {
    /// Si true, en cada paso se elige (op, ti) que maximiza information gain.
    pub use_information_gain: bool,
    /// Si use_information_gain, número de candidatos a muestrear (None = todos).
    pub ig_sample: Option<usize>,
    /// Si true, aplica de verdad el mejor candidato IG (experimental; puede cambiar veredictos).
    pub ig_experimental: bool,
    /// No aplicar candidatos que no parten el bloque ni agregan valores nuevos.
    pub skip_useless_candidates: bool,
    /// Precheck: si T no es unión de clases ≈ aproximadas (subálgebra etiquetada), NOT DEFINABLE.
    pub approx_precheck: bool,
    /// Emitir todas las constantes 0-arias al inicio (ya es el comportamiento del generador; flag documentado).
    pub emit_all_constants: bool,
    /// Tope de pasos de splitting (None = sin tope). Si se excede, se trata como no definible.
    pub max_steps: Option<u64>,
    /// Simplificar la fórmula resultado (True/False redundantes).
    pub simplify_formula: bool,
    /// DFS (default) o BFS.
    pub explore_order: ExploreOrder,
    /// Max children explored with Rayon in DFS. Default 1 = fully serial (RAM-safe).
    /// Parallelize only when `1 < children.len() <= max_rayon_children`.
    pub max_rayon_children: usize,
    /// If false, skip expensive QF witness synthesis (bench/scale): decision only.
    pub synthesize_formula: bool,
    /// Soft RSS cap in KiB (`None` = no check). On Linux, exceeding it aborts as
    /// not definable (same as `max_steps`) to avoid node OOM kills.
    pub max_rss_kib: Option<u64>,
    /// On impure blocks, run compact IsoType (TMH) immediately instead of waiting
    /// for the HIT generator to finish. Sound for the decision: TMH uses ambient
    /// IsoType of the original tuple `t`, independent of generator state. Avoids
    /// magma `n32_k2` TIMEOUT where skip_useless never reaches finished-impure.
    pub eager_isotype_on_impure: bool,
}

impl Default for HitConfig {
    fn default() -> Self {
        Self {
            use_information_gain: false,
            ig_sample: Some(20),
            ig_experimental: false,
            skip_useless_candidates: true,
            approx_precheck: false,
            emit_all_constants: true,
            max_steps: None,
            simplify_formula: false,
            explore_order: ExploreOrder::Dfs,
            max_rayon_children: 1,
            synthesize_formula: true,
            max_rss_kib: None,
            eager_isotype_on_impure: true,
        }
    }
}

fn current_rss_kib() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("VmRSS:") {
                return rest.split_whitespace().next()?.parse().ok();
            }
        }
        None
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Contadores globales de una corrida (atómicos para Rayon).
pub struct HitRunStats {
    pub steps: std::sync::atomic::AtomicU64,
    pub candidates_considered: std::sync::atomic::AtomicU64,
    pub candidates_skipped: std::sync::atomic::AtomicU64,
    pub approx_reject: std::sync::atomic::AtomicU64,
}

impl HitRunStats {
    pub fn new() -> Self {
        Self {
            steps: std::sync::atomic::AtomicU64::new(0),
            candidates_considered: std::sync::atomic::AtomicU64::new(0),
            candidates_skipped: std::sync::atomic::AtomicU64::new(0),
            approx_reject: std::sync::atomic::AtomicU64::new(0),
        }
    }
    pub fn snapshot(&self) -> (u64, u64, u64, u64) {
        use std::sync::atomic::Ordering::Relaxed;
        (
            self.steps.load(Relaxed),
            self.candidates_considered.load(Relaxed),
            self.candidates_skipped.load(Relaxed),
            self.approx_reject.load(Relaxed),
        )
    }
}

static RUN_STATS: HitRunStats = HitRunStats {
    steps: std::sync::atomic::AtomicU64::new(0),
    candidates_considered: std::sync::atomic::AtomicU64::new(0),
    candidates_skipped: std::sync::atomic::AtomicU64::new(0),
    approx_reject: std::sync::atomic::AtomicU64::new(0),
};

pub fn reset_run_stats() {
    use std::sync::atomic::Ordering::Relaxed;
    RUN_STATS.steps.store(0, Relaxed);
    RUN_STATS.candidates_considered.store(0, Relaxed);
    RUN_STATS.candidates_skipped.store(0, Relaxed);
    RUN_STATS.approx_reject.store(0, Relaxed);
}

pub fn run_stats_snapshot() -> (u64, u64, u64, u64) {
    RUN_STATS.snapshot()
}

fn entropy(in_count: i64, out_count: i64) -> f64 {
    let n = in_count + out_count;
    if n == 0 {
        return 0.0;
    }
    let p = in_count as f64 / n as f64;
    let q = out_count as f64 / n as f64;
    let h = |x: f64| if x > 0.0 { -x * x.log2() } else { 0.0 };
    h(p) + h(q)
}

pub(crate) fn information_gain_from_counts(
    n: usize,
    in_total: usize,
    partition_counts: &HashMap<(usize, bool), (usize, usize)>,
) -> f64 {
    if n == 0 {
        return 0.0;
    }
    let out_total = n - in_total;
    let h_before = entropy(in_total as i64, out_total as i64);
    let mut h_after = 0.0;
    for (_, (g_in, g_out)) in partition_counts {
        let gs = g_in + g_out;
        h_after += (gs as f64 / n as f64) * entropy(*g_in as i64, *g_out as i64);
    }
    h_before - h_after
}

#[derive(Clone, Debug)]
pub struct Counterexample(pub Vec<Vec<i64>>);

fn cartesian_product_indices(pool: &[usize], n: usize, forced: &HashSet<usize>) -> Vec<Vec<usize>> {
    if n == 0 {
        return vec![vec![]];
    }
    let mut result = Vec::new();
    let mut stack = vec![vec![]];
    while let Some(current) = stack.pop() {
        if current.len() == n {
            if current.iter().any(|i| forced.contains(i)) {
                result.push(current);
            }
        } else {
            for &p in pool {
                let mut next = current.clone();
                next.push(p);
                stack.push(next);
            }
        }
    }
    result
}

#[derive(Clone, Debug)]
pub struct TupleHistory {
    pub t: Vec<i64>,
    pub history: Vec<i64>,
    index_map: HashMap<i64, usize>,
    pub in_target: bool,
    pub has_generated: bool,
}

impl TupleHistory {
    pub fn new(t: Vec<i64>, targets: &[&Relation]) -> Self {
        let history = t.clone();
        let mut index_map = HashMap::new();
        for (i, &x) in t.iter().enumerate() {
            index_map.insert(x, i);
        }
        let in_target = targets.iter().all(|tg| {
            if tg.arity == t.len() {
                tg.contains(&t)
            } else {
                true
            }
        });
        Self {
            t,
            history,
            index_map,
            in_target,
            has_generated: false,
        }
    }

    pub fn step(&mut self, op: &Operation, ti: &[usize]) -> usize {
        let args: Vec<i64> = ti.iter().map(|&i| self.history[i]).collect();
        let x = op.call(&args).unwrap();
        if let Some(&xi) = self.index_map.get(&x) {
            self.has_generated = false;
            return xi;
        }
        let idx = self.history.len();
        self.history.push(x);
        self.index_map.insert(x, idx);
        self.has_generated = true;
        idx
    }

    pub fn simulate_step(&self, op: &Operation, ti: &[usize]) -> (usize, bool) {
        let args: Vec<i64> = ti.iter().map(|&i| self.history[i]).collect();
        let x = op.call(&args).unwrap();
        if let Some(&xi) = self.index_map.get(&x) {
            return (xi, false);
        }
        (self.history.len(), true)
    }
}

impl std::hash::Hash for TupleHistory {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for x in &self.t {
            x.hash(state);
        }
        for x in &self.history {
            x.hash(state);
        }
    }
}
impl PartialEq for TupleHistory {
    fn eq(&self, other: &Self) -> bool {
        self.t == other.t && self.history == other.history
    }
}
impl Eq for TupleHistory {}

#[derive(Clone)]
struct IndicesTupleGenerator {
    ops: Arc<OpsMap>,
    arity: usize,
    viejos: Vec<usize>,
    nuevos: Vec<usize>,
    sintactico: Vec<Term>,
    last_term: Option<Term>,
    forked: bool,
    pub finished: bool,
    state: GenState,
}

#[derive(Clone)]
enum GenState {
    Init,
    Iter {
        arities: Vec<usize>,
        arity_idx: usize,
        op_idx: usize,
        perm_idx: usize,
        perms: Vec<Vec<usize>>,
    },
}

impl IndicesTupleGenerator {
    fn new(
        ops: Arc<OpsMap>,
        arity: usize,
        viejos: Vec<usize>,
        nuevos: Vec<usize>,
    ) -> Self {
        let sintactico: Vec<Term> = (0..=arity + 10) // allow growth
            .map(|i| Term::Variable(Variable::from_index(i as i32)))
            .collect();
        Self {
            ops,
            arity,
            viejos,
            nuevos,
            sintactico,
            last_term: None,
            forked: false,
            finished: false,
            state: GenState::Init,
        }
    }

    fn step(&mut self) -> Option<(Operation, Vec<usize>)> {
        if self.forked {
            panic!("Generator was forked!");
        }
        loop {
            match &mut self.state {
                GenState::Init => {
                    if let Some(constants) = self.ops.get(&0) {
                        if !constants.is_empty() {
                            let op = constants[0].clone();
                            self.last_term = Some(Term::OpTerm {
                                sym: OpSym::new(&op.sym, 0),
                                args: vec![],
                            });
                            self.state = GenState::Iter {
                                arities: vec![0],
                                arity_idx: 0,
                                op_idx: 1,
                                perm_idx: 0,
                                perms: vec![vec![]],
                            };
                            return Some((op, vec![]));
                        }
                    }
                    let arities: Vec<usize> = self.ops.keys().filter(|&&a| a > 0).cloned().collect();
                    if arities.is_empty() && !self.ops.contains_key(&0) {
                        self.finished = true;
                        return None;
                    }
                    self.state = GenState::Iter {
                        arities: arities.clone(),
                        arity_idx: 0,
                        op_idx: 0,
                        perm_idx: 0,
                        perms: vec![],
                    };
                    if !self.nuevos.is_empty() {
                        let pool: Vec<usize> = self.viejos.iter().chain(&self.nuevos).cloned().collect();
                        let forced: HashSet<usize> = self.nuevos.iter().cloned().collect();
                        if let Some(&a) = arities.first() {
                            let perms = cartesian_product_indices(&pool, a, &forced);
                            self.state = GenState::Iter {
                                arities: arities.clone(),
                                arity_idx: 0,
                                op_idx: 0,
                                perm_idx: 0,
                                perms,
                            };
                        }
                    }
                }
                GenState::Iter {
                    arities,
                    arity_idx,
                    op_idx,
                    perm_idx,
                    perms,
                } => {
                    if arities.is_empty() {
                        if self.nuevos.is_empty() {
                            self.finished = true;
                            return None;
                        }
                        // Keep `nuevos` so Init rebuilds the next wave with them forced.
                        self.state = GenState::Init;
                        continue;
                    }
                    if *arity_idx >= arities.len() {
                        if self.nuevos.is_empty() {
                            self.finished = true;
                            return None;
                        }
                        // Promote this wave's nuevos into viejos but force the next
                        // wave on those indices (not on an empty set after drain).
                        let forced: HashSet<usize> = self.nuevos.iter().copied().collect();
                        self.viejos.extend(self.nuevos.drain(..));
                        *arity_idx = 0;
                        *op_idx = 0;
                        *perm_idx = 0;
                        let pool = self.viejos.clone();
                        *perms = cartesian_product_indices(&pool, arities[0], &forced);
                        continue;
                    }
                    let a = arities[*arity_idx];
                    let op_list = self.ops.get(&a)?;
                    if *op_idx >= op_list.len() {
                        *op_idx = 0;
                        *perm_idx = 0;
                        *arity_idx += 1;
                        if *arity_idx >= arities.len() {
                            if self.nuevos.is_empty() {
                                self.finished = true;
                                return None;
                            }
                            let forced: HashSet<usize> = self.nuevos.iter().copied().collect();
                            self.viejos.extend(self.nuevos.drain(..));
                            *arity_idx = 0;
                            *op_idx = 0;
                            *perm_idx = 0;
                            let pool = self.viejos.clone();
                            *perms = cartesian_product_indices(&pool, arities[0], &forced);
                            continue;
                        }
                        let pool: Vec<usize> =
                            self.viejos.iter().chain(&self.nuevos).cloned().collect();
                        let forced: HashSet<usize> = self.nuevos.iter().cloned().collect();
                        *perms = cartesian_product_indices(&pool, arities[*arity_idx], &forced);
                        continue;
                    }
                    if *perm_idx >= perms.len() {
                        *op_idx += 1;
                        *perm_idx = 0;
                        continue;
                    }
                    let op = op_list[*op_idx].clone();
                    let ti = perms[*perm_idx].clone();
                    *perm_idx += 1;
                    let args: Vec<Term> = ti.iter().map(|&i| self.sintactico[i].clone()).collect();
                    self.last_term = Some(Term::OpTerm {
                        sym: OpSym::new(&op.sym, op.arity),
                        args,
                    });
                    return Some((op, ti));
                }
            }
        }
    }

    #[allow(dead_code)]
    fn set_last_term(&mut self, op: &Operation, ti: &[usize]) {
        let args: Vec<Term> = ti.iter().map(|&i| self.sintactico[i].clone()).collect();
        self.last_term = Some(Term::OpTerm {
            sym: OpSym::new(&op.sym, op.arity),
            args,
        });
    }

    /// Avanza el generador hasta haber “consumido” el candidato (op, ti), para que
    /// la siguiente llamada a step() o take_candidates() devuelva el siguiente.
    /// Necesario cuando el candidato se eligió por IG en lugar de por step().
    #[allow(dead_code)]
    fn advance_until(&mut self, op: &Operation, ti: &[usize]) {
        loop {
            match self.step() {
                None => return,
                Some((o, t)) => {
                    if o.sym == op.sym && o.arity == op.arity && t == ti {
                        return;
                    }
                }
            }
        }
    }

    fn formula_diferenciadora(&mut self, index: usize) -> Formula {
        let term = self.last_term.as_ref().unwrap().clone();
        formulas::eq(term, self.sintactico[index].clone())
    }

    fn hubo_nuevo(&mut self) {
        if self.forked {
            panic!("Generator was forked!");
        }
        let new_idx = self.viejos.len() + self.nuevos.len();
        self.nuevos.push(new_idx);
        while self.sintactico.len() <= new_idx {
            self.sintactico
                .push(Term::Variable(Variable::from_index(self.sintactico.len() as i32)));
        }
        self.sintactico[new_idx] = self.last_term.as_ref().unwrap().clone();
    }

    fn fork(&mut self, quantity: usize) -> Vec<IndicesTupleGenerator> {
        if self.forked {
            panic!("Generator was forked!");
        }
        self.forked = true;
        (0..quantity)
            .map(|_| IndicesTupleGenerator {
                ops: self.ops.clone(),
                arity: self.arity,
                viejos: self.viejos.clone(),
                nuevos: self.nuevos.clone(),
                sintactico: self.sintactico.clone(),
                last_term: self.last_term.clone(),
                forked: false,
                finished: self.finished,
                state: GenState::Init,
            })
            .collect()
    }

    /// Candidatos en el mismo orden que step(), para que advance_until quede bien.
    fn enumerate_candidates(&self) -> Vec<(Operation, Vec<usize>)> {
        let mut gen = self.clone();
        std::iter::from_fn(move || gen.step()).collect()
    }

    /// Primeros `limit` candidatos en el mismo orden que step().
    fn take_candidates(&self, limit: usize) -> Vec<(Operation, Vec<usize>)> {
        let mut gen = self.clone();
        (0..limit).filter_map(|_| gen.step()).collect()
    }
}

#[derive(Clone)]
pub struct Block {
    /// Ambient algebra for IsoType/TMH (targets `T*` stripped from relations).
    ambient: Arc<Model>,
    operations: Arc<OpsMap>,
    tuples: Vec<TupleHistory>,
    targets: Vec<Relation>,
    pub formula: Formula,
    arity: usize,
    generator: IndicesTupleGenerator,
    config: HitConfig,
    /// Shared TMH IsoType cache for this `is_open_def` run.
    tmh_cache: Arc<crate::engines::tuple_model_hash::TmhCache>,
}

impl Block {
    pub fn new(
        operations: OpsMap,
        ambient: Arc<Model>,
        tuples: Vec<TupleHistory>,
        targets: Vec<Relation>,
        formula: Formula,
        config: HitConfig,
    ) -> Self {
        Self::new_shared(
            Arc::new(operations),
            ambient,
            tuples,
            targets,
            formula,
            config,
            Arc::new(crate::engines::tuple_model_hash::TmhCache::new()),
        )
    }

    fn new_shared(
        operations: Arc<OpsMap>,
        ambient: Arc<Model>,
        tuples: Vec<TupleHistory>,
        targets: Vec<Relation>,
        formula: Formula,
        config: HitConfig,
        tmh_cache: Arc<crate::engines::tuple_model_hash::TmhCache>,
    ) -> Self {
        let arity = targets[0].arity;
        let viejos = vec![];
        let nuevos: Vec<usize> = (0..arity).collect();
        let generator = IndicesTupleGenerator::new(Arc::clone(&operations), arity, viejos, nuevos);
        Self {
            ambient,
            operations,
            tuples,
            targets,
            formula,
            arity,
            generator,
            config,
            tmh_cache,
        }
    }

    pub fn is_all_in_targets(&self) -> bool {
        self.tuples.iter().all(|th| th.in_target)
    }

    pub fn is_disjunt_to_targets(&self) -> bool {
        self.tuples.iter().all(|th| !th.in_target)
    }

    pub fn finished(&self) -> bool {
        self.generator.finished
    }

    pub fn step(&mut self) -> Result<Vec<Block>, Counterexample> {
        let steps = RUN_STATS.steps.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        if let Some(max) = self.config.max_steps {
            if steps > max {
                return Err(Counterexample(
                    self.tuples.iter().map(|th| th.t.clone()).collect(),
                ));
            }
        }
        if crate::engines::tuple_model_hash::tmh_stats_enabled()
            && (steps <= 8 || steps % 64 == 0)
        {
            let (c, h, pr, pf, fi) = crate::engines::tuple_model_hash::tmh_stats_snapshot();
            eprintln!(
                "HIT_PROGRESS steps={steps} computes={c} cache_hits={h} proxy_rejects={pr} proxy_full={pf} finished_impure={fi} block_tuples={}",
                self.tuples.len()
            );
        }
        if let Some(max_kib) = self.config.max_rss_kib {
            // Sample every 64 steps to keep /proc reads cheap.
            if steps % 64 == 0 {
                if let Some(rss) = current_rss_kib() {
                    if rss > max_kib {
                        return Err(Counterexample(
                            self.tuples.iter().map(|th| th.t.clone()).collect(),
                        ));
                    }
                }
            }
        }
        let (op, ti) = if self.config.use_information_gain {
            // Con un solo candidato usamos step() para mantener el mismo orden que sin IG y no colgar.
            if self.config.ig_sample == Some(1) {
                match self.generator.step() {
                    Some(x) => x,
                    None => {
                        self.generator.finished = true;
                        return Ok(vec![self.clone()]);
                    }
                }
            } else {
            let cand_list = match self.config.ig_sample {
                Some(n) => self.generator.take_candidates(n),
                None => self.generator.enumerate_candidates(),
            };
            let mut cand_list = cand_list;
            if self.config.skip_useless_candidates {
                let before = cand_list.len();
                cand_list.retain(|(op, ti)| {
                    let mut splits = false;
                    let mut adds = false;
                    let mut first_idx: Option<usize> = None;
                    for th in &self.tuples {
                        let (idx, is_new) = th.simulate_step(op, ti);
                        if is_new {
                            adds = true;
                        }
                        match first_idx {
                            None => first_idx = Some(idx),
                            Some(f) if f != idx => splits = true,
                            _ => {}
                        }
                    }
                    splits || adds
                });
                RUN_STATS.candidates_skipped.fetch_add(
                    (before - cand_list.len()) as u64,
                    std::sync::atomic::Ordering::Relaxed,
                );
            }
            RUN_STATS.candidates_considered.fetch_add(
                cand_list.len() as u64,
                std::sync::atomic::Ordering::Relaxed,
            );
            // Si el sample quedó vacío tras skip, no marcar finished: el generador
            // puede tener más candidatos (legacy -i sigue step(); experimental busca útil).
            if cand_list.is_empty() {
                if self.config.ig_experimental && self.config.skip_useless_candidates {
                    loop {
                        match self.generator.step() {
                            None => {
                                self.generator.finished = true;
                                return Ok(vec![self.clone()]);
                            }
                            Some((op, ti)) => {
                                RUN_STATS
                                    .candidates_considered
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                let mut splits = false;
                                let mut adds = false;
                                let mut first_idx: Option<usize> = None;
                                for th in &self.tuples {
                                    let (idx, is_new) = th.simulate_step(&op, &ti);
                                    if is_new {
                                        adds = true;
                                    }
                                    match first_idx {
                                        None => first_idx = Some(idx),
                                        Some(f) if f != idx => splits = true,
                                        _ => {}
                                    }
                                }
                                if splits || adds {
                                    break (op, ti);
                                }
                                RUN_STATS
                                    .candidates_skipped
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            }
                        }
                    }
                } else {
                    match self.generator.step() {
                        Some(x) => x,
                        None => {
                            self.generator.finished = true;
                            return Ok(vec![self.clone()]);
                        }
                    }
                }
            } else {
            let tuples_clone = self.tuples.clone();
            let n = tuples_clone.len();
            let in_total = tuples_clone.iter().filter(|th| th.in_target).count();

            #[allow(unused_assignments)]
            let mut best: Option<(f64, usize, Operation, Vec<usize>)> = None;
            #[cfg(feature = "cuda")]
            {
                if let Some(((op_c, ti_c), _ig)) =
                    crate::hit_cuda::best_candidate_ig_cuda(&tuples_clone, &cand_list, n, in_total)
                {
                    let idx = cand_list
                        .iter()
                        .position(|(o, t)| o.sym == op_c.sym && o.arity == op_c.arity && t == ti_c)
                        .unwrap_or(0);
                    best = Some((0.0f64, idx, op_c, ti_c));
                }
            }
            if best.is_none() {
                best = cand_list
                .par_iter()
                .enumerate()
                .map(|(idx, (op, ti))| {
                    let part: HashMap<(usize, bool), (usize, usize)> = tuples_clone
                        .par_iter()
                        .map(|th| {
                            let (idx, _) = th.simulate_step(op, ti);
                            let key = (idx, th.in_target);
                            let val = if th.in_target { (1, 0) } else { (0, 1) };
                            let mut m = HashMap::new();
                            m.insert(key, val);
                            m
                        })
                        .reduce(
                            || HashMap::new(),
                            |mut a, b| {
                                for (k, v) in b {
                                    let e = a.entry(k).or_insert((0, 0));
                                    e.0 += v.0;
                                    e.1 += v.1;
                                }
                                a
                            },
                        );
                    let ig = information_gain_from_counts(n, in_total, &part);
                    (ig, idx, op.clone(), ti.clone())
                })
                .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            }
            // Por defecto (-i) se calcula IG pero se sigue el orden step() (legacy).
            // Con ig_experimental se aplica el mejor candidato + advance_until.
            match best {
                Some((_ig, _best_idx, op, ti)) if self.config.ig_experimental => {
                    self.generator.advance_until(&op, &ti);
                    (op, ti)
                }
                Some(_) => {
                    let (op, ti) = self.generator.step().expect("al menos un candidato");
                    (op, ti)
                }
                None => {
                    self.generator.finished = true;
                    return Ok(vec![self.clone()]);
                }
            }
            }
            }
        } else if self.config.skip_useless_candidates {
            loop {
                match self.generator.step() {
                    None => {
                        self.generator.finished = true;
                        return Ok(vec![self.clone()]);
                    }
                    Some((op, ti)) => {
                        RUN_STATS
                            .candidates_considered
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let mut splits = false;
                        let mut adds = false;
                        let mut first_idx: Option<usize> = None;
                        for th in &self.tuples {
                            let (idx, is_new) = th.simulate_step(&op, &ti);
                            if is_new {
                                adds = true;
                            }
                            match first_idx {
                                None => first_idx = Some(idx),
                                Some(f) if f != idx => splits = true,
                                _ => {}
                            }
                        }
                        if splits || adds {
                            break (op, ti);
                        }
                        RUN_STATS
                            .candidates_skipped
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }
        } else {
            match self.generator.step() {
                Some(x) => x,
                None => return Ok(vec![self.clone()]),
            }
        };

        let mut result: HashMap<usize, Vec<TupleHistory>> = HashMap::new();
        let mut any_has_gen = false;
        for mut th in std::mem::take(&mut self.tuples) {
            let idx = th.step(&op, &ti);
            any_has_gen = any_has_gen || th.has_generated;
            result.entry(idx).or_default().push(th);
        }

        if result.len() == 1 {
            if any_has_gen {
                self.generator.hubo_nuevo();
            }
            self.tuples = result.into_values().next().unwrap();
            return Ok(vec![self.clone()]);
        }

        let num_groups = result.len();
        let mut generators = self.generator.fork(num_groups);
        let mut blocks = Vec::new();
        let mut fneg = formulas::true_formula(None);
        let mut negados = Vec::new();
        let mut i = 0;
        let synth = self.config.synthesize_formula;
        for (index, tuples_new) in result {
            if tuples_new.iter().any(|th| th.has_generated) {
                generators[i].hubo_nuevo();
                negados.push((i, index, tuples_new));
            } else {
                let f = if synth {
                    let fd = generators[i].formula_diferenciadora(index);
                    fneg = fneg.and_formula(&fd.neg());
                    self.formula.clone().and_formula(&fd)
                } else {
                    formulas::true_formula(None)
                };
                blocks.push(Block {
                    ambient: Arc::clone(&self.ambient),
                    operations: self.operations.clone(),
                    tuples: tuples_new,
                    targets: self.targets.clone(),
                    formula: f,
                    arity: self.arity,
                    generator: generators[i].clone(),
                    config: self.config,
                    tmh_cache: Arc::clone(&self.tmh_cache),
                });
            }
            i += 1;
        }
        for (i, _index, tuples_new) in negados {
            let f = if synth {
                self.formula.clone().and_formula(&fneg)
            } else {
                formulas::true_formula(None)
            };
            blocks.push(Block {
                ambient: Arc::clone(&self.ambient),
                operations: self.operations.clone(),
                tuples: tuples_new,
                targets: self.targets.clone(),
                formula: f,
                arity: self.arity,
                generator: generators[i].clone(),
                config: self.config,
                tmh_cache: Arc::clone(&self.tmh_cache),
            });
        }
        Ok(blocks)
    }
}

fn cartesian_product_sized<T: Clone>(items: &[T], n: usize) -> Vec<Vec<T>> {
    if n == 0 {
        return vec![vec![]];
    }
    let mut result = vec![vec![]];
    for _ in 0..n {
        let mut next = Vec::new();
        for r in &result {
            for item in items {
                let mut row = r.clone();
                row.push(item.clone());
                next.push(row);
            }
        }
        result = next;
    }
    result
}

pub fn is_open_def(
    model: &Model,
    targets: Vec<Relation>,
    config: HitConfig,
) -> Result<Formula, Counterexample> {
    reset_run_stats();
    crate::engines::tuple_model_hash::reset_tmh_stats();
    crate::engines::tuple_model_hash::ensure_tmh_stats_atexit();
    if config.approx_precheck {
        for target in &targets {
            if crate::approx::target_breaks_approx_classes(model, target) {
                RUN_STATS
                    .approx_reject
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Err(Counterexample(
                    target.r.iter().take(1).cloned().collect(),
                ));
            }
        }
    }
    let arity = targets[0].arity;
    let universe = &model.universe;
    let targets_ref: Vec<&Relation> = targets.iter().collect();
    // After equality-pattern preprocessing, each piece only speaks about tuples of
    // that pattern. Enumerating all of A^k injects foreign-pattern rows (e.g.
    // diagonals into an off-diagonal piece), so the initial block is never full
    // even when the original target was R=A^k — killing the info-gap early accept.
    let pattern = targets[0].pattern.clone();
    let mut tuples: Vec<TupleHistory> = cartesian_product_sized(universe, arity)
        .into_iter()
        .filter(|t| match pattern.as_ref() {
            Some(p) => Pattern::new(t.clone()) == **p,
            None => true,
        })
        .map(|t| TupleHistory::new(t, &targets_ref))
        .collect();
    tuples.sort_by(|a, b| a.t.cmp(&b.t));
    let mut operations: BTreeMap<usize, Vec<Operation>> = BTreeMap::new();
    for op in model.operations.values() {
        operations.entry(op.arity).or_default().push(op.clone());
    }
    for op_list in operations.values_mut() {
        op_list.sort_by(|a, b| a.sym.cmp(&b.sym));
    }
    let formula = match targets[0].pattern.as_ref() {
        Some(p) => p.preprocessed_formula(),
        // Empty / unpatterned targets (e.g. random_half drawing zero rows).
        None => formulas::true_formula(None),
    };
    // Strip T* so TMH/IsoType matches merge ambient (historical main.py).
    let mut ambient_model = model.clone();
    ambient_model
        .relations
        .retain(|sym, _| !sym.starts_with('T'));
    let ambient = Arc::new(ambient_model);
    let start_block = Block::new(operations, ambient, tuples, targets, formula, config);
    let result = match config.explore_order {
        ExploreOrder::Dfs => is_open_def_parallel_recursive(start_block),
        ExploreOrder::Bfs => is_open_def_bfs(start_block),
    };
    crate::engines::tuple_model_hash::dump_tmh_stats_if_enabled();
    match result {
        Ok(f) if config.simplify_formula => Ok(f.simplify_ast()),
        other => other,
    }
}

fn operations_by_name(operations: &OpsMap) -> HashMap<String, Operation> {
    let mut map = HashMap::new();
    for list in operations.values() {
        for op in list {
            map.insert(op.sym.clone(), op.clone());
        }
    }
    map
}

fn generate_terms(
    operations: &OpsMap,
    arity: usize,
    max_depth: usize,
) -> Vec<Term> {
    const CAP: usize = 64;
    let mut terms: Vec<Term> = (0..arity)
        .map(|i| Term::Variable(Variable::from_index(i as i32)))
        .collect();
    for _ in 0..max_depth {
        if terms.len() >= CAP {
            break;
        }
        let snapshot = terms.clone();
        for op_list in operations.values() {
            for op in op_list {
                if op.arity == 0 {
                    let t = Term::OpTerm {
                        sym: OpSym::new(&op.sym, 0),
                        args: vec![],
                    };
                    if !terms.iter().any(|x| x == &t) {
                        terms.push(t);
                    }
                } else if op.arity == 1 {
                    for a in &snapshot {
                        let t = Term::OpTerm {
                            sym: OpSym::new(&op.sym, 1),
                            args: vec![a.clone()],
                        };
                        if !terms.iter().any(|x| x == &t) {
                            terms.push(t);
                            if terms.len() >= CAP {
                                return terms;
                            }
                        }
                    }
                } else if op.arity == 2 {
                    for a in &snapshot {
                        for b in &snapshot {
                            let t = Term::OpTerm {
                                sym: OpSym::new(&op.sym, 2),
                                args: vec![a.clone(), b.clone()],
                            };
                            if !terms.iter().any(|x| x == &t) {
                                terms.push(t);
                                if terms.len() >= CAP {
                                    return terms;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    terms
}

fn eval_term_on_tuple(
    term: &Term,
    ops: &HashMap<String, Operation>,
    tup: &[i64],
) -> Option<i64> {
    let mut env = HashMap::new();
    for (i, &x) in tup.iter().enumerate() {
        env.insert(Variable::from_index(i as i32), x);
    }
    term.evaluate(ops, &env).ok()
}

fn atom_holds_on_tuple(
    phi: &Formula,
    ops: &HashMap<String, Operation>,
    tup: &[i64],
) -> bool {
    match phi {
        Formula::True(_) => true,
        Formula::False(_) => false,
        Formula::Eq(t1, t2) => {
            match (
                eval_term_on_tuple(t1, ops, tup),
                eval_term_on_tuple(t2, ops, tup),
            ) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            }
        }
        Formula::Neg(inner) => !atom_holds_on_tuple(inner, ops, tup),
        _ => false,
    }
}

/// When the HIT generator finishes on an impure block, partition by
/// compact IsoType. Same type with mixed in/out target => `None` (NOT).
/// Multiple pure types => one child per class.
///
/// Order: index all in-R first, then scan out-R so a mixed IsoType aborts early.
/// Optional env knobs: `HIT_TMH_CLOSURE_CACHE`, `HIT_TMH_PROXY`, `HIT_TMH_RAYON`.
fn try_split_finished_impure(block: &Block) -> Option<Vec<Block>> {
    use crate::engines::tuple_model_hash::{
        cheap_proxy, classify_compact, note_finished_impure, note_proxy_full, note_proxy_reject,
        tmh_proxy_enabled, tmh_rayon_threads, CompactIsoType,
    };
    note_finished_impure();
    use std::collections::{HashMap, HashSet};

    struct ClassInfo {
        id: usize,
        has_in: bool,
        has_out: bool,
        members: Vec<usize>,
    }

    let mut in_idxs: Vec<usize> = Vec::new();
    let mut out_idxs: Vec<usize> = Vec::new();
    for (i, th) in block.tuples.iter().enumerate() {
        if th.in_target {
            in_idxs.push(i);
        } else {
            out_idxs.push(i);
        }
    }

    let cache = if crate::engines::tuple_model_hash::tmh_closure_cache_enabled() {
        Some(&block.tmh_cache)
    } else {
        None
    };

    let use_proxy = tmh_proxy_enabled();
    let mut in_proxies: HashSet<u64> = HashSet::new();
    let proxies: Vec<Option<u64>> = if use_proxy {
        block
            .tuples
            .iter()
            .map(|th| Some(cheap_proxy(&block.ambient, &th.t)))
            .collect()
    } else {
        vec![None; block.tuples.len()]
    };
    if use_proxy {
        for &idx in &in_idxs {
            in_proxies.insert(proxies[idx].unwrap());
        }
    }

    // Precompute compacts for in-R (optional Rayon).
    let rayon_n = tmh_rayon_threads();
    let in_compacts: Vec<(usize, CompactIsoType)> = if rayon_n > 1 && in_idxs.len() > 1 {
        use rayon::prelude::*;
        in_idxs
            .par_iter()
            .map(|&idx| {
                (
                    idx,
                    classify_compact(&block.ambient, &block.tuples[idx].t, cache),
                )
            })
            .collect()
    } else {
        in_idxs
            .iter()
            .map(|&idx| {
                (
                    idx,
                    classify_compact(&block.ambient, &block.tuples[idx].t, cache),
                )
            })
            .collect()
    };

    let mut classes: HashMap<CompactIsoType, ClassInfo> = HashMap::new();
    let mut n_classes = 0usize;

    for (idx, compact) in in_compacts {
        match classes.get_mut(&compact) {
            Some(info) => {
                if info.has_out {
                    return None;
                }
                info.has_in = true;
                info.members.push(idx);
            }
            None => {
                classes.insert(
                    compact,
                    ClassInfo {
                        id: n_classes,
                        has_in: true,
                        has_out: false,
                        members: vec![idx],
                    },
                );
                n_classes += 1;
            }
        }
    }

    // Out-R: with proxy, check overlapping proxies first (NOT candidates); novel proxies after.
    let (out_overlap, out_novel): (Vec<usize>, Vec<usize>) = if use_proxy {
        let mut o = Vec::new();
        let mut n = Vec::new();
        for &idx in &out_idxs {
            if in_proxies.contains(&proxies[idx].unwrap()) {
                o.push(idx);
            } else {
                n.push(idx);
            }
        }
        (o, n)
    } else {
        (out_idxs.clone(), Vec::new())
    };

    for &idx in out_overlap.iter().chain(out_novel.iter()) {
        if use_proxy {
            if in_proxies.contains(&proxies[idx].unwrap()) {
                note_proxy_full();
            } else {
                note_proxy_reject();
            }
        }
        let compact = classify_compact(&block.ambient, &block.tuples[idx].t, cache);
        match classes.get_mut(&compact) {
            Some(info) => {
                if info.has_in {
                    return None;
                }
                info.has_out = true;
                info.members.push(idx);
            }
            None => {
                classes.insert(
                    compact,
                    ClassInfo {
                        id: n_classes,
                        has_in: false,
                        has_out: true,
                        members: vec![idx],
                    },
                );
                n_classes += 1;
            }
        }
    }

    if n_classes <= 1 {
        return None;
    }

    let mut by_class: Vec<Vec<usize>> = vec![Vec::new(); n_classes];
    for info in classes.into_values() {
        by_class[info.id] = info.members;
    }

    let ops = operations_by_name(&block.operations);
    let phi = if block.config.synthesize_formula {
        find_shallow_separator(
            &ops,
            block.arity,
            &block.tuples[by_class[0][0]].t,
            &block.tuples[by_class[1][0]].t,
        )
    } else {
        None
    };

    let mut children = Vec::with_capacity(n_classes);
    for members in by_class {
        let tuples: Vec<TupleHistory> = members
            .into_iter()
            .map(|i| block.tuples[i].clone())
            .collect();
        let formula = if block.config.synthesize_formula {
            match &phi {
                Some(atom) => {
                    let holds = atom_holds_on_tuple(atom, &ops, &tuples[0].t);
                    if holds {
                        block.formula.clone().and_formula(atom)
                    } else {
                        block.formula.clone().and_formula(&atom.neg())
                    }
                }
                None => block.formula.clone(),
            }
        } else if tuples[0].in_target {
            formulas::true_formula(None)
        } else {
            formulas::false_formula(None)
        };
        children.push(Block::new_shared(
            Arc::clone(&block.operations),
            Arc::clone(&block.ambient),
            tuples,
            block.targets.clone(),
            formula,
            block.config,
            Arc::clone(&block.tmh_cache),
        ));
    }
    Some(children)
}

/// Prefer shallow term equalities that separate `tin` from `tout` (depth ≤ 2).
fn find_shallow_separator(
    ops: &HashMap<String, Operation>,
    arity: usize,
    tin: &[i64],
    tout: &[i64],
) -> Option<Formula> {
    // Rebuild ops map by arity for generate_terms.
    let mut by_ar: OpsMap = BTreeMap::new();
    for op in ops.values() {
        by_ar.entry(op.arity).or_default().push(op.clone());
    }
    for depth in 1..=2 {
        let terms = generate_terms(&by_ar, arity, depth);
        let mut indexed: Vec<(usize, &Term)> = terms.iter().enumerate().collect();
        indexed.sort_by_key(|(_, t)| t.grade());
        for (ii, (_, ti)) in indexed.iter().enumerate() {
            for (_, tj) in indexed.iter().take(ii) {
                let Some(va1) = eval_term_on_tuple(ti, ops, tin) else {
                    continue;
                };
                let Some(va2) = eval_term_on_tuple(tj, ops, tin) else {
                    continue;
                };
                let Some(vb1) = eval_term_on_tuple(ti, ops, tout) else {
                    continue;
                };
                let Some(vb2) = eval_term_on_tuple(tj, ops, tout) else {
                    continue;
                };
                if (va1 == va2) != (vb1 == vb2) {
                    return Some(formulas::eq((*ti).clone(), (*tj).clone()));
                }
            }
        }
    }
    None
}

fn handle_finished_impure(block: &Block) -> Result<Vec<Block>, Counterexample> {
    if let Some(children) = try_split_finished_impure(block) {
        return Ok(children);
    }
    Err(Counterexample(
        block.tuples.iter().map(|th| th.t.clone()).collect(),
    ))
}

/// Impure block should run TMH/IsoType now (generator finished or eager policy).
fn should_run_isotype_split(block: &Block) -> bool {
    if block.is_all_in_targets() || block.is_disjunt_to_targets() {
        return false;
    }
    block.finished() || block.config.eager_isotype_on_impure
}

/// Exploración BFS: cola de bloques; combina fórmulas de hojas con OR.
fn is_open_def_bfs(start: Block) -> Result<Formula, Counterexample> {
    use std::collections::VecDeque;
    let mut queue: VecDeque<Block> = VecDeque::new();
    queue.push_back(start);
    let mut leaf_formulas: Vec<Formula> = Vec::new();
    while let Some(mut block) = queue.pop_front() {
        if block.is_all_in_targets() {
            leaf_formulas.push(block.formula);
            continue;
        }
        if block.is_disjunt_to_targets() {
            leaf_formulas.push(formulas::false_formula(None));
            continue;
        }
        if should_run_isotype_split(&block) {
            match handle_finished_impure(&block)? {
                children if children.len() == 1 => {
                    queue.push_back(children.into_iter().next().unwrap());
                    continue;
                }
                children => {
                    for child in children {
                        queue.push_back(child);
                    }
                    continue;
                }
            }
        }
        match block.step() {
            Err(ce) => return Err(ce),
            Ok(children) => {
                for child in children {
                    queue.push_back(child);
                }
            }
        }
    }
    let mut combined = formulas::false_formula(None);
    for f in leaf_formulas {
        combined = combined.or_formula(&f);
    }
    Ok(combined)
}

fn combine_child_formulas(results: Vec<Result<Formula, Counterexample>>) -> Result<Formula, Counterexample> {
    if let Some(ce) = results.iter().find_map(|r| r.as_ref().err()) {
        return Err(ce.clone());
    }
    let mut combined = formulas::false_formula(None);
    for r in results {
        combined = combined.or_formula(&r.unwrap());
    }
    Ok(combined)
}

/// Explore sibling blocks: serial by default (`max_rayon_children == 1`);
/// Rayon only when `1 < n <= max_rayon_children`.
fn explore_children(children: Vec<Block>, max_rayon_children: usize) -> Result<Formula, Counterexample> {
    let n = children.len();
    if n == 0 {
        return Ok(formulas::false_formula(None));
    }
    if max_rayon_children > 1 && n > 1 && n <= max_rayon_children {
        let results: Vec<_> = children
            .into_par_iter()
            .map(is_open_def_parallel_recursive)
            .collect();
        return combine_child_formulas(results);
    }
    let results: Vec<_> = children
        .into_iter()
        .map(is_open_def_parallel_recursive)
        .collect();
    combine_child_formulas(results)
}

/// Explora el árbol de bloques; con un solo hijo sigue en bucle (evita stack overflow).
/// Paralelismo Rayon acotado por `HitConfig.max_rayon_children` (default 1 = serie).
fn is_open_def_parallel_recursive(mut block: Block) -> Result<Formula, Counterexample> {
    let max_rayon = block.config.max_rayon_children;
    loop {
        if block.is_all_in_targets() {
            return Ok(block.formula);
        }
        if block.is_disjunt_to_targets() {
            return Ok(formulas::false_formula(None));
        }
        if should_run_isotype_split(&block) {
            let children = handle_finished_impure(&block)?;
            if children.len() == 1 {
                block = children.into_iter().next().unwrap();
                continue;
            }
            return explore_children(children, max_rayon);
        }
        match block.step() {
            Err(ce) => return Err(ce),
            Ok(children) => {
                if children.len() == 1 {
                    block = children.into_iter().next().unwrap();
                    continue;
                }
                return explore_children(children, max_rayon);
            }
        }
    }
}

#[allow(dead_code)]
fn is_open_def_iterative(block: &mut Block) -> Result<Formula, Counterexample> {
    #[derive(Clone)]
    enum StackItem {
        Block(Block),
        Result(Formula),
    }
    let mut stack = vec![StackItem::Block(block.clone())];
    let mut accumulate: Vec<(Vec<Block>, Vec<Formula>)> = vec![];

    while let Some(item) = stack.pop() {
        match item {
            StackItem::Block(b) => {
                if b.is_all_in_targets() {
                    stack.push(StackItem::Result(b.formula));
                } else if b.is_disjunt_to_targets() {
                    stack.push(StackItem::Result(formulas::false_formula(None)));
                } else if should_run_isotype_split(&b) {
                    match handle_finished_impure(&b) {
                        Ok(children) => {
                            if children.len() == 1 {
                                stack.push(StackItem::Block(
                                    children.into_iter().next().unwrap(),
                                ));
                            } else {
                                accumulate.push((children.clone(), vec![]));
                                for child in children.into_iter().rev() {
                                    stack.push(StackItem::Block(child));
                                }
                            }
                        }
                        Err(ce) => return Err(ce),
                    }
                } else {
                    let mut bmut = b;
                    match bmut.step() {
                        Ok(children) => {
                            if children.len() == 1 {
                                stack.push(StackItem::Block(children.into_iter().next().unwrap()));
                            } else {
                                accumulate.push((children.clone(), vec![]));
                                for child in children.into_iter().rev() {
                                    stack.push(StackItem::Block(child));
                                }
                            }
                        }
                        Err(ce) => return Err(ce),
                    }
                }
            }
            StackItem::Result(formula) => {
                if accumulate.is_empty() {
                    return Ok(formula);
                }
                let (children, results) = accumulate.pop().unwrap();
                let mut results = results;
                results.push(formula);
                if results.len() == children.len() {
                    let mut combined = formulas::false_formula(None);
                    for r in results {
                        combined = combined.or_formula(&r);
                    }
                    stack.push(StackItem::Result(combined));
                } else {
                    let next_idx = results.len();
                    accumulate.push((children.clone(), results));
                    let next_block = children[next_idx].clone();
                    stack.push(StackItem::Block(next_block));
                }
            }
        }
    }
    Ok(formulas::false_formula(None))
}

/// Cap on IsoType generation steps per tuple (safety against runaway generators).
pub const ISOTYPE_MAX_STEPS: u64 = 50_000;

/// Equality fingerprint of the QF IsoType of `row`, using the same term generator
/// as HIT (`IndicesTupleGenerator` + `TupleHistory`) until the generator finishes.
///
/// The key is independent of absolute element labels: it records history length and
/// which pairs of history positions are equal. Two tuples with the same key agree
/// on all term equalities produced by the HIT generation schedule.
pub fn qf_isotype_equality_key(model: &Model, row: &[i64]) -> Vec<u8> {
    let arity = row.len();
    let mut operations: BTreeMap<usize, Vec<Operation>> = BTreeMap::new();
    for op in model.operations.values() {
        operations.entry(op.arity).or_default().push(op.clone());
    }
    for op_list in operations.values_mut() {
        op_list.sort_by(|a, b| a.sym.cmp(&b.sym));
    }
    let mut th = TupleHistory::new(row.to_vec(), &[]);
    let mut gen = IndicesTupleGenerator::new(Arc::new(operations), arity, vec![], (0..arity).collect());
    let mut steps = 0u64;
    while !gen.finished {
        steps += 1;
        if steps > ISOTYPE_MAX_STEPS {
            break;
        }
        match gen.step() {
            None => {
                gen.finished = true;
                break;
            }
            Some((op, ti)) => {
                let _idx = th.step(&op, &ti);
                if th.has_generated {
                    gen.hubo_nuevo();
                }
            }
        }
    }
    isotype_equality_bytes(&th.history)
}

fn isotype_equality_bytes(history: &[i64]) -> Vec<u8> {
    let n = history.len();
    let mut key = Vec::with_capacity(4 + n * (n.saturating_sub(1)));
    key.extend_from_slice(&(n as u32).to_le_bytes());
    for i in 0..n {
        for j in (i + 1)..n {
            if history[i] == history[j] {
                key.push(1);
            } else {
                key.push(0);
            }
        }
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preprocessing::Pattern;
    use std::collections::{BTreeMap, HashMap};

    fn make_target(arity: usize, tuples: &[Vec<i64>], with_pattern: bool) -> Relation {
        let mut r = Relation::new("T0", arity);
        for t in tuples {
            r.add(t.clone());
        }
        if with_pattern && !tuples.is_empty() {
            r.pattern = Some(Box::new(Pattern::new(tuples[0].clone())));
        }
        r
    }

    #[test]
    fn test_entropy_cero_cero() {
        assert_eq!(entropy(0, 0), 0.0);
    }

    #[test]
    fn test_entropy_mitad_mitad() {
        let h = entropy(5, 5);
        assert!((h - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_entropy_uniforme() {
        let h = entropy(1, 1);
        assert!(h > 0.0 && (h - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_entropy_puro_in() {
        assert!(entropy(10, 0).abs() < 1e-12);
    }

    #[test]
    fn test_entropy_puro_out() {
        assert!(entropy(0, 10).abs() < 1e-12);
    }

    #[test]
    fn test_ig_n_cero() {
        let ig = information_gain_from_counts(0, 0, &HashMap::new());
        assert_eq!(ig, 0.0);
    }

    #[test]
    fn test_ig_particion_perfecta() {
        let mut part = HashMap::new();
        part.insert((0, true), (5, 0));
        part.insert((1, false), (0, 5));
        let ig = information_gain_from_counts(10, 5, &part);
        assert!(ig > 0.9);
    }

    #[test]
    fn test_ig_sin_ganancia() {
        let mut part = HashMap::new();
        part.insert((0, true), (3, 2));
        part.insert((0, false), (2, 3));
        let ig = information_gain_from_counts(10, 5, &part);
        assert!(ig >= 0.0);
    }

    #[test]
    fn test_tuple_history_step_nuevo_elemento() {
        let target = make_target(2, &[vec![0, 0], vec![1, 1]], false);
        let targets = [&target];
        let mut th = TupleHistory::new(vec![0, 0], &targets);
        let mut op = Operation::new("f", 2);
        op.add(vec![0, 0, 1]);
        op.add(vec![0, 1, 0]);
        op.add(vec![1, 0, 0]);
        op.add(vec![1, 1, 0]);
        let idx = th.step(&op, &[0, 1]);
        assert_eq!(idx, 2);
        assert!(th.has_generated);
    }

    #[test]
    fn test_tuple_history_step_existente() {
        let target = make_target(2, &[vec![0, 0], vec![1, 1]], false);
        let targets = [&target];
        let mut th = TupleHistory::new(vec![0, 0], &targets);
        let mut op = Operation::new("id", 1);
        op.add(vec![0, 0]);
        op.add(vec![1, 1]);
        let step_idx = th.step(&op, &[0]);
        assert!(step_idx == 0 || step_idx == 1);
        assert!(!th.has_generated);
    }

    #[test]
    fn test_tuple_history_simulate_step() {
        let target = make_target(2, &[vec![0, 1]], false);
        let targets = [&target];
        let th = TupleHistory::new(vec![0, 0], &targets);
        let mut op = Operation::new("f", 2);
        op.add(vec![0, 0, 1]);
        op.add(vec![0, 1, 0]);
        op.add(vec![1, 0, 0]);
        op.add(vec![1, 1, 1]);
        let (_, has_gen) = th.simulate_step(&op, &[0, 1]);
        assert!(has_gen);
        assert_eq!(th.history.len(), 2);
    }

    #[test]
    fn test_tuple_history_eq_hash() {
        let target = make_target(1, &[vec![0]], false);
        let targets = [&target];
        let a = TupleHistory::new(vec![0], &targets);
        let b = TupleHistory::new(vec![0], &targets);
        assert!(a == b);
        use std::hash::{Hash, Hasher};
        let mut ha = std::collections::hash_map::DefaultHasher::new();
        let mut hb = std::collections::hash_map::DefaultHasher::new();
        a.hash(&mut ha);
        b.hash(&mut hb);
        assert_eq!(ha.finish(), hb.finish());
    }

    #[test]
    fn test_block_is_all_in_targets() {
        let mut op = Operation::new("f", 2);
        op.add(vec![0, 0, 0]);
        op.add(vec![0, 1, 1]);
        op.add(vec![1, 0, 1]);
        op.add(vec![1, 1, 0]);
        let mut ops = BTreeMap::new();
        ops.insert(2, vec![op]);
        let target = make_target(2, &[vec![0, 0], vec![1, 1], vec![0, 1], vec![1, 0]], true);
        let targets = vec![target];
        let th = TupleHistory::new(vec![0, 0], &[&targets[0]]);
        let f = targets[0].pattern.as_ref().unwrap().preprocessed_formula();
        let ambient = Arc::new(Model::new(vec![0, 1], HashMap::new(), HashMap::new()));
        let block = Block::new(ops, ambient, vec![th], targets, f, HitConfig::default());
        assert!(block.is_all_in_targets());
    }

    #[test]
    fn test_block_is_disjunt_to_targets() {
        let target = make_target(2, &[vec![0, 1]], true);
        let targets = vec![target.clone()];
        let th = TupleHistory::new(vec![0, 0], &[&target]);
        let f = target.pattern.as_ref().unwrap().preprocessed_formula();
        let ambient = Arc::new(Model::new(vec![0, 1], HashMap::new(), HashMap::new()));
        let block = Block::new(
            BTreeMap::new(),
            ambient,
            vec![th],
            targets,
            f,
            HitConfig::default(),
        );
        assert!(block.is_disjunt_to_targets());
    }

    #[test]
    fn test_counterexample_raise() {
        let ce = Counterexample(vec![vec![1], vec![2], vec![3]]);
        let s = format!("{:?}", ce);
        assert!(s.contains("1"));
    }

    #[test]
    fn full_target_pattern_pieces_accept_immediately() {
        // R = A^2 after preprocess: each equality-pattern piece is complete, so
        // pattern-filtered initial blocks are full and accept without TMH/stepping.
        let mut op = Operation::new("f", 2);
        for a in 0..4i64 {
            for b in 0..4i64 {
                op.add(vec![a, b, (a + b) % 4]);
            }
        }
        let mut operations = HashMap::new();
        operations.insert("f".into(), op);
        let mut target = Relation::new("T0", 2);
        for a in 0..4i64 {
            for b in 0..4i64 {
                target.add(vec![a, b]);
            }
        }
        let pieces = crate::preprocessing::preprocesamiento2(&target);
        assert!(pieces.len() >= 2, "expected diagonal + off-diagonal pieces");
        let model = Model::new((0..4).collect(), HashMap::new(), operations);
        let mut cfg = HitConfig::default();
        cfg.synthesize_formula = false;
        for piece in pieces {
            let f = is_open_def(&model, vec![piece], cfg).expect("full pattern piece definable");
            let _ = f;
        }
    }
}
