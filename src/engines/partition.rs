//! Tuple partition refinement (parallel keying, deterministic buckets).

use crate::first_order::models::Model;
use crate::first_order::relops::Relation;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::hash::Hash;

pub const MAX_TUPLE_PARTITION: usize = 256;

#[derive(Debug, Clone)]
pub struct TuplePartition {
    pub blocks: Vec<Vec<Vec<i64>>>,
}

impl TuplePartition {
    pub fn from_model(model: &Model, arity: usize) -> Result<Self, String> {
        let tuples = enumerate_tuples(model, arity)?;
        Ok(Self {
            blocks: vec![tuples],
        })
    }

    /// Refine each block by `key_fn`. Keys in parallel; buckets sorted.
    pub fn refine<K, F>(&mut self, key_fn: F)
    where
        K: Eq + Hash + Ord + Send + Sync,
        F: Fn(&[i64]) -> K + Sync,
    {
        let mut refined: Vec<Vec<Vec<i64>>> = Vec::new();
        for block in self.blocks.drain(..) {
            let keyed: Vec<(K, Vec<i64>)> = block
                .into_par_iter()
                .map(|row| {
                    let k = key_fn(&row);
                    (k, row)
                })
                .collect();
            let mut buckets: BTreeMap<K, Vec<Vec<i64>>> = BTreeMap::new();
            for (k, row) in keyed {
                buckets.entry(k).or_default().push(row);
            }
            for mut rows in buckets.into_values() {
                rows.sort();
                refined.push(rows);
            }
        }
        self.blocks = refined;
    }

    pub fn is_target_pure(&self, target: &Relation) -> bool {
        for block in &self.blocks {
            let mut seen_in = false;
            let mut seen_out = false;
            for row in block {
                if target.contains(row) {
                    seen_in = true;
                } else {
                    seen_out = true;
                }
                if seen_in && seen_out {
                    return false;
                }
            }
        }
        true
    }
}

pub fn enumerate_tuples(model: &Model, arity: usize) -> Result<Vec<Vec<i64>>, String> {
    let u = &model.universe;
    if arity == 0 {
        return Ok(vec![vec![]]);
    }
    let count = u.len().checked_pow(arity as u32).unwrap_or(usize::MAX);
    if count > MAX_TUPLE_PARTITION {
        return Err(format!(
            "Cannot enumerate {arity}-tuples: |U|^{arity} = {count} exceeds MAX={MAX_TUPLE_PARTITION}"
        ));
    }
    let mut out = Vec::with_capacity(count);
    let mut cur = vec![0usize; arity];
    loop {
        out.push(cur.iter().map(|&i| u[i]).collect());
        let mut digit = arity;
        loop {
            if digit == 0 {
                return Ok(out);
            }
            digit -= 1;
            cur[digit] += 1;
            if cur[digit] < u.len() {
                break;
            }
            cur[digit] = 0;
        }
    }
}

pub fn has_nontrivial_ops(model: &Model) -> bool {
    model.operations.values().any(|op| op.arity >= 1)
}
