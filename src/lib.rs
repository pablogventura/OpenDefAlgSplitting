pub mod first_order;
pub mod parser;
pub mod preprocessing;
pub mod hit;
pub mod approx;
pub mod strategy;
pub mod unary;
pub mod engines;
pub mod stone;

#[cfg(feature = "cuda")]
pub mod hit_cuda;

pub use first_order::{formulas, models, relops};
pub use parser::{parse_model, ParserError};
pub use preprocessing::{preprocesamiento2, Pattern};
pub use hit::{
    is_open_def, Counterexample, ExploreOrder, HitConfig, reset_run_stats, run_stats_snapshot,
    qf_isotype_equality_key,
};
pub use strategy::{
    extract_features, select_strategy, select_strategy_explained, StrategyDecision,
};
pub use unary::{audit_unary_targets, decide_unary, UnaryAuditRow, UnaryDecision};
pub use engines::{check_engine, EngineKind, EngineOutcome, FragmentKind};
pub use stone::{filtering_functions, FilterReport, StoneSpec};

#[cfg(test)]
mod tests {
    //! Tests de definibilidad: modelos definibles y no definibles (mismo criterio que Python).
    use super::*;
    use std::path::Path;

    fn check_model(model_path: &Path) -> Result<bool, hit::Counterexample> {
        check_model_with_config(model_path, HitConfig::default())
    }

    fn check_model_with_config(
        model_path: &Path,
        config: HitConfig,
    ) -> Result<bool, hit::Counterexample> {
        let model = parse_model(Some(model_path), true).map_err(|_| hit::Counterexample(vec![]))?;
        let target_syms: Vec<String> = model
            .relations
            .keys()
            .filter(|s| s.starts_with('T'))
            .cloned()
            .collect();
        if target_syms.is_empty() {
            return Ok(true);
        }
        let mut targets_by_arity: std::collections::HashMap<usize, Vec<_>> =
            std::collections::HashMap::new();
        let mut model = model;
        for sym in &target_syms {
            let rel = model.relations.remove(sym).unwrap();
            targets_by_arity.entry(rel.arity).or_default().push(rel);
        }
        for (_, targets) in targets_by_arity {
            for target in targets {
                match is_open_def(&model, vec![target], config) {
                    Ok(_) => {}
                    Err(ce) => return Err(ce),
                }
            }
        }
        Ok(true)
    }

    #[test]
    fn test_modeloqueanda_definable() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let model_path = project_root.join("model_examples").join("modeloqueanda.model");
        if !model_path.exists() {
            return;
        }
        match check_model(&model_path) {
            Ok(_) => {}
            Err(_) => panic!("modeloqueanda debería ser DEFINABLE"),
        }
    }

    #[test]
    fn test_suma4_not_definable() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let model_path = project_root.join("model_examples").join("suma4.model");
        if !model_path.exists() {
            return;
        }
        match check_model(&model_path) {
            Ok(_) => panic!("suma4 debería ser NOT DEFINABLE"),
            Err(_) => {}
        }
    }

    #[test]
    fn test_retrombo_nodef_not_definable() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let model_path = project_root.join("model_examples").join("retrombo_nodef.model");
        if !model_path.exists() {
            return;
        }
        match check_model(&model_path) {
            Ok(_) => panic!("retrombo_nodef debería ser NOT DEFINABLE"),
            Err(_) => {}
        }
    }

    #[test]
    fn test_minimal_model_parses() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let model_path = project_root
            .join("testing")
            .join("tests_definibilidad")
            .join("fixtures")
            .join("minimal.model");
        if !model_path.exists() {
            return;
        }
        let model = parse_model(Some(&model_path), true).expect("Debe parsear");
        assert!(!model.universe.is_empty());
        assert!(model.relations.keys().any(|s| s.starts_with('T')));
    }

    #[test]
    fn test_modelo_solo_target_not_definable() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let model_path = project_root
            .join("testing")
            .join("tests_definibilidad")
            .join("fixtures")
            .join("modelo_solo_target.model");
        if !model_path.exists() {
            return;
        }
        match check_model(&model_path) {
            Ok(_) => panic!("Solo target sin operaciones debe ser NOT DEFINABLE"),
            Err(_) => {}
        }
    }

    /// Mismo modelo debe dar el mismo resultado con y sin IG, y con distintos ig_sample.
    #[test]
    fn test_definability_unchanged_by_ig_sample() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let definable = project_root
            .join("model_examples")
            .join("modeloqueanda.model");
        let not_definable = project_root.join("model_examples").join("suma4.model");
        if !definable.exists() || !not_definable.exists() {
            return;
        }
        let configs = [
            HitConfig::default(),
            HitConfig {
                use_information_gain: true,
                ig_sample: Some(1),
                ..HitConfig::default()
            },
            HitConfig {
                use_information_gain: true,
                ig_sample: Some(3),
                ..HitConfig::default()
            },
            HitConfig {
                use_information_gain: true,
                ig_sample: Some(5),
                ..HitConfig::default()
            },
            HitConfig {
                use_information_gain: true,
                ig_sample: Some(20),
                ..HitConfig::default()
            },
        ];
        for config in &configs {
            match check_model_with_config(&definable, *config) {
                Ok(_) => {}
                Err(_) => panic!(
                    "modeloqueanda debe ser DEFINABLE con ig_sample={:?}",
                    config.ig_sample
                ),
            }
        }
        for config in &configs {
            match check_model_with_config(&not_definable, *config) {
                Ok(_) => panic!(
                    "suma4 debe ser NOT DEFINABLE con ig_sample={:?}",
                    config.ig_sample
                ),
                Err(_) => {}
            }
        }
    }
}
