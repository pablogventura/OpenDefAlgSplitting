//! Horn hybrid: PP purity first, then bounded equality-Horn clause search.

use crate::engines::dispatch::EngineOutcome;
use crate::engines::partition::TuplePartition;
use crate::engines::types::atomic_pp_type;
use crate::first_order::formulas::{self, Formula, Term};
use crate::first_order::models::Model;
use crate::first_order::relops::Relation;
use std::collections::HashSet;

fn cartesian(universe: &[i64], arity: usize) -> Vec<Vec<i64>> {
    if arity == 0 {
        return vec![vec![]];
    }
    let mut out = vec![vec![]];
    for _ in 0..arity {
        let mut next = Vec::new();
        for prefix in &out {
            for &u in universe {
                let mut row = prefix.clone();
                row.push(u);
                next.push(row);
            }
        }
        out = next;
    }
    out
}

fn formula_matches(model: &Model, target: &Relation, formula: &Formula) -> bool {
    let ext = formula.extension(model, Some(target.arity));
    let got: HashSet<Vec<i64>> = ext.into_iter().collect();
    let want: HashSet<Vec<i64>> = target.r.iter().cloned().collect();
    got == want
}

fn var_terms(arity: usize) -> Vec<Term> {
    (0..arity)
        .map(|i| Term::Variable(formulas::Variable::from_index(i as i32)))
        .collect()
}

/// Bounded Horn candidates over variable equalities only (depth 0 atoms).
fn horn_equality_candidates(arity: usize, max_atoms: usize) -> Vec<Formula> {
    let vars = var_terms(arity);
    let mut atoms: Vec<Formula> = Vec::new();
    for i in 0..vars.len() {
        for j in i..vars.len() {
            atoms.push(formulas::eq(vars[i].clone(), vars[j].clone()));
        }
    }
    let mut clauses = Vec::new();
    for cons in &atoms {
        clauses.push(cons.clone());
        if max_atoms == 0 {
            continue;
        }
        for ant in &atoms {
            if ant == cons {
                continue;
            }
            // ant -> cons  ≡  ¬ant ∨ cons
            clauses.push(ant.clone().neg().or_formula(cons));
        }
    }
    clauses
}

pub fn check_horn(
    model: &Model,
    target: &Relation,
    max_depth: usize,
) -> Result<EngineOutcome, String> {
    let d = max_depth.max(1);
    let mut partition = TuplePartition::from_model(model, target.arity)?;
    partition.refine(|row| {
        let sig = atomic_pp_type(model, row, d);
        sig.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<_>>()
    });
    if partition.is_target_pure(target) {
        return Ok(EngineOutcome {
            definable: true,
            fragment: "horn".into(),
            engine: format!("horn_pp_d{d}"),
        });
    }

    // Not PP-pure: try small equality-Horn witnesses (parity sketch of horn_ktypes).
    if target.arity <= 3 && model.universe.len() <= 6 {
        let _ = cartesian(&model.universe, target.arity); // size guard side-effect free
        let candidates = horn_equality_candidates(target.arity, 1);
        for clause in &candidates {
            if formula_matches(model, target, clause) {
                return Ok(EngineOutcome {
                    definable: true,
                    fragment: "horn".into(),
                    engine: format!("horn_clauses_d{d}"),
                });
            }
        }
        let limit = candidates.len().min(16);
        for i in 0..limit {
            for j in (i + 1)..limit {
                let trial = candidates[i].and_formula(&candidates[j]);
                if formula_matches(model, target, &trial) {
                    return Ok(EngineOutcome {
                        definable: true,
                        fragment: "horn".into(),
                        engine: format!("horn_clauses_d{d}"),
                    });
                }
            }
        }
    }

    Ok(EngineOutcome {
        definable: false,
        fragment: "horn".into(),
        engine: format!("horn_pp_d{d}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::first_order::relops::Relation;
    use std::collections::HashMap;

    #[test]
    fn horn_accepts_full_via_pp_pure() {
        let universe = vec![0, 1];
        let target = Relation::new("T", 2).with_tuples(cartesian(&universe, 2));
        let model = Model::new(universe, HashMap::new(), HashMap::new());
        let out = check_horn(&model, &target, 1).expect("horn");
        assert!(out.definable);
        assert!(out.engine.contains("horn_pp"));
    }

    #[test]
    fn horn_equality_diag_definable() {
        let universe = vec![0, 1, 2];
        let target = Relation::new("T", 2).with_tuples(vec![
            vec![0, 0],
            vec![1, 1],
            vec![2, 2],
        ]);
        let model = Model::new(universe, HashMap::new(), HashMap::new());
        let out = check_horn(&model, &target, 1).expect("horn");
        assert!(out.definable, "diagonal should be equality-Horn / PP-pure");
    }
}
