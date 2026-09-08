//! Horn hybrid: PP purity first, then budgeted BFS over equality-Horn clauses.

use crate::engines::dispatch::EngineOutcome;
use crate::engines::partition::TuplePartition;
use crate::engines::types::atomic_pp_type;
use crate::first_order::formulas::{self, Formula, Term};
use crate::first_order::models::Model;
use crate::first_order::relops::Relation;
use std::collections::{HashSet, VecDeque};

/// Soft caps to avoid OOM on tiporet / small algebras.
const HORN_MAX_ARITY: usize = 4;
const HORN_MAX_UNIVERSE: usize = 8;
const HORN_MAX_CANDIDATES: usize = 48;
const HORN_MAX_BFS_COMBOS: usize = 128;
const HORN_MAX_CLAUSES: usize = 2;
const HORN_TERM_DEPTH_CAP: usize = 1;

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

/// Enumerate shallow terms (vars + one layer of ops) for Horn atoms.
fn shallow_terms(model: &Model, arity: usize, max_depth: usize) -> Vec<Term> {
    let mut terms = var_terms(arity);
    if max_depth == 0 || model.operations.is_empty() {
        return terms;
    }
    let base = terms.clone();
    let mut op_syms: Vec<_> = model.operations.iter().collect();
    op_syms.sort_by_key(|(sym, op)| (op.arity, (*sym).clone()));
    for (sym, op) in op_syms {
        if op.arity == 0 {
            terms.push(Term::OpTerm {
                sym: formulas::OpSym::new(sym.clone(), 0),
                args: vec![],
            });
            continue;
        }
        if op.arity > 2 || base.len().saturating_pow(op.arity as u32) > 64 {
            continue;
        }
        // Cartesian product of base terms for op args.
        let mut arg_tuples: Vec<Vec<Term>> = vec![vec![]];
        for _ in 0..op.arity {
            let mut next = Vec::new();
            for prefix in &arg_tuples {
                for t in &base {
                    let mut row = prefix.clone();
                    row.push(t.clone());
                    next.push(row);
                }
            }
            arg_tuples = next;
            if arg_tuples.len() > 64 {
                break;
            }
        }
        for args in arg_tuples {
            if args.len() == op.arity {
                terms.push(Term::OpTerm {
                    sym: formulas::OpSym::new(sym.clone(), op.arity),
                    args,
                });
            }
        }
        if terms.len() > 32 {
            break;
        }
    }
    terms
}

fn equality_atoms(terms: &[Term]) -> Vec<Formula> {
    let mut atoms = Vec::new();
    for i in 0..terms.len() {
        for j in i..terms.len() {
            atoms.push(formulas::eq(terms[i].clone(), terms[j].clone()));
        }
    }
    atoms
}

/// Unit clauses + single-antecedent implications over equality atoms.
fn horn_clause_seed(atoms: &[Formula], max_atoms: usize) -> Vec<Formula> {
    let mut clauses = Vec::new();
    for cons in atoms {
        clauses.push(cons.clone());
        if max_atoms == 0 {
            continue;
        }
        for ant in atoms {
            if ant == cons {
                continue;
            }
            clauses.push(ant.clone().neg().or_formula(cons));
        }
    }
    clauses
}

/// Budgeted BFS over conjunctions of up to `HORN_MAX_CLAUSES` seed clauses.
fn horn_bfs_find(
    model: &Model,
    target: &Relation,
    seeds: &[Formula],
) -> Option<Formula> {
    let limit = seeds.len().min(HORN_MAX_CANDIDATES);
    let seeds = &seeds[..limit];
    for clause in seeds {
        if formula_matches(model, target, clause) {
            return Some(clause.clone());
        }
    }
    if HORN_MAX_CLAUSES < 2 {
        return None;
    }
    let mut queue: VecDeque<(Formula, usize)> = VecDeque::new();
    for (i, clause) in seeds.iter().enumerate() {
        queue.push_back((clause.clone(), i));
    }
    let mut checked = 0usize;
    while let Some((formula, last_idx)) = queue.pop_front() {
        if checked >= HORN_MAX_BFS_COMBOS {
            break;
        }
        checked += 1;
        for j in (last_idx + 1)..seeds.len() {
            if checked >= HORN_MAX_BFS_COMBOS {
                break;
            }
            let trial = formula.and_formula(&seeds[j]);
            checked += 1;
            if formula_matches(model, target, &trial) {
                return Some(trial);
            }
            // Depth-2 only (pairs); do not grow further under budget.
        }
    }
    None
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
        return Ok(EngineOutcome::basic(
            true,
            "horn",
            format!("horn_pp_d{d}"),
        ));
    }

    // Not PP-pure at depth d: budgeted equality-Horn BFS (var atoms, + shallow
    // terms when ops are small). Aligns tiporet extras without OOM.
    let u = model.universe.len();
    let ar = target.arity;
    if ar <= HORN_MAX_ARITY && u <= HORN_MAX_UNIVERSE && u.saturating_pow(ar as u32) <= 4096 {
        let term_depth = if model.operations.is_empty() || u > 4 || ar > 3 {
            0
        } else {
            HORN_TERM_DEPTH_CAP.min(d)
        };
        let terms = shallow_terms(model, ar, term_depth);
        let atoms = equality_atoms(&terms);
        let seeds = horn_clause_seed(&atoms, 1);
        if let Some(_wit) = horn_bfs_find(model, target, &seeds) {
            return Ok(EngineOutcome::basic(
                true,
                "horn",
                format!("horn_clauses_bfs_d{d}"),
            ));
        }
    }

    Ok(EngineOutcome::basic(
        false,
        "horn",
        format!("horn_pp_d{d}"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::first_order::relops::Relation;
    use std::collections::HashMap;

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

    #[test]
    fn horn_implication_xeq_y_implies_xeq_z() {
        // T = { (x,y,z) | x≠y ∨ x=z } — PP-impure at var patterns? Actually pure.
        // Use a target that needs an implication witness when not caught as PP:
        // {(0,0),(1,1)} diagonal already PP. Here: full binary minus (0,1) via
        // clause search on small U — still PP-pure by pattern. Smoke the BFS path
        // stays within budget on arity-3.
        let universe = vec![0, 1];
        let mut rows = Vec::new();
        for x in &universe {
            for y in &universe {
                for z in &universe {
                    if x != y || x == z {
                        rows.push(vec![*x, *y, *z]);
                    }
                }
            }
        }
        let target = Relation::new("T", 3).with_tuples(rows);
        let model = Model::new(universe, HashMap::new(), HashMap::new());
        let out = check_horn(&model, &target, 1).expect("horn");
        assert!(out.definable);
    }
}
