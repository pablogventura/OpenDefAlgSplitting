//! Subalgebra lattice + homomorphisms (parity with fopy.universal / lindenbaum).

use crate::first_order::models::Model;
use crate::first_order::relops::Operation;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct HomArrow {
    pub mapping: HashMap<i64, i64>,
    pub domain: HashSet<i64>,
}

impl HomArrow {
    pub fn vector_call(&self, tup: &[i64]) -> Option<Vec<i64>> {
        let mut out = Vec::with_capacity(tup.len());
        for &x in tup {
            out.push(*self.mapping.get(&x)?);
        }
        Some(out)
    }
}

fn is_subalgebra(model: &Model, sub: &HashSet<i64>) -> bool {
    for op in model.operations.values() {
        if op.arity == 0 {
            if let Some(v) = op.call(&[]) {
                if !sub.contains(&v) {
                    return false;
                }
            }
            continue;
        }
        let elems: Vec<i64> = sub.iter().copied().collect();
        if !check_op_closed(op, &elems, sub) {
            return false;
        }
    }
    true
}

fn check_op_closed(op: &Operation, elems: &[i64], sub: &HashSet<i64>) -> bool {
    fn rec(
        op: &Operation,
        elems: &[i64],
        sub: &HashSet<i64>,
        args: &mut Vec<i64>,
    ) -> bool {
        if args.len() == op.arity {
            if let Some(v) = op.call(args) {
                return sub.contains(&v);
            }
            return true;
        }
        for &e in elems {
            args.push(e);
            if !rec(op, elems, sub, args) {
                return false;
            }
            args.pop();
        }
        true
    }
    let mut args = Vec::new();
    rec(op, elems, sub, &mut args)
}

pub fn subalgebra_lattice(model: &Model) -> Vec<HashSet<i64>> {
    let u = &model.universe;
    if u.len() > 8 {
        let mut result = vec![u.iter().copied().collect()];
        for &a in u {
            let s: HashSet<i64> = [a].into_iter().collect();
            if is_subalgebra(model, &s) {
                result.push(s);
            }
        }
        return result;
    }
    let mut all_subs = Vec::new();
    let n = u.len();
    for mask in 1..(1usize << n) {
        let sub: HashSet<i64> = (0..n)
            .filter(|i| mask & (1 << i) != 0)
            .map(|i| u[i])
            .collect();
        if is_subalgebra(model, &sub) {
            all_subs.push(sub);
        }
    }
    all_subs
}

fn induce_ops(model: &Model, sub: &HashSet<i64>) -> HashMap<String, Operation> {
    model
        .operations
        .iter()
        .map(|(name, op)| (name.clone(), op.restrict(sub)))
        .collect()
}

fn compatible(
    src_ops: &HashMap<String, Operation>,
    tgt_ops: &HashMap<String, Operation>,
    src_u: &[i64],
    m: &HashMap<i64, i64>,
) -> bool {
    for (name, op) in src_ops {
        let Some(top) = tgt_ops.get(name) else {
            return false;
        };
        if op.arity == 0 {
            match (op.call(&[]), top.call(&[])) {
                (Some(sv), Some(tv)) => {
                    if m.get(&sv) != Some(&tv) {
                        return false;
                    }
                }
                _ => return false,
            }
            continue;
        }
        if !check_hom_op(op, top, src_u, m) {
            return false;
        }
    }
    true
}

fn check_hom_op(
    op: &Operation,
    top: &Operation,
    src_u: &[i64],
    m: &HashMap<i64, i64>,
) -> bool {
    fn rec(
        op: &Operation,
        top: &Operation,
        src_u: &[i64],
        m: &HashMap<i64, i64>,
        args: &mut Vec<i64>,
    ) -> bool {
        if args.len() == op.arity {
            if !args.iter().all(|a| m.contains_key(a)) {
                return true;
            }
            let Some(s) = op.call(args) else {
                return false;
            };
            let t_args: Vec<i64> = args.iter().map(|a| m[a]).collect();
            let Some(t) = top.call(&t_args) else {
                return false;
            };
            return m.get(&s) == Some(&t);
        }
        for &e in src_u {
            args.push(e);
            if !rec(op, top, src_u, m, args) {
                return false;
            }
            args.pop();
        }
        true
    }
    let mut args = Vec::new();
    rec(op, top, src_u, m, &mut args)
}

fn homomorphisms_between(
    src_ops: &HashMap<String, Operation>,
    tgt_ops: &HashMap<String, Operation>,
    src_u: &[i64],
    tgt_u: &[i64],
) -> Vec<HashMap<i64, i64>> {
    if src_u.len() > 6 || tgt_u.len() > 6 {
        return vec![];
    }
    let mut maps = Vec::new();

    fn extend(
        src_ops: &HashMap<String, Operation>,
        tgt_ops: &HashMap<String, Operation>,
        src_u: &[i64],
        tgt_u: &[i64],
        m: &mut HashMap<i64, i64>,
        idx: usize,
        maps: &mut Vec<HashMap<i64, i64>>,
    ) {
        if idx == src_u.len() {
            if compatible(src_ops, tgt_ops, src_u, m) {
                maps.push(m.clone());
            }
            return;
        }
        let x = src_u[idx];
        for &y in tgt_u {
            m.insert(x, y);
            if compatible(src_ops, tgt_ops, src_u, m) {
                extend(src_ops, tgt_ops, src_u, tgt_u, m, idx + 1, maps);
            }
            m.remove(&x);
        }
    }

    let mut m = HashMap::new();
    extend(src_ops, tgt_ops, src_u, tgt_u, &mut m, 0, &mut maps);
    maps
}

/// Enumerate morphisms for EP (all homs) or open (isos + inverses).
pub fn morphisms(model: &Model, open_only: bool) -> Vec<HomArrow> {
    let subs = subalgebra_lattice(model);
    let pairs: Vec<_> = subs
        .iter()
        .flat_map(|s| subs.iter().map(move |t| (s, t)))
        .collect();

    let partial: Vec<HomArrow> = pairs
        .par_iter()
        .flat_map(|(source_sub, target_sub)| {
            let src_ops = induce_ops(model, source_sub);
            let tgt_ops = induce_ops(model, target_sub);
            let mut src_u: Vec<i64> = source_sub.iter().copied().collect();
            let mut tgt_u: Vec<i64> = target_sub.iter().copied().collect();
            src_u.sort_unstable();
            tgt_u.sort_unstable();
            let mut local = Vec::new();
            for mapping in homomorphisms_between(&src_ops, &tgt_ops, &src_u, &tgt_u) {
                let image: HashSet<i64> = mapping.values().copied().collect();
                if open_only {
                    if image != **target_sub || image.len() != source_sub.len() {
                        continue;
                    }
                }
                local.push(HomArrow {
                    mapping: mapping.clone(),
                    domain: (**source_sub).clone(),
                });
                if open_only && image.len() == source_sub.len() {
                    let inv: HashMap<i64, i64> = mapping.iter().map(|(s, t)| (*t, *s)).collect();
                    local.push(HomArrow {
                        mapping: inv,
                        domain: image,
                    });
                }
            }
            local
        })
        .collect();

    // Dedup by (domain sorted, mapping items)
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for arrow in partial {
        let mut dom: Vec<_> = arrow.domain.iter().copied().collect();
        dom.sort_unstable();
        let mut items: Vec<_> = arrow.mapping.iter().map(|(a, b)| (*a, *b)).collect();
        items.sort_unstable();
        let key = (dom, items);
        if seen.insert(key) {
            out.push(arrow);
        }
    }
    out
}
