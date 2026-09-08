//! Guarded-fragment merge: neighbour orbits, or A2 term+End hybrid with ops.

use crate::engines::dispatch::EngineOutcome;
use crate::engines::morphisms::{morphisms, HomArrow};
use crate::engines::partition::{effective_target_arity, TuplePartition};
use crate::engines::types::{atomic_pp_type, guarded_neighbour_orbit, FO_PP_DEPTH_CAP};
use crate::first_order::models::Model;
use crate::first_order::relops::Relation;
use rayon::prelude::*;
use std::collections::{BTreeSet, HashSet};
use std::sync::Mutex;

fn end_orbit_key(row: &[i64], arrows: &[HomArrow]) -> BTreeSet<Vec<i64>> {
    let mut orbit: HashSet<Vec<i64>> = HashSet::new();
    orbit.insert(row.to_vec());
    let mut changed = true;
    while changed {
        changed = false;
        let snapshot: Vec<_> = orbit.iter().cloned().collect();
        for tup in snapshot {
            for arrow in arrows {
                if let Some(img) = arrow.vector_call(&tup) {
                    if orbit.insert(img) {
                        changed = true;
                    }
                }
            }
        }
    }
    orbit.into_iter().collect()
}

fn full_universe_endos(model: &Model) -> Vec<HomArrow> {
    let full_u: HashSet<i64> = model.universe.iter().copied().collect();
    morphisms(model, false)
        .into_iter()
        .filter(|a| a.domain == full_u)
        .collect()
}

pub fn check_guarded_merge(model: &Model, target: &Relation) -> Result<EngineOutcome, String> {
    if model.universe.len() > 6 {
        return Err("guarded merge requires |U| <= 6".into());
    }
    let tuple_arity = effective_target_arity(target);
    let mut partition = TuplePartition::from_model(model, tuple_arity)?;

    if !model.operations.is_empty() {
        // A2: term depth (atomic PP) + End(U) orbit hybrid.
        let term_depth = tuple_arity.max(FO_PP_DEPTH_CAP);
        let endos = full_universe_endos(model);
        let all: Vec<Vec<i64>> = partition.blocks[0].clone();
        let keyed: Vec<(Vec<i64>, (Vec<i64>, BTreeSet<Vec<i64>>))> = all
            .into_par_iter()
            .map(|row| {
                let pp = atomic_pp_type(model, &row, term_depth);
                let end = end_orbit_key(&row, &endos);
                (row, (pp, end))
            })
            .collect();
        let cache: Mutex<std::collections::HashMap<Vec<i64>, (Vec<i64>, BTreeSet<Vec<i64>>)>> =
            Mutex::new(std::collections::HashMap::new());
        {
            let mut g = cache.lock().unwrap();
            for (row, key) in &keyed {
                g.insert(row.clone(), key.clone());
            }
        }
        partition.refine(|row| {
            cache.lock().unwrap().get(row).cloned().unwrap_or_else(|| {
                (
                    atomic_pp_type(model, row, term_depth),
                    end_orbit_key(row, &endos),
                )
            })
        });
        return Ok(EngineOutcome::basic(
            partition.is_target_pure(target),
            "gf",
            "guarded_merge_term_end",
        ));
    }

    let guard_depth = tuple_arity.max(FO_PP_DEPTH_CAP);
    let cache: Mutex<std::collections::HashMap<Vec<i64>, BTreeSet<Vec<i64>>>> =
        Mutex::new(std::collections::HashMap::new());
    let all: Vec<Vec<i64>> = partition.blocks[0].clone();
    let keyed: Vec<(Vec<i64>, BTreeSet<Vec<i64>>)> = all
        .into_par_iter()
        .map(|row| {
            let key = guarded_neighbour_orbit(model, &row, guard_depth);
            (row, key)
        })
        .collect();
    {
        let mut g = cache.lock().unwrap();
        for (row, key) in &keyed {
            g.insert(row.clone(), key.clone());
        }
    }
    partition.refine(|row| {
        cache
            .lock()
            .unwrap()
            .get(row)
            .cloned()
            .unwrap_or_else(|| guarded_neighbour_orbit(model, row, guard_depth))
    });
    Ok(EngineOutcome::basic(
        partition.is_target_pure(target),
        "gf",
        "guarded_merge",
    ))
}

#[cfg(test)]
mod tests {
    use super::check_guarded_merge;
    use crate::engines::types::{guarded_neighbour_orbit, is_guarded_neighbour};
    use crate::first_order::models::Model;
    use crate::first_order::relops::Relation;
    use std::collections::HashMap;

    #[test]
    fn guarded_neighbour_sees_relation_guard() {
        let mut relations = HashMap::new();
        let mut leq = Relation::new("leq", 2);
        for t in [(0, 0), (0, 1), (0, 2), (0, 3), (1, 1), (1, 3), (2, 2), (2, 3), (3, 3)]
        {
            leq.add(vec![t.0, t.1]);
        }
        relations.insert("leq".into(), leq);
        let model = Model::new(vec![0, 1, 2, 3], relations, HashMap::new());
        let row = vec![0, 0];
        assert!(is_guarded_neighbour(&model, &row, 1, 1, 2));
        let orbit = guarded_neighbour_orbit(&model, &row, 2);
        assert!(orbit.len() > 1, "expected nontrivial orbit, got {}", orbit.len());
    }

    #[test]
    fn guarded_merge_diag_term_end_with_ops() {
        // Lattice with meet/join: A2 hybrid approximates OrbitPureGuarded;
        // diagonal is equality-definable (should accept under term+End).
        use crate::first_order::relops::Operation;
        let mut relations = HashMap::new();
        let mut leq = Relation::new("leq", 2);
        for t in [(0, 0), (0, 1), (0, 2), (0, 3), (1, 1), (1, 3), (2, 2), (2, 3), (3, 3)]
        {
            leq.add(vec![t.0, t.1]);
        }
        relations.insert("leq".into(), leq);
        let mut operations = HashMap::new();
        // Identity-like unary so the model has ops (triggers hybrid).
        let mut id = Operation::new("id", 1);
        for x in 0..4 {
            id.add(vec![x, x]);
        }
        operations.insert("id".into(), id);
        let mut target = Relation::new("T_diag", 2);
        for x in 0..4 {
            target.add(vec![x, x]);
        }
        let model = Model::new(vec![0, 1, 2, 3], relations, operations);
        let out = check_guarded_merge(&model, &target).expect("merge");
        assert_eq!(out.engine, "guarded_merge_term_end");
        assert!(
            out.definable,
            "diagonal should be OrbitPureGuarded under term+End hybrid"
        );
    }

    #[test]
    fn guarded_merge_empty_ops_uses_neighbour_orbit() {
        // No operations -> neighbour-orbit engine (not term+End hybrid).
        let model = Model::new(vec![0, 1], HashMap::new(), HashMap::new());
        let mut target = Relation::new("T", 1);
        target.add(vec![0]);
        target.add(vec![1]);
        let out = check_guarded_merge(&model, &target).expect("merge");
        assert_eq!(out.engine, "guarded_merge");
        assert!(out.definable, "full unary target is orbit-pure");
    }

    #[test]
    fn compound_term_eq_is_not_a_guard() {
        // With only ops (no RelMap), x op x = x must not guard arbitrary moves.
        use crate::first_order::relops::Operation;
        let mut operations = HashMap::new();
        let mut meet = Operation::new("meet", 2);
        for a in 0..3 {
            for b in 0..3 {
                meet.add(vec![a, b, a.min(b)]);
            }
        }
        operations.insert("meet".into(), meet);
        let model = Model::new(vec![0, 1, 2], HashMap::new(), operations);
        let row = vec![0, 1];
        // neighbour (2,1): no coord eq, no RelMap -> not guarded
        assert!(
            !is_guarded_neighbour(&model, &row, 0, 2, 2),
            "compound meet-eq must not guard (0,1)->0:=2"
        );
    }
}
