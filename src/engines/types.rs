//! Atomic PP / FO type signatures (parity with fopy.finite.ktypes).

use crate::first_order::models::Model;
use crate::first_order::relops::Operation;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// Cap term depth used inside FO-k neighbour signatures (independent of arity).
pub const FO_PP_DEPTH_CAP: usize = 2;
/// Under ternary explosion risk, cap PP depth inside FO neighbours (memo still shared).
pub const FO_PP_DEPTH_RISK_CAP: usize = 1;

#[derive(Clone)]
enum TermNode {
    Var(usize),
    Op {
        sym: String,
        arity: usize,
        args: Vec<TermNode>,
    },
}

impl TermNode {
    fn depth(&self) -> usize {
        match self {
            TermNode::Var(_) => 0,
            TermNode::Op { args, .. } => 1 + args.iter().map(|a| a.depth()).max().unwrap_or(0),
        }
    }

    fn eval(&self, ops: &BTreeMap<String, &Operation>, row: &[i64]) -> i64 {
        match self {
            TermNode::Var(i) => row.get(*i).copied().unwrap_or(-1),
            TermNode::Op { sym, arity, args } => {
                let Some(op) = ops.get(sym) else {
                    return -1;
                };
                let arg_vals: Vec<i64> = args.iter().map(|a| a.eval(ops, row)).collect();
                if arg_vals.len() != *arity {
                    return -1;
                }
                op.call(&arg_vals).unwrap_or(-1)
            }
        }
    }

    fn sort_key(&self) -> String {
        match self {
            TermNode::Var(i) => format!("v{i}"),
            TermNode::Op { sym, args, .. } => {
                let inner: Vec<_> = args.iter().map(|a| a.sort_key()).collect();
                format!("{sym}({})", inner.join(","))
            }
        }
    }
}

fn term_arg_tuples(terms: &[TermNode], arity: usize) -> Vec<Vec<TermNode>> {
    if arity == 0 {
        return vec![vec![]];
    }
    let mut result = vec![vec![]];
    for _ in 0..arity {
        let mut next = Vec::new();
        for r in &result {
            for t in terms {
                let mut row = r.clone();
                row.push(t.clone());
                next.push(row);
            }
        }
        result = next;
    }
    result
}

thread_local! {
    static PP_CACHE: RefCell<HashMap<(Vec<i64>, usize), Vec<i64>>> =
        RefCell::new(HashMap::new());
    static FO_CACHE: RefCell<HashMap<(Vec<i64>, usize, usize), Vec<u8>>> =
        RefCell::new(HashMap::new());
    static GUARDED_CACHE: RefCell<HashMap<(Vec<i64>, usize, usize), Vec<u8>>> =
        RefCell::new(HashMap::new());
}

/// Clear thread-local type caches (call between independent engine runs if needed).
pub fn clear_type_caches() {
    PP_CACHE.with(|c| c.borrow_mut().clear());
    FO_CACHE.with(|c| c.borrow_mut().clear());
    GUARDED_CACHE.with(|c| c.borrow_mut().clear());
}

pub fn atomic_pp_type(model: &Model, row: &[i64], max_depth: usize) -> Vec<i64> {
    let key = (row.to_vec(), max_depth);
    if let Some(hit) = PP_CACHE.with(|c| c.borrow().get(&key).cloned()) {
        return hit;
    }
    let computed = atomic_pp_type_uncached(model, row, max_depth);
    PP_CACHE.with(|c| {
        c.borrow_mut().insert(key, computed.clone());
    });
    computed
}

fn atomic_pp_type_uncached(model: &Model, row: &[i64], max_depth: usize) -> Vec<i64> {
    let arity = row.len();
    let mut ops_by_arity: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for (sym, op) in &model.operations {
        ops_by_arity.entry(op.arity).or_default().push(sym.clone());
    }
    for syms in ops_by_arity.values_mut() {
        syms.sort();
    }
    let op_refs: BTreeMap<String, &Operation> = model
        .operations
        .iter()
        .map(|(k, v)| (k.clone(), v))
        .collect();

    let mut terms: Vec<TermNode> = (0..arity).map(TermNode::Var).collect();
    let mut signatures: Vec<i64> = Vec::new();

    for depth in 0..=max_depth {
        let mut current: Vec<&TermNode> = terms.iter().filter(|t| t.depth() == depth).collect();
        current.sort_by_key(|t| t.sort_key());
        for term in current {
            signatures.push(term.eval(&op_refs, row));
        }
        if depth == max_depth {
            break;
        }
        let mut next_terms = Vec::new();
        for (&ar, syms) in &ops_by_arity {
            if ar == 0 {
                continue;
            }
            for sym in syms {
                for args in term_arg_tuples(&terms, ar) {
                    next_terms.push(TermNode::Op {
                        sym: sym.clone(),
                        arity: ar,
                        args,
                    });
                }
            }
        }
        terms.extend(next_terms);
    }
    signatures
}

/// FO-k type: PP matrix plus multiset of `(k-1)`-types of **unguarded** neighbours
/// (any one-coordinate change). Differs from [`guarded_type`], which only walks
/// guarded extensions.
///
/// Adaptive under [`type_explosion_risk`]: still computes full k (no blind k=0),
/// but ranks neighbours by cheap PP delta first and caps PP depth to
/// [`FO_PP_DEPTH_RISK_CAP`] so memoized walks stay lighter (e.g. modeloqueanda).
pub fn fo_type(model: &Model, row: &[i64], k: usize, arity_vars: usize) -> Vec<u8> {
    let key = (row.to_vec(), k, arity_vars);
    if let Some(hit) = FO_CACHE.with(|c| c.borrow().get(&key).cloned()) {
        return hit;
    }
    let computed = fo_type_uncached(model, row, k, arity_vars);
    FO_CACHE.with(|c| {
        c.borrow_mut().insert(key, computed.clone());
    });
    computed
}

fn fo_pp_depth_for(model: &Model, arity_vars: usize) -> usize {
    let base = arity_vars.max(2).min(FO_PP_DEPTH_CAP);
    if type_explosion_risk(model, arity_vars) {
        base.min(FO_PP_DEPTH_RISK_CAP)
    } else {
        base
    }
}

/// Choose FO split k: never blind-force 0; cap under explosion risk; allow k>1 on small grids.
pub fn choose_fo_split_k(model: &Model, target_arity: usize, max_k: usize) -> usize {
    let bound = max_k.max(1);
    if type_explosion_risk(model, target_arity) {
        return bound.min(1);
    }
    let n = model.universe.len();
    let power = if target_arity == 0 {
        n
    } else {
        n.saturating_pow(target_arity as u32)
    };
    if power <= 16 && bound >= 2 {
        bound
    } else {
        bound.min(1)
    }
}

fn fo_type_uncached(model: &Model, row: &[i64], k: usize, arity_vars: usize) -> Vec<u8> {
    let pp_depth = fo_pp_depth_for(model, arity_vars);
    if k == 0 {
        let sig = atomic_pp_type(model, row, pp_depth);
        return sig.iter().flat_map(|x| x.to_le_bytes()).collect();
    }
    let prev = fo_type(model, row, k - 1, arity_vars);
    // Rank neighbours: PP-changing moves first (cheaper discrimination / warmer memo),
    // then the rest. Multiset of hashes is order-independent after sort.
    let self_pp = atomic_pp_type(model, row, pp_depth);
    let mut ranked: Vec<(u8, usize, i64)> = Vec::new();
    for i in 0..row.len() {
        for &u in &model.universe {
            if row[i] == u {
                continue;
            }
            let mut neighbour = row.to_vec();
            neighbour[i] = u;
            let nb_pp = atomic_pp_type(model, &neighbour, pp_depth);
            let rank = if nb_pp != self_pp { 0u8 } else { 1u8 };
            ranked.push((rank, i, u));
        }
    }
    ranked.sort_unstable_by_key(|&(rank, i, u)| (rank, i, u));
    let mut neighbour_hashes: Vec<u64> = Vec::with_capacity(ranked.len());
    for &(_, i, u) in &ranked {
        let mut neighbour = row.to_vec();
        neighbour[i] = u;
        let nt = fo_type(model, &neighbour, k - 1, arity_vars);
        let mut h = std::collections::hash_map::DefaultHasher::new();
        nt.hash(&mut h);
        neighbour_hashes.push(h.finish());
    }
    neighbour_hashes.sort_unstable();
    let mut out = prev;
    for nh in neighbour_hashes {
        out.extend_from_slice(&nh.to_le_bytes());
    }
    out
}

/// Lean ``HasAtomicGuard``: non-trivial coordinate equality or RelMap atom.
/// Compound term equalities from ops are not guards (parity with fopy).
fn atomic_guard_holds_mentioning(
    model: &Model,
    row: &[i64],
    var_idx: usize,
    _guard_depth: usize,
) -> bool {
    if var_idx >= row.len() {
        return false;
    }
    let arity = row.len();
    // HasEqGuard
    for j in 0..arity {
        if j != var_idx && row[var_idx] == row[j] {
            return true;
        }
    }
    // HasRelGuard
    for rel in model.relations.values() {
        let m = rel.arity;
        if m == 0 {
            continue;
        }
        let idx_count = arity.pow(m as u32);
        for idx in 0..idx_count {
            let mut idxs = Vec::with_capacity(m);
            let mut x = idx;
            for _ in 0..m {
                idxs.push(x % arity);
                x /= arity;
            }
            if !idxs.contains(&var_idx) {
                continue;
            }
            let args: Vec<i64> = idxs.iter().map(|&j| row[j]).collect();
            if rel.contains(&args) {
                return true;
            }
        }
    }
    false
}

pub fn is_guarded_neighbour(
    model: &Model,
    row: &[i64],
    coord: usize,
    value: i64,
    guard_depth: usize,
) -> bool {
    if coord >= row.len() || row[coord] == value {
        return false;
    }
    let mut neighbour = row.to_vec();
    neighbour[coord] = value;
    atomic_guard_holds_mentioning(model, &neighbour, coord, guard_depth)
}

pub fn guarded_type(model: &Model, row: &[i64], k: usize, arity_vars: usize) -> Vec<u8> {
    let key = (row.to_vec(), k, arity_vars);
    if let Some(hit) = GUARDED_CACHE.with(|c| c.borrow().get(&key).cloned()) {
        return hit;
    }
    let computed = guarded_type_uncached(model, row, k, arity_vars);
    GUARDED_CACHE.with(|c| {
        c.borrow_mut().insert(key, computed.clone());
    });
    computed
}

fn guarded_type_uncached(model: &Model, row: &[i64], k: usize, arity_vars: usize) -> Vec<u8> {
    let guard_depth = arity_vars.max(FO_PP_DEPTH_CAP);
    if k == 0 {
        let sig = atomic_pp_type(model, row, guard_depth);
        return sig.iter().flat_map(|x| x.to_le_bytes()).collect();
    }
    let prev = guarded_type(model, row, k - 1, arity_vars);
    let mut neighbour_hashes: Vec<u64> = Vec::new();
    for i in 0..row.len() {
        for &u in &model.universe {
            if is_guarded_neighbour(model, row, i, u, guard_depth) {
                let mut neighbour = row.to_vec();
                neighbour[i] = u;
                let nt = guarded_type(model, &neighbour, k - 1, arity_vars);
                let mut h = std::collections::hash_map::DefaultHasher::new();
                nt.hash(&mut h);
                neighbour_hashes.push(h.finish());
            }
        }
    }
    neighbour_hashes.sort_unstable();
    let mut out = prev;
    for nh in neighbour_hashes {
        out.extend_from_slice(&nh.to_le_bytes());
    }
    out
}

pub fn guarded_neighbour_orbit(model: &Model, row: &[i64], guard_depth: usize) -> BTreeSet<Vec<i64>> {
    let mut orbit: HashSet<Vec<i64>> = HashSet::new();
    orbit.insert(row.to_vec());
    let mut changed = true;
    while changed {
        changed = false;
        let snapshot: Vec<_> = orbit.iter().cloned().collect();
        for tup in snapshot {
            for i in 0..tup.len() {
                for &u in &model.universe {
                    if is_guarded_neighbour(model, &tup, i, u, guard_depth) {
                        let mut nb = tup.clone();
                        nb[i] = u;
                        if orbit.insert(nb) {
                            changed = true;
                        }
                    }
                }
            }
        }
    }
    orbit.into_iter().collect()
}

/// True when FO/GF type computation is likely to explode (ternary+ ops, arity>=3).
/// Used as a *cost signal* (PP-depth cap / GF k fallback), not to blind FO to k=0.
pub fn type_explosion_risk(model: &Model, target_arity: usize) -> bool {
    target_arity >= 3
        && model
            .operations
            .values()
            .any(|op| op.arity >= 3)
}

/// Equality pattern of a row (pairs i<j with equal coordinates).
/// Used for EP types when the operational signature is empty: absolute element
/// ids are not EP-definable without constants/ops.
fn equality_pattern(row: &[i64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(row.len() * row.len() / 2);
    for i in 0..row.len() {
        for j in (i + 1)..row.len() {
            out.push(if row[i] == row[j] { 1 } else { 0 });
        }
    }
    out
}

fn ep_matrix_type(model: &Model, row: &[i64], max_depth: usize) -> Vec<u8> {
    if model.operations.is_empty() {
        equality_pattern(row)
    } else {
        atomic_pp_type(model, row, max_depth)
            .iter()
            .flat_map(|x| x.to_le_bytes())
            .collect()
    }
}

/// EP type: PP (or equality) matrix plus multiset of types of bounded
/// existential extensions. Parity with `fopy.finite.fragments.ep_ktypes`.
pub fn ep_type(
    model: &Model,
    row: &[i64],
    max_depth: usize,
    max_existentials: usize,
) -> Vec<u8> {
    let base = ep_matrix_type(model, row, max_depth);
    if max_existentials == 0 {
        return base;
    }
    let universe = &model.universe;
    let mut extensions: Vec<Vec<u8>> = Vec::new();
    // Cartesian products U^m for m = 1..=max_existentials.
    let mut extras: Vec<Vec<i64>> = vec![vec![]];
    for _ in 0..max_existentials {
        let mut next = Vec::new();
        for prefix in &extras {
            for &u in universe {
                let mut e = prefix.clone();
                e.push(u);
                next.push(e);
            }
        }
        extras = next;
        for extra in &extras {
            let mut extended = row.to_vec();
            extended.extend_from_slice(extra);
            extensions.push(ep_matrix_type(model, &extended, max_depth));
        }
    }
    extensions.sort();
    let mut out = base;
    out.push(0xff); // separator
    for ext in extensions {
        out.extend_from_slice(&(ext.len() as u32).to_le_bytes());
        out.extend_from_slice(&ext);
    }
    out
}
