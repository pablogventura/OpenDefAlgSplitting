//! Positive equality-diagram split (`qf_pos`).

use crate::engines::dispatch::EngineOutcome;
use crate::engines::partition::{has_nontrivial_ops, TuplePartition};
use crate::first_order::models::Model;
use crate::first_order::relops::Relation;

fn positive_eq_key(row: &[i64]) -> Vec<(usize, usize)> {
    let k = row.len();
    let mut pairs = Vec::new();
    for i in 0..k {
        for j in 0..k {
            if row[i] == row[j] {
                pairs.push((i, j));
            }
        }
    }
    pairs
}

pub fn check_positive_split(model: &Model, target: &Relation) -> Result<EngineOutcome, String> {
    let mut partition = TuplePartition::from_model(model, target.arity)?;
    partition.refine(positive_eq_key);
    let engine = if has_nontrivial_ops(model) {
        "positive_split_ops_partial"
    } else {
        "positive_split"
    };
    Ok(EngineOutcome {
        definable: partition.is_target_pure(target),
        fragment: "qf_pos".into(),
        engine: engine.into(),
    })
}
