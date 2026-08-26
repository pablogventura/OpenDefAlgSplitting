//! Selector automático de flags HIT según features baratas del modelo/target.
//!
//! Política (criterio velocidad, mismo veredicto):
//! - `skip_useless` siempre ON en modo auto; simplify OFF (velocidad)
//! - rechazo por mezcla de patrón de igualdad si no hay ops de aridad > 0 (sound)
//! - `approx_precheck` OFF en auto (sin predictor de early-exit; usar flag manual)

use std::collections::HashMap;

use crate::first_order::models::Model;
use crate::first_order::relops::Relation;
use crate::hit::{ExploreOrder, HitConfig};
use crate::preprocessing::Pattern;
use crate::unary::{decide_unary, UnaryDecision};

/// Tope histórico de tuplas dominio^aridad (documentación / flags manuales).
/// En modo auto no se enciende approx: si no hay early-exit el costo supera a HIT+skip.
pub const APPROX_MAX_DOMAIN_TUPLES: u64 = 50_000;
/// Tope histórico de operaciones para approx manual.
pub const APPROX_MAX_OPS: usize = 8;
/// Tope de tuplas para el chequeo de mezcla de patrones.
pub const PATTERN_CHECK_MAX_TUPLES: u64 = 200_000;

#[derive(Clone, Debug)]
pub struct ModelFeatures {
    pub universe_size: usize,
    pub target_arity: usize,
    pub target_size: usize,
    pub ops_total: usize,
    pub ops_arity_gt0: usize,
    pub domain_tuples: u64,
    /// Mezcla de patrón de igualdad (si el chequeo cupo en presupuesto).
    pub pattern_mix: Option<bool>,
}

#[derive(Clone, Debug)]
pub enum StrategyDecision {
    /// Rechazo sound: T mezcla un patrón de igualdad y no hay ops de aridad > 0.
    RejectPattern { reason: String },
    /// Definible por backend unario (sin correr HIT).
    AcceptUnary { reason: String },
    /// No definible por backend unario (sin correr HIT).
    RejectUnary { reason: String },
    /// Correr HIT con esta config.
    Run {
        config: HitConfig,
        reasons: Vec<String>,
    },
}

fn saturating_pow(base: u64, exp: usize) -> u64 {
    let mut acc = 1u64;
    for _ in 0..exp {
        acc = acc.saturating_mul(base);
    }
    acc
}

fn cartesian_product(universe: &[i64], k: usize) -> Vec<Vec<i64>> {
    if k == 0 {
        return vec![vec![]];
    }
    let mut out = vec![vec![]];
    for _ in 0..k {
        let mut next = Vec::with_capacity(out.len().saturating_mul(universe.len()));
        for prefix in out {
            for &u in universe {
                let mut t = prefix.clone();
                t.push(u);
                next.push(t);
            }
        }
        out = next;
    }
    out
}

/// True si algún patrón de igualdad tiene realizaciones en T y fuera de T.
pub fn pattern_mixes_target(model: &Model, target: &Relation) -> bool {
    let k = target.arity;
    if k == 0 {
        return false;
    }
    let mut by_pat: HashMap<String, (bool, bool)> = HashMap::new();
    for tuple in cartesian_product(&model.universe, k) {
        let in_t = target.contains(&tuple);
        let key = Pattern::new(tuple).name();
        let entry = by_pat.entry(key).or_insert((false, false));
        if in_t {
            entry.0 = true;
        } else {
            entry.1 = true;
        }
        if entry.0 && entry.1 {
            return true;
        }
    }
    false
}

pub fn extract_features(model: &Model, target: &Relation) -> ModelFeatures {
    let universe_size = model.universe.len();
    let target_arity = target.arity;
    let domain_tuples = saturating_pow(universe_size as u64, target_arity);
    let ops_total = model.operations.len();
    let ops_arity_gt0 = model.operations.values().filter(|op| op.arity > 0).count();

    // Solo hace falta para rechazo sound sin ops; con ops no decide approx en auto.
    let pattern_mix = if ops_arity_gt0 == 0
        && domain_tuples > 0
        && domain_tuples <= PATTERN_CHECK_MAX_TUPLES
    {
        Some(pattern_mixes_target(model, target))
    } else {
        None
    };

    ModelFeatures {
        universe_size,
        target_arity,
        target_size: target.r.len(),
        ops_total,
        ops_arity_gt0,
        domain_tuples,
        pattern_mix,
    }
}

fn base_fast_config() -> HitConfig {
    HitConfig {
        use_information_gain: false,
        ig_sample: Some(20),
        ig_experimental: false,
        skip_useless_candidates: true,
        approx_precheck: false,
        emit_all_constants: true,
        max_steps: None,
        simplify_formula: false,
        explore_order: ExploreOrder::Dfs,
    }
}

/// Elige config (o rechazo por patrón) según features.
pub fn select_strategy(model: &Model, target: &Relation) -> StrategyDecision {
    select_strategy_explained(model, target).0
}

/// Igual que [`select_strategy`], más texto legible de la decisión.
pub fn select_strategy_explained(
    model: &Model,
    target: &Relation,
) -> (StrategyDecision, String) {
    let feat = extract_features(model, target);
    let mut reasons = Vec::new();
    reasons.push(format!(
        "features: |U|={}, arity={}, |T|={}, ops={}, ops_arity>0={}, domain_tuples={}, pattern_mix={:?}",
        feat.universe_size,
        feat.target_arity,
        feat.target_size,
        feat.ops_total,
        feat.ops_arity_gt0,
        feat.domain_tuples,
        feat.pattern_mix
    ));

    if feat.ops_arity_gt0 == 0 {
        if feat.pattern_mix == Some(true) {
            let reason = "pattern_mix sin ops de aridad>0: rechazo sound (Lema 3a)".to_string();
            reasons.push(reason.clone());
            let decision = StrategyDecision::RejectPattern { reason };
            return (decision, reasons.join("\n"));
        }
        if feat.pattern_mix == Some(false) {
            // No ops and every equality pattern is pure: QF-definable by
            // equalities/inequalities alone (converse of Lema 3a).
            // Preprocessed pieces: HIT main ANDs ⊤ with pattern.post.
            let reason =
                "sin ops y sin pattern_mix: acepto equality-definable (Lema 3a conversa)".to_string();
            reasons.push(reason.clone());
            return (
                StrategyDecision::AcceptUnary { reason },
                reasons.join("\n"),
            );
        }
        reasons.push("sin ops de aridad>0 y sin pattern_mix (o check omitido)".into());
    }

    // Backend unario antes de HIT:
    // - NotDefinable: early-exit sound (Lean / Castellano).
    // - Definable sin ops (∅/A): fórmula trivial.
    // - Definable con ops: el conteo afirma definibilidad; HIT sintetiza la fórmula.
    match decide_unary(model, target) {
        UnaryDecision::NotDefinable { reason } => {
            reasons.push(format!("unary backend: {reason}"));
            return (
                StrategyDecision::RejectUnary { reason },
                reasons.join("\n"),
            );
        }
        UnaryDecision::Definable { reason } => {
            reasons.push(format!("unary backend: {reason}"));
            let ops_gt0 = model.operations.values().any(|op| op.arity > 0);
            if !ops_gt0 {
                return (
                    StrategyDecision::AcceptUnary { reason },
                    reasons.join("\n"),
                );
            }
            reasons.push(
                "unary afirma DEFINABLE; se corre HIT+skip para sintetizar fórmula".into(),
            );
        }
        UnaryDecision::Skip => {
            reasons.push("unary backend: skip (arity!=1 o ops aridad>1)".into());
        }
    }

    let config = base_fast_config();
    reasons.push("skip_useless=true (Lema 1)".into());
    reasons.push("simplify_formula=false (auto prioriza velocidad; usar --simplify)".into());
    reasons.push(format!(
        "approx_precheck=false en auto (manual si dominio<={} y ops<={})",
        APPROX_MAX_DOMAIN_TUPLES, APPROX_MAX_OPS
    ));

    let explanation = reasons.join("\n");
    (
        StrategyDecision::Run { config, reasons },
        explanation,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::first_order::relops::Operation;
    use std::collections::HashMap;

    fn model_no_ops_pattern_mix() -> (Model, Relation) {
        // U={0,1}, T={(0,0)}: patrón |0|1|a2 no mezclado; patrón |0,1|a1:
        // (0,0) in T, (1,1) not in T -> mix del patrón diagonal.
        let universe = vec![0, 1];
        let target = Relation::new("T", 2).with_tuples(vec![vec![0, 0]]);
        let model = Model::new(universe, HashMap::new(), HashMap::new());
        (model, target)
    }

    fn model_with_op_arity2() -> (Model, Relation) {
        // Aridad 2: el backend unario hace Skip y el selector cae en HIT+skip.
        let universe = vec![0, 1];
        let mut ops = HashMap::new();
        let mut f = Operation::new("f", 1);
        f.add(vec![0, 0]);
        f.add(vec![1, 1]);
        ops.insert("f".into(), f);
        let target = Relation::new("T", 2).with_tuples(vec![vec![0, 0]]);
        let model = Model::new(universe, HashMap::new(), ops);
        (model, target)
    }

    #[test]
    fn pattern_mix_detects_diagonal() {
        let (model, target) = model_no_ops_pattern_mix();
        assert!(pattern_mixes_target(&model, &target));
    }

    #[test]
    fn select_rejects_pattern_without_ops() {
        let (model, target) = model_no_ops_pattern_mix();
        match select_strategy(&model, &target) {
            StrategyDecision::RejectPattern { .. } => {}
            other => panic!("esperaba RejectPattern, got {:?}", other),
        }
    }

    #[test]
    fn select_always_enables_skip() {
        let (model, target) = model_with_op_arity2();
        match select_strategy(&model, &target) {
            StrategyDecision::Run { config, .. } => {
                assert!(config.skip_useless_candidates);
                assert!(!config.simplify_formula);
            }
            other => panic!("esperaba Run con skip, got {:?}", other),
        }
    }

    #[test]
    fn default_hit_config_has_skip() {
        assert!(HitConfig::default().skip_useless_candidates);
    }
}
