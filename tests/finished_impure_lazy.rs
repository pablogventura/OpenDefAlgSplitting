//! Regression: finished-impure compact/lazy vs Python false NOT on unary slice.
use opendefalgsplitting::engines::{check_engine, EngineKind, FragmentKind};
use opendefalgsplitting::hit::{is_open_def, HitConfig};
use opendefalgsplitting::parser::parse_model;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn write_unary_only_from_half_i0() -> PathBuf {
    let src = project_root().join(
        "benches/scale/corpus/pilot/random_magma_n32_k2_random_half_i0.model",
    );
    let raw = fs::read_to_string(&src).expect("corpus model");
    let mut body = Vec::new();
    for line in raw.lines() {
        if line.starts_with("T0 ") || line.starts_with("T0|") {
            break;
        }
        body.push(line.to_string());
    }
    let model = parse_model(Some(src.as_path()), true).expect("parse");
    let unary = model
        .relations
        .values()
        .find(|r| r.arity == 1)
        .cloned()
        .expect("unary preprocessed target");
    let mut lines = body;
    lines.push(format!("T0 {} {}", unary.r.len(), unary.arity));
    let mut rows: Vec<_> = unary.r.iter().cloned().collect();
    rows.sort();
    for t in rows {
        lines.push(
            t.iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    let out = std::env::temp_dir().join("hit_lazy_unary_half_i0.model");
    fs::write(&out, lines.join("\n") + "\n").expect("write");
    out
}

#[test]
fn unary_random_half_i0_hit_definable() {
    let path = write_unary_only_from_half_i0();
    let model = parse_model(Some(path.as_path()), true).expect("parse unary");
    let target = model
        .relations
        .values()
        .find(|r| r.sym.starts_with('T'))
        .cloned()
        .expect("T");
    let cfg = HitConfig {
        synthesize_formula: false,
        ..HitConfig::default()
    };
    let t0 = Instant::now();
    let res = is_open_def(&model, vec![target], cfg);
    assert!(
        t0.elapsed() < Duration::from_secs(5),
        "unary finished-impure should be cheap, took {:?}",
        t0.elapsed()
    );
    assert!(res.is_ok(), "Rust HIT must DEFINABLE on unary slice (Python false NOT)");
}

#[test]
fn unary_random_half_i0_merge_definable() {
    let path = write_unary_only_from_half_i0();
    let model = parse_model(Some(path.as_path()), true).expect("parse unary");
    let target = model
        .relations
        .values()
        .find(|r| r.sym.starts_with('T'))
        .cloned()
        .expect("T");
    let out = check_engine(
        &model,
        &target,
        FragmentKind::Qf,
        EngineKind::Merge,
        2,
        1,
    )
    .expect("merge");
    assert!(out.definable);
}
