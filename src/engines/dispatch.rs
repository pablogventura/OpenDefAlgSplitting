//! Engine dispatch types and fragment routing.

use crate::engines::horn::check_horn;
use crate::engines::morph::{check_embedding_split, check_morph_split, check_qf_merge};
use crate::engines::partition::TuplePartition;
use crate::engines::positive::check_positive_split;
use crate::engines::types::{atomic_pp_type, clear_type_caches, fo_type, type_explosion_risk};
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
}

#[derive(Debug, Clone)]
pub struct EngineOutcome {
    pub definable: bool,
    pub fragment: String,
    pub engine: String,
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
    Ok(EngineOutcome {
        definable: partition.is_target_pure(target),
        fragment: fragment.into(),
        engine: engine.into(),
    })
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
                let d = max_depth;
                type_split(model, target, "ep", &format!("ep_ktypes_d{d}"), |row| {
                    let sig = atomic_pp_type(model, row, d);
                    sig.iter().flat_map(|x| x.to_le_bytes()).collect()
                })
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
        FragmentKind::Gf => {
            // Ternary ops + arity>=3: stay at k=0 (PP core) to avoid Gaifman blow-up.
            let k = if type_explosion_risk(model, target.arity) {
                0
            } else {
                max_k.max(1)
            };
            type_split(model, target, "gf", &format!("guarded_split_k{k}"), |row| {
                fo_type(model, row, k, target.arity)
            })
        }
        FragmentKind::Fo => {
            let k = if type_explosion_risk(model, target.arity) {
                0
            } else {
                max_k.max(1)
            };
            type_split(model, target, "fo", &format!("fo_split_k{k}"), |row| {
                fo_type(model, row, k, target.arity)
            })
        }
        FragmentKind::Horn => check_horn(model, target, max_depth),
        FragmentKind::Qf => match engine {
            EngineKind::Merge => check_qf_merge(model, target),
            EngineKind::Hit | EngineKind::Split | EngineKind::Auto => {
                match is_open_def(model, vec![target.clone()], HitConfig::default()) {
                    Ok(_) => Ok(EngineOutcome {
                        definable: true,
                        fragment: "qf".into(),
                        engine: "hit_split".into(),
                    }),
                    Err(_) => Ok(EngineOutcome {
                        definable: false,
                        fragment: "qf".into(),
                        engine: "hit_split".into(),
                    }),
                }
            }
            EngineKind::Ktypes => Err("qf has no ktypes engine".into()),
        },
    }
}
