//! Engine dispatch types and fragment routing.

use crate::engines::guarded::check_guarded_merge;
use crate::engines::horn::check_horn;
use crate::engines::megahit::check_qf_megahit_merge;
use crate::engines::morph::{
    check_embedding_split, check_ep_cert, check_morph_split,
};
use crate::engines::partition::{
    enumerate_tuples, target_accepts_row, TuplePartition,
};
use crate::engines::positive::check_positive_split;
use crate::engines::types::{
    atomic_pp_type, choose_fo_split_k, clear_type_caches, ep_type, fo_type, guarded_type,
    type_explosion_risk,
};
use crate::first_order::models::Model;
use crate::first_order::relops::Relation;
use crate::hit::{is_open_def, HitConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentKind {
    Qf,
    QfPos,
    Ep,
    Ex,
    Pp,
    AtomicConj,
    Fo,
    Horn,
    Gf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    Auto,
    Split,
    Merge,
    Ktypes,
    Hit,
    HomType,
    Cert,
}

#[derive(Debug, Clone)]
pub struct EngineOutcome {
    pub definable: bool,
    pub fragment: String,
    pub engine: String,
    /// Optional End-orbit / formula sketch (A1: positive orbit reps as JSON).
    pub witness_sketch: Option<String>,
}

impl EngineOutcome {
    pub fn basic(definable: bool, fragment: impl Into<String>, engine: impl Into<String>) -> Self {
        Self {
            definable,
            fragment: fragment.into(),
            engine: engine.into(),
            witness_sketch: None,
        }
    }

    pub fn with_sketch(mut self, sketch: impl Into<String>) -> Self {
        self.witness_sketch = Some(sketch.into());
        self
    }
}

impl FragmentKind {
    pub fn parse(s: &str) -> Result<Self, String> {
        let key = s.trim().to_lowercase().replace('-', "_");
        match key.as_str() {
            "qf" | "open" | "quantifier_free" => Ok(Self::Qf),
            "qf_pos" | "qfpos" | "positive" => Ok(Self::QfPos),
            "ep" | "existential_positive" => Ok(Self::Ep),
            "ex" | "existential" => Ok(Self::Ex),
            "pp" => Ok(Self::Pp),
            "atomic_conj" | "atomic" => Ok(Self::AtomicConj),
            "fo" => Ok(Self::Fo),
            "horn" => Ok(Self::Horn),
            "gf" | "guarded" => Ok(Self::Gf),
            other => Err(format!("unknown fragment {other}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Qf => "qf",
            Self::QfPos => "qf_pos",
            Self::Ep => "ep",
            Self::Ex => "ex",
            Self::Pp => "pp",
            Self::AtomicConj => "atomic_conj",
            Self::Fo => "fo",
            Self::Horn => "horn",
            Self::Gf => "gf",
        }
    }
}

impl EngineKind {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "split" => Ok(Self::Split),
            "merge" => Ok(Self::Merge),
            "ktypes" => Ok(Self::Ktypes),
            "hit" => Ok(Self::Hit),
            "hom_type" | "homtype" => Ok(Self::HomType),
            "cert" | "certificate" | "ep_cert" => Ok(Self::Cert),
            other => Err(format!("unknown engine {other}")),
        }
    }
}

fn type_split(
    model: &Model,
    target: &Relation,
    fragment: &str,
    engine: &str,
    key: impl Fn(&[i64]) -> Vec<u8> + Sync,
) -> Result<EngineOutcome, String> {
    let mut partition = TuplePartition::from_model(model, target.arity)?;
    partition.refine(|row| key(row));
    Ok(EngineOutcome::basic(
        partition.is_target_pure(target),
        fragment,
        engine,
    ))
}

/// Positive term-equality pairs that hold on a PP signature (Python type_conjunction).
fn positive_eq_pairs(evals: &[i64]) -> Vec<(usize, usize)> {
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

fn matches_positive_eq_pairs(evals: &[i64], pairs: &[(usize, usize)]) -> bool {
    pairs.iter().all(|&(i, j)| evals.get(i) == evals.get(j))
}

/// Parity with fopy `is_fo_definable` z3/fast crosscheck: after FO-k purity, the
/// equality-only PP witness of positive blocks must define exactly `target`.
/// Unguarded FO neighbours refine types; this witness step is what separates
/// sparse edge targets (fo=no) from guarded split (gf=si).
fn fo_pp_witness_defines_target(
    model: &Model,
    target: &Relation,
    positive_reps: &[Vec<i64>],
    max_depth: usize,
) -> Result<bool, String> {
    let tuples = enumerate_tuples(model, target.arity)?;
    if positive_reps.is_empty() {
        return Ok(tuples
            .iter()
            .all(|row| !target_accepts_row(target, row)));
    }
    let masks: Vec<Vec<(usize, usize)>> = positive_reps
        .iter()
        .map(|rep| positive_eq_pairs(&atomic_pp_type(model, rep, max_depth)))
        .collect();
    for row in &tuples {
        let evals = atomic_pp_type(model, row, max_depth);
        let in_witness = masks
            .iter()
            .any(|pairs| matches_positive_eq_pairs(&evals, pairs));
        let in_target = target_accepts_row(target, row);
        if in_witness != in_target {
            return Ok(false);
        }
    }
    Ok(true)
}

fn fo_type_split(
    model: &Model,
    target: &Relation,
    k: usize,
) -> Result<EngineOutcome, String> {
    let engine = format!("fo_split_k{k}");
    let mut partition = TuplePartition::from_model(model, target.arity)?;
    // FO-k uses *unguarded* neighbours (see `fo_type`); differs from `guarded_type`.
    partition.refine(|row| fo_type(model, row, k, target.arity));
    if !partition.is_target_pure(target) {
        return Ok(EngineOutcome::basic(false, "fo", engine));
    }
    let positive_reps: Vec<Vec<i64>> = partition
        .blocks
        .iter()
        .filter_map(|block| {
            let rep = block.first()?;
            if target_accepts_row(target, rep) {
                Some(rep.clone())
            } else {
                None
            }
        })
        .collect();
    // Match fopy.fo_ktypes: max_depth = max(arity, max_k).
    let witness_depth = target.arity.max(k);
    let definable =
        fo_pp_witness_defines_target(model, target, &positive_reps, witness_depth)?;
    Ok(EngineOutcome::basic(definable, "fo", engine))
}

/// Run a DefLab engine; parity target is `definable` only.
pub fn check_engine(
    model: &Model,
    target: &Relation,
    fragment: FragmentKind,
    engine: EngineKind,
    max_depth: usize,
    max_k: usize,
) -> Result<EngineOutcome, String> {
    clear_type_caches();
    match fragment {
        FragmentKind::QfPos => check_positive_split(model, target),
        FragmentKind::Ep => {
            if engine == EngineKind::Ktypes {
                // M1 hybrid (parity with fopy.is_ep_definable):
                // ops + |U|<=6 -> hom_type (no silent over-accept);
                // ops + |U|>6 -> bounded types marked incomplete;
                // empty ops -> equality-pattern ep_type (complete).
                let d = max_depth;
                let e = 1usize;
                if !model.operations.is_empty() && model.universe.len() <= 6 {
                    let mut out = check_morph_split(model, target)?;
                    out.engine = "ktypes_via_hom_type".into();
                    Ok(out)
                } else if !model.operations.is_empty() {
                    type_split(
                        model,
                        target,
                        "ep",
                        &format!("ep_ktypes_d{d}_e{e}_ops_incomplete"),
                        |row| ep_type(model, row, d, e),
                    )
                } else {
                    type_split(model, target, "ep", &format!("ep_ktypes_d{d}_e{e}"), |row| {
                        ep_type(model, row, d, e)
                    })
                }
            } else if engine == EngineKind::HomType {
                // End(U)-orbit type; parity with fopy is_ep_hom_definable / MorphOrbit.
                let mut out = check_morph_split(model, target)?;
                out.engine = "hom_type".into();
                Ok(out)
            } else if engine == EngineKind::Cert {
                check_ep_cert(model, target)
            } else {
                check_morph_split(model, target)
            }
        }
        FragmentKind::Ex => check_embedding_split(model, target),
        FragmentKind::Pp => {
            let d = max_depth.max(1);
            type_split(model, target, "pp", &format!("pp_split_d{d}"), |row| {
                let sig = atomic_pp_type(model, row, d);
                sig.iter().flat_map(|x| x.to_le_bytes()).collect()
            })
        }
        FragmentKind::AtomicConj => {
            let d = if max_depth == 0 { 1 } else { max_depth };
            type_split(
                model,
                target,
                "atomic_conj",
                &format!("atomic_split_d{d}"),
                |row| {
                    let sig = atomic_pp_type(model, row, d);
                    sig.iter().flat_map(|x| x.to_le_bytes()).collect()
                },
            )
        }
        FragmentKind::Gf => match engine {
            EngineKind::Merge => check_guarded_merge(model, target),
            EngineKind::Split | EngineKind::Auto | EngineKind::Ktypes => {
                let k = if type_explosion_risk(model, target.arity) {
                    0
                } else {
                    max_k.max(1)
                };
                type_split(model, target, "gf", &format!("guarded_split_k{k}"), |row| {
                    guarded_type(model, row, k, target.arity)
                })
            }
            EngineKind::Hit => Err("gf has no hit engine".into()),
            EngineKind::HomType => Err("gf has no hom_type engine".into()),
            EngineKind::Cert => Err("gf has no cert engine".into()),
        }
        FragmentKind::Fo => {
            // M2: adaptive FO — memo + ranked neighbours; never blind k=0.
            let k = choose_fo_split_k(model, target.arity, max_k);
            fo_type_split(model, target, k)
        }
        FragmentKind::Horn => check_horn(model, target, max_depth),
        FragmentKind::Qf => match engine {
            EngineKind::Merge => {
                // Prefer joint purity over all preprocessed T-pieces of this arity.
                check_qf_megahit_merge(model, target)
            }
            EngineKind::Hit | EngineKind::Split | EngineKind::Auto => {
                match is_open_def(model, vec![target.clone()], HitConfig::default()) {
                    Ok(_) => Ok(EngineOutcome::basic(true, "qf", "hit_split")),
                    Err(_) => Ok(EngineOutcome::basic(false, "qf", "hit_split")),
                }
            }
            EngineKind::Ktypes => Err("qf has no ktypes engine".into()),
            EngineKind::HomType => Err("qf has no hom_type engine".into()),
            EngineKind::Cert => Err("qf has no cert engine".into()),
        },
    }
}

#[cfg(test)]
mod fo_gf_tests {
    use super::*;
    use crate::engines::types::{fo_type, guarded_type};
    use crate::parser::parse_model;
    use std::path::Path;

    #[test]
    fn sparse_edge_fo_rejects_gf_accepts() {
        let path = Path::new("../fopy/discovery/corpus/sparse_fo_gf/sparse_u3_e1_01_T_edge.model");
        let model = parse_model(Some(path), true).expect("parse");
        let target = model
            .relations
            .values()
            .find(|r| r.sym.contains("T_edge") || r.sym.starts_with('T'))
            .expect("T_edge")
            .clone();
        let fo = check_engine(&model, &target, FragmentKind::Fo, EngineKind::Split, 2, 1)
            .expect("fo");
        let gf = check_engine(&model, &target, FragmentKind::Gf, EngineKind::Split, 2, 1)
            .expect("gf");
        assert!(!fo.definable, "FO must reject sparse edge (PP witness)");
        assert!(gf.definable, "GF must accept sparse edge");
        let row = [0i64, 1];
        assert_ne!(
            fo_type(&model, &row, 1, 2),
            guarded_type(&model, &row, 1, 2),
            "unguarded FO neighbours must differ from guarded"
        );
    }
}
