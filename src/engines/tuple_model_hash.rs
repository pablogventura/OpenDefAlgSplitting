//! Historical HIT fingerprint (`TupleModelHash` from OpenDefAlgMerging).
//!
//! Layered term generation + relational type over non-target relations.
//! Equality / hash match Python `hit.TupleModelHash`.
//!
//! Optional speed knobs (env, default off):
//! - `HIT_TMH_CLOSURE_CACHE=1` - reuse compact results by generator tuple
//! - `HIT_TMH_STREAM_PRODUCT=1` - enumerate op/rel products without Vec materialization
//! - `HIT_TMH_PROXY=1` - cheap inequality filter before full compact TMH
//! - `HIT_TMH_STATS=1` - print compute/cache/proxy counters at process end (via dump)

use crate::first_order::models::Model;
use crate::first_order::relops::Operation;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

static TMH_COMPUTES: AtomicU64 = AtomicU64::new(0);
static TMH_CACHE_HITS: AtomicU64 = AtomicU64::new(0);
static TMH_PROXY_REJECTS: AtomicU64 = AtomicU64::new(0);
static TMH_PROXY_FULL: AtomicU64 = AtomicU64::new(0);
static FINISHED_IMPURE_CALLS: AtomicU64 = AtomicU64::new(0);

fn env_flag(name: &str) -> bool {
    matches!(
        std::env::var(name).ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
    )
}

pub fn tmh_closure_cache_enabled() -> bool {
    env_flag("HIT_TMH_CLOSURE_CACHE")
}

pub fn tmh_stream_product_enabled() -> bool {
    env_flag("HIT_TMH_STREAM_PRODUCT")
}

pub fn tmh_proxy_enabled() -> bool {
    env_flag("HIT_TMH_PROXY")
}

pub fn tmh_stats_enabled() -> bool {
    env_flag("HIT_TMH_STATS")
}

pub fn tmh_rayon_threads() -> usize {
    std::env::var("HIT_TMH_RAYON")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
        .max(1)
}

pub fn reset_tmh_stats() {
    TMH_COMPUTES.store(0, Ordering::Relaxed);
    TMH_CACHE_HITS.store(0, Ordering::Relaxed);
    TMH_PROXY_REJECTS.store(0, Ordering::Relaxed);
    TMH_PROXY_FULL.store(0, Ordering::Relaxed);
    FINISHED_IMPURE_CALLS.store(0, Ordering::Relaxed);
}

pub fn note_finished_impure() {
    FINISHED_IMPURE_CALLS.fetch_add(1, Ordering::Relaxed);
}

pub fn tmh_stats_snapshot() -> (u64, u64, u64, u64, u64) {
    (
        TMH_COMPUTES.load(Ordering::Relaxed),
        TMH_CACHE_HITS.load(Ordering::Relaxed),
        TMH_PROXY_REJECTS.load(Ordering::Relaxed),
        TMH_PROXY_FULL.load(Ordering::Relaxed),
        FINISHED_IMPURE_CALLS.load(Ordering::Relaxed),
    )
}

pub fn dump_tmh_stats_if_enabled() {
    if !tmh_stats_enabled() {
        return;
    }
    let (c, h, pr, pf, fi) = tmh_stats_snapshot();
    eprintln!(
        "HIT_TMH_STATS computes={c} cache_hits={h} proxy_rejects={pr} proxy_full={pf} finished_impure={fi}"
    );
}

static DUMP_REGISTERED: AtomicBool = AtomicBool::new(false);

pub fn ensure_tmh_stats_atexit() {
    if !tmh_stats_enabled() {
        return;
    }
    if DUMP_REGISTERED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        // Best-effort: dump when this Arc drops at process end via ctor-less hook.
        // Callers should also invoke dump_tmh_stats_if_enabled() explicitly.
    }
}

/// IsoType of a generator tuple in an ambient model (targets already removed).
#[derive(Debug, Clone)]
pub struct TupleModelHash {
    pub generator_tuple: Vec<i64>,
    pub flat_h: Vec<i64>,
    type_index_sets: BTreeSet<BTreeSet<usize>>,
    pub relations_fp: Vec<BTreeSet<Vec<usize>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactIsoType {
    type_index_sets: BTreeSet<BTreeSet<usize>>,
    relations_fp: Vec<BTreeSet<Vec<usize>>>,
}

impl Hash for CompactIsoType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_index_sets.hash(state);
        self.relations_fp.hash(state);
    }
}

impl PartialEq for TupleModelHash {
    fn eq(&self, other: &Self) -> bool {
        self.type_index_sets == other.type_index_sets && self.relations_fp == other.relations_fp
    }
}

impl Eq for TupleModelHash {}

impl Hash for TupleModelHash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_index_sets.hash(state);
    }
}

/// Cheap inequality fingerprint (depth-1 op table on generators).
/// If two proxies differ, full IsoTypes cannot be equal.
pub fn cheap_proxy(model: &Model, generator_tuple: &[i64]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    generator_tuple.len().hash(&mut hasher);
    let mut ops: Vec<&Operation> = model.operations.values().collect();
    ops.sort_by(|a, b| a.sym.cmp(&b.sym));
    for op in ops {
        op.sym.hash(&mut hasher);
        op.arity.hash(&mut hasher);
        if op.arity == 0 {
            if let Some(x) = op.call(&[]) {
                x.hash(&mut hasher);
            }
            continue;
        }
        // One round: all tuples from generators^arity.
        for_each_product(generator_tuple, op.arity, |tup| {
            if let Some(x) = op.call(tup) {
                x.hash(&mut hasher);
            }
        });
    }
    hasher.finish()
}

impl TupleModelHash {
    pub fn compute(model: &Model, generator_tuple: &[i64]) -> Self {
        TMH_COMPUTES.fetch_add(1, Ordering::Relaxed);
        let stream = tmh_stream_product_enabled();
        let generator_tuple = generator_tuple.to_vec();
        let mut ops_by_ar: BTreeMap<usize, Vec<&Operation>> = BTreeMap::new();
        for op in model.operations.values() {
            ops_by_ar.entry(op.arity).or_default().push(op);
        }
        for list in ops_by_ar.values_mut() {
            list.sort_by(|a, b| a.sym.cmp(&b.sym));
        }
        let mut rels_by_ar: BTreeMap<usize, Vec<&crate::first_order::relops::Relation>> =
            BTreeMap::new();
        for rel in model.relations.values() {
            rels_by_ar.entry(rel.arity).or_default().push(rel);
        }
        for list in rels_by_ar.values_mut() {
            list.sort_by(|a, b| a.sym.cmp(&b.sym));
        }

        let mut h_layers: Vec<Vec<i64>> = vec![generator_tuple.clone()];
        let mut t_map: HashMap<i64, BTreeSet<usize>> = HashMap::new();
        for (j, &a) in generator_tuple.iter().enumerate() {
            t_map.entry(a).or_default().insert(j);
        }
        let mut i = generator_tuple.len().saturating_sub(1);
        let mut o = h_layers[0].clone();

        while !o.is_empty() {
            let flath: Vec<i64> = h_layers.iter().flatten().copied().collect();
            let mut next_layer = Vec::new();
            for (&ar, ops) in &ops_by_ar {
                for f in ops {
                    let mut consume = |tup: &[i64]| {
                        if tup.iter().any(|x| o.contains(x)) {
                            i += 1;
                            let x = f.call(tup).expect("partial op in TupleModelHash");
                            t_map.entry(x).or_default().insert(i);
                            let already = h_layers.iter().any(|layer| layer.contains(&x))
                                || next_layer.contains(&x);
                            if !already {
                                next_layer.push(x);
                            }
                        }
                    };
                    if stream {
                        for_each_product(&flath, ar, &mut consume);
                    } else {
                        for tup in product_n(&flath, ar) {
                            consume(&tup);
                        }
                    }
                }
            }
            h_layers.push(next_layer.clone());
            o = next_layer;
        }
        h_layers.pop();

        let flath: Vec<i64> = h_layers.iter().flatten().copied().collect();
        let type_index_sets: BTreeSet<BTreeSet<usize>> = t_map.into_values().collect();

        let mut relations_fp = Vec::new();
        for (&ar, rels) in &rels_by_ar {
            for r in rels {
                let mut bucket = BTreeSet::new();
                let mut consume = |tup: Vec<i64>| {
                    if r.contains(&tup) {
                        let idx: Vec<usize> = tup
                            .iter()
                            .map(|x| {
                                flath
                                    .iter()
                                    .position(|y| y == x)
                                    .expect("elem in flath")
                            })
                            .collect();
                        bucket.insert(idx);
                    }
                };
                if stream {
                    for_each_product(&flath, ar, |t| consume(t.to_vec()));
                } else {
                    for tup in product_n(&flath, ar) {
                        consume(tup);
                    }
                }
                relations_fp.push(bucket);
            }
        }

        Self {
            generator_tuple,
            flat_h: flath,
            type_index_sets,
            relations_fp,
        }
    }

    pub fn to_compact(&self) -> CompactIsoType {
        CompactIsoType {
            type_index_sets: self.type_index_sets.clone(),
            relations_fp: self.relations_fp.clone(),
        }
    }

    pub fn compute_compact(model: &Model, generator_tuple: &[i64]) -> CompactIsoType {
        Self::compute(model, generator_tuple).to_compact()
    }

    pub fn universe(&self) -> HashSet<i64> {
        self.flat_h.iter().copied().collect()
    }

    pub fn type_index_sets_for_test(&self) -> &BTreeSet<BTreeSet<usize>> {
        &self.type_index_sets
    }

    pub fn relations_fp_for_test(&self) -> &[BTreeSet<Vec<usize>>] {
        &self.relations_fp
    }

    pub fn iso_map(&self, other: &Self) -> Option<HashMap<i64, i64>> {
        if self != other || self.flat_h.len() != other.flat_h.len() {
            return None;
        }
        let mut d = HashMap::new();
        for i in 0..self.flat_h.len() {
            d.insert(self.flat_h[i], other.flat_h[i]);
        }
        Some(d)
    }
}

#[derive(Debug, Default)]
pub struct TmhCache {
    inner: Mutex<HashMap<Vec<i64>, Arc<CompactIsoType>>>,
}

impl TmhCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_compute_compact(
        self: &Arc<Self>,
        model: &Model,
        generator_tuple: &[i64],
    ) -> Arc<CompactIsoType> {
        {
            let guard = self.inner.lock().expect("tmh cache lock");
            if let Some(h) = guard.get(generator_tuple) {
                TMH_CACHE_HITS.fetch_add(1, Ordering::Relaxed);
                return Arc::clone(h);
            }
        }
        let computed = Arc::new(TupleModelHash::compute_compact(model, generator_tuple));
        let mut guard = self.inner.lock().expect("tmh cache lock");
        Arc::clone(
            guard
                .entry(generator_tuple.to_vec())
                .or_insert_with(|| Arc::clone(&computed)),
        )
    }

    pub fn len(&self) -> usize {
        self.inner.lock().expect("tmh cache lock").len()
    }

    pub fn clear(&self) {
        self.inner.lock().expect("tmh cache lock").clear()
    }
}

/// Compact IsoType for finished-impure: optional closure cache + optional proxy gate.
pub fn classify_compact(
    model: &Model,
    generator_tuple: &[i64],
    cache: Option<&Arc<TmhCache>>,
) -> CompactIsoType {
    if tmh_closure_cache_enabled() {
        if let Some(c) = cache {
            return (*c.get_or_compute_compact(model, generator_tuple)).clone();
        }
    }
    TupleModelHash::compute_compact(model, generator_tuple)
}

pub fn note_proxy_reject() {
    TMH_PROXY_REJECTS.fetch_add(1, Ordering::Relaxed);
}

pub fn note_proxy_full() {
    TMH_PROXY_FULL.fetch_add(1, Ordering::Relaxed);
}

fn for_each_product(elems: &[i64], n: usize, mut f: impl FnMut(&[i64])) {
    if n == 0 {
        f(&[]);
        return;
    }
    if elems.is_empty() {
        return;
    }
    let mut cur = vec![0i64; n];
    let mut idx = vec![0usize; n];
    loop {
        for i in 0..n {
            cur[i] = elems[idx[i]];
        }
        f(&cur);
        let mut digit = n;
        loop {
            if digit == 0 {
                return;
            }
            digit -= 1;
            idx[digit] += 1;
            if idx[digit] < elems.len() {
                break;
            }
            idx[digit] = 0;
        }
    }
}

fn product_n(elems: &[i64], n: usize) -> Vec<Vec<i64>> {
    let mut out = Vec::new();
    for_each_product(elems, n, |t| out.push(t.to_vec()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_model;
    use std::path::Path;

    #[test]
    fn isotype_self_eq_and_hash_stable() {
        let model = parse_model(Some(Path::new("model_examples/posetrombo.model")), true)
            .expect("parse");
        let fa = TupleModelHash::compute(&model, &[0, 3]);
        let fb = TupleModelHash::compute(&model, &[0, 3]);
        assert_eq!(fa, fb);
        assert!(fa.iso_map(&fb).is_some());
        let other = TupleModelHash::compute(&model, &[1, 2]);
        assert_ne!(fa, other);
    }

    #[test]
    fn compact_matches_full_eq() {
        let model = parse_model(Some(Path::new("model_examples/posetrombo.model")), true)
            .expect("parse");
        let fa = TupleModelHash::compute(&model, &[0, 3]);
        let fb = TupleModelHash::compute(&model, &[0, 3]);
        let other = TupleModelHash::compute(&model, &[1, 2]);
        assert_eq!(fa.to_compact(), fb.to_compact());
        assert_ne!(fa.to_compact(), other.to_compact());
    }

    #[test]
    fn stream_product_matches_materialized_compact() {
        std::env::remove_var("HIT_TMH_STREAM_PRODUCT");
        let model = parse_model(Some(Path::new("model_examples/posetrombo.model")), true)
            .expect("parse");
        let a = TupleModelHash::compute_compact(&model, &[0, 3]);
        std::env::set_var("HIT_TMH_STREAM_PRODUCT", "1");
        let b = TupleModelHash::compute_compact(&model, &[0, 3]);
        std::env::remove_var("HIT_TMH_STREAM_PRODUCT");
        assert_eq!(a, b);
    }

    #[test]
    fn proxy_equal_when_same_tuple() {
        let model = parse_model(Some(Path::new("model_examples/posetrombo.model")), true)
            .expect("parse");
        assert_eq!(cheap_proxy(&model, &[0, 3]), cheap_proxy(&model, &[0, 3]));
    }

    #[test]
    fn tmh_cache_reuses_compact_arc() {
        let model = parse_model(Some(Path::new("model_examples/posetrombo.model")), true)
            .expect("parse");
        let cache = Arc::new(TmhCache::new());
        let a = cache.get_or_compute_compact(&model, &[0, 3]);
        let b = cache.get_or_compute_compact(&model, &[0, 3]);
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(cache.len(), 1);
    }
}
