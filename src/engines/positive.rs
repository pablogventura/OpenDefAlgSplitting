//! Positive equality / term-diagram split (`qf_pos`).
//!
//! NoFunctions: coordinate equality diagrams (`SamePositiveEqDiagram`).
//! With ops (finite algebraic): positive term diagrams via term-collapse
//! (`TermKaForwardCollapse` / Lean `PositiveCampercholi.of_finite`).

use crate::engines::dispatch::EngineOutcome;
use crate::engines::partition::{has_nontrivial_ops, TuplePartition};
use crate::engines::types::atomic_pp_type;
use crate::first_order::models::Model;
use crate::first_order::relops::Relation;

const POSITIVE_TERM_DEPTH: usize = 2;

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

/// Equality pattern among term evaluations up to Ka-depth (term-collapse key).
fn positive_term_diagram_key(evals: &[i64]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for i in 0..evals.len() {
        for j in (i + 1)..evals.len() {
            if evals[i] == evals[j] {
                pairs.push((i, j));
            }
        }
    }
    pairs
}

pub fn check_positive_split(model: &Model, target: &Relation) -> Result<EngineOutcome, String> {
    let mut partition = TuplePartition::from_model(model, target.arity)?;
    let engine = if has_nontrivial_ops(model) {
        // Lean: positiveTermDiagram / termCollapseHomOfForwardCollapse.
        partition.refine(|row| {
            let evals = atomic_pp_type(model, row, POSITIVE_TERM_DEPTH);
            positive_term_diagram_key(&evals)
        });
        "positive_split_term_collapse"
    } else {
        partition.refine(positive_eq_key);
        "positive_split"
    };
    Ok(EngineOutcome::basic(
        partition.is_target_pure(target),
        "qf_pos",
        engine,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::first_order::relops::{Operation, Relation};
    use std::collections::HashMap;

    #[test]
    fn positive_diag_no_ops() {
        let universe = vec![0, 1, 2];
        let target = Relation::new("T", 2).with_tuples(vec![
            vec![0, 0],
            vec![1, 1],
            vec![2, 2],
        ]);
        let model = Model::new(universe, HashMap::new(), HashMap::new());
        let out = check_positive_split(&model, &target).expect("pos");
        assert!(out.definable);
        assert_eq!(out.engine, "positive_split");
    }

    #[test]
    fn positive_term_collapse_engine_label_with_ops() {
        let universe = vec![0, 1];
        let mut ops = HashMap::new();
        let mut f = Operation::new("f", 1);
        f.add(vec![0, 0]);
        f.add(vec![1, 1]);
        ops.insert("f".into(), f);
        let target = Relation::new("T", 2).with_tuples(vec![vec![0, 0], vec![1, 1]]);
        let model = Model::new(universe, HashMap::new(), ops);
        let out = check_positive_split(&model, &target).expect("pos");
        assert!(out.definable);
        assert_eq!(out.engine, "positive_split_term_collapse");
    }
}
