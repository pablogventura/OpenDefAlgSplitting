//! Atomic PP / FO type signatures (parity with fopy.finite.ktypes).

use crate::first_order::models::Model;
use crate::first_order::relops::Operation;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

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

pub fn atomic_pp_type(model: &Model, row: &[i64], max_depth: usize) -> Vec<i64> {
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

pub fn fo_type(model: &Model, row: &[i64], k: usize, arity_vars: usize) -> Vec<u8> {
    if k == 0 {
        let sig = atomic_pp_type(model, row, arity_vars.max(2));
        return sig.iter().flat_map(|x| x.to_le_bytes()).collect();
    }
    let prev = fo_type(model, row, k - 1, arity_vars);
    let mut neighbour_hashes: Vec<u64> = Vec::new();
    for i in 0..row.len() {
        for &u in &model.universe {
            if row[i] == u {
                continue;
            }
            let mut neighbour = row.to_vec();
            neighbour[i] = u;
            let nt = fo_type(model, &neighbour, k - 1, arity_vars);
            let mut h = std::collections::hash_map::DefaultHasher::new();
            nt.hash(&mut h);
            neighbour_hashes.push(h.finish());
        }
    }
    neighbour_hashes.sort_unstable();
    let mut out = prev;
    for nh in neighbour_hashes {
        out.extend_from_slice(&nh.to_le_bytes());
    }
    out
}
