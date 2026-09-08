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
    let definable = partition.is_target_pure(target);
    let mut out = EngineOutcome::basic(definable, fragment, engine);
    if definable {
        // A1/N5a: canonical (lex-min) positive End-orbit representatives.
        let mut reps: Vec<String> = Vec::new();
        for block in &partition.blocks {
            if block.is_empty() {
                continue;
            }
            let rep = block.iter().min().expect("non-empty block");
            if crate::engines::partition::target_accepts_row(target, rep) {
                let cells: Vec<String> = rep.iter().map(|x| x.to_string()).collect();
                reps.push(format!("[{}]", cells.join(",")));
            }
        }
        reps.sort();
        out.witness_sketch = Some(format!("{{\"positive_orbit_reps\":[{}]}}", reps.join(",")));
    }
    Ok(out)
}

pub fn check_morph_split(model: &Model, target: &Relation) -> Result<EngineOutcome, String> {
    split_by_orbits(model, target, false, "ep", "morph_split")
}

pub fn check_ep_cert(model: &Model, target: &Relation) -> Result<EngineOutcome, String> {
    let mut out = split_by_orbits(model, target, false, "ep", "cert")?;
    out.engine = "cert".into();
    Ok(out)
}

pub fn check_embedding_split(model: &Model, target: &Relation) -> Result<EngineOutcome, String> {
    split_by_orbits(model, target, true, "ex", "embedding_split")
}

pub fn check_qf_merge(model: &Model, target: &Relation) -> Result<EngineOutcome, String> {
    split_by_orbits(model, target, true, "qf", "iso_merge")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::dispatch::{check_engine, EngineKind, FragmentKind};
    use crate::parser::parse_model;
    use std::path::Path;

    #[test]
    fn retrombo_nodef_sinpura_morph_and_hom_type_not_definable() {
        let path = Path::new("../fopy/tests/fixtures/models/retrombo_nodef_sinpura.model");
        let model = parse_model(Some(path), true).expect("parse");
        let target = model
            .relations
            .values()
            .find(|r| r.sym.starts_with('T'))
            .expect("T")
            .clone();
        let morph = check_morph_split(&model, &target).unwrap();
        assert!(!morph.definable, "morph_split must reject mixed End-orbit");
        let hom = check_engine(
            &model,
            &target,
            FragmentKind::Ep,
            EngineKind::HomType,
            2,
            1,
        )
        .unwrap();
        assert!(!hom.definable, "hom_type must align with morph_split");
        assert_eq!(hom.engine, "hom_type");
    }

    #[test]
    fn retrombo_nodef_morph_and_hom_type_not_definable() {
        let path = Path::new("../fopy/tests/fixtures/models/retrombo_nodef.model");
        let model = parse_model(Some(path), true).expect("parse");
        let target = model
            .relations
            .values()
            .find(|r| r.sym.starts_with('T'))
            .expect("T")
            .clone();
        assert!(!check_morph_split(&model, &target).unwrap().definable);
        let hom = check_engine(
            &model,
            &target,
            FragmentKind::Ep,
            EngineKind::HomType,
            2,
            1,
        )
        .unwrap();
        assert!(!hom.definable);
    }

    #[test]
    fn retrombo_nodef_ktypes_hybrid_via_hom_type_not_definable() {
        let path = Path::new("../fopy/tests/fixtures/models/retrombo_nodef.model");
        let model = parse_model(Some(path), true).expect("parse");
        let target = model
            .relations
            .values()
            .find(|r| r.sym.starts_with('T'))
            .expect("T")
            .clone();
        assert!(!model.operations.is_empty());
        assert!(model.universe.len() <= 6);
        let kt = check_engine(
            &model,
            &target,
            FragmentKind::Ep,
            EngineKind::Ktypes,
            2,
            1,
        )
        .unwrap();
        assert!(!kt.definable, "M1: ktypes must not silently over-accept");
        assert_eq!(kt.engine, "ktypes_via_hom_type");
    }

    #[test]
    fn ep_cert_emits_sketch_on_diagonal() {
        use crate::first_order::relops::Relation;
        use std::collections::HashMap;
        let universe = vec![0, 1, 2];
        let target = Relation::new("T", 2).with_tuples(vec![
            vec![0, 0],
            vec![1, 1],
            vec![2, 2],
        ]);
        let model = crate::first_order::models::Model::new(
            universe,
            HashMap::new(),
            HashMap::new(),
        );
        let cert = check_ep_cert(&model, &target).unwrap();
        assert!(cert.definable, "diagonal should be End-orbit pure");
        assert_eq!(cert.engine, "cert");
        let sketch = cert.witness_sketch.expect("A1 sketch");
        assert!(sketch.contains("positive_orbit_reps"));
    }

    #[test]
    fn ep_cert_rejects_retrombo_nodef() {
        let path = Path::new("../fopy/tests/fixtures/models/retrombo_nodef.model");
        let model = parse_model(Some(path), true).expect("parse");
        let target = model
            .relations
            .values()
            .find(|r| r.sym.starts_with('T'))
            .expect("T")
            .clone();
        let cert = check_engine(
            &model,
            &target,
            FragmentKind::Ep,
            EngineKind::Cert,
            2,
            1,
        )
        .unwrap();
        assert!(!cert.definable);
        assert_eq!(cert.engine, "cert");
        assert!(cert.witness_sketch.is_none());
    }
}
