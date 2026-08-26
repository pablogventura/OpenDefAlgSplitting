//! Morph-orbit / embedding / IsoType merge splits.

use crate::engines::dispatch::EngineOutcome;
use crate::engines::morphisms::{morphisms, HomArrow};
use crate::engines::partition::TuplePartition;
use crate::first_order::models::Model;
use crate::first_order::relops::Relation;
use rayon::prelude::*;
use std::collections::{BTreeSet, HashSet};
use std::sync::Mutex;

fn orbit_key(row: &[i64], arrows: &[HomArrow]) -> BTreeSet<Vec<i64>> {
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

fn split_by_orbits(
    model: &Model,
    target: &Relation,
    open_only: bool,
    fragment: &str,
    engine: &str,
) -> Result<EngineOutcome, String> {
    if model.universe.len() > 6 {
        return Err(format!("{engine} requires |U| <= 6"));
    }
    let arrows_all = morphisms(model, open_only);
    // Parity with fopy: orbit keys only under endomorphisms of the full U
    // (maps with proper subdomain falsely merge orbits / inflate purity).
    let full_u: HashSet<i64> = model.universe.iter().copied().collect();
    let arrows: Vec<HomArrow> = arrows_all
        .into_iter()
        .filter(|a| a.domain == full_u)
        .collect();
    let mut partition = TuplePartition::from_model(model, target.arity)?;
    let cache: Mutex<std::collections::HashMap<Vec<i64>, BTreeSet<Vec<i64>>>> =
        Mutex::new(std::collections::HashMap::new());
    // Precompute keys for all tuples in parallel once
    let all: Vec<Vec<i64>> = partition.blocks[0].clone();
    let keyed: Vec<(Vec<i64>, BTreeSet<Vec<i64>>)> = all
        .into_par_iter()
        .map(|row| {
            let key = orbit_key(&row, &arrows);
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
            .unwrap_or_else(|| orbit_key(row, &arrows))
    });
    Ok(EngineOutcome {
        definable: partition.is_target_pure(target),
        fragment: fragment.into(),
        engine: engine.into(),
    })
}

pub fn check_morph_split(model: &Model, target: &Relation) -> Result<EngineOutcome, String> {
    split_by_orbits(model, target, false, "ep", "morph_split")
}

pub fn check_embedding_split(model: &Model, target: &Relation) -> Result<EngineOutcome, String> {
    split_by_orbits(model, target, true, "ex", "embedding_split")
}

pub fn check_qf_merge(model: &Model, target: &Relation) -> Result<EngineOutcome, String> {
    split_by_orbits(model, target, true, "qf", "iso_merge")
}
