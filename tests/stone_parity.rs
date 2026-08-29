//! Golden parity tests Lean fixtures ↔ Rust stone-filter.

use opendefalgsplitting::{filtering_functions, StoneSpec};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct GoldenFixture {
    expected: GoldenExpected,
}

#[derive(Deserialize)]
struct GoldenExpected {
    raw_op_count: usize,
    filtered_op_count: usize,
    operations: Vec<String>,
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("qfdef")
        .join("test/fixtures/stone")
        .join(name)
}

fn check_fixture(name: &str) {
    let path = fixture_path(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {name}: {e}"));
    let golden: GoldenFixture =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse golden {name}: {e}"));
    let spec = StoneSpec::parse_json(&text).expect("stone spec");
    let report = filtering_functions(&spec);
    assert_eq!(
        report.raw_op_count, golden.expected.raw_op_count,
        "{name} raw_op_count"
    );
    assert_eq!(
        report.filtered_op_count, golden.expected.filtered_op_count,
        "{name} filtered_op_count"
    );
    assert_eq!(report.operations, golden.expected.operations, "{name} operations");
}

#[test]
fn parity_fin2_example33() {
    check_fixture("fin2_example33.json");
}

#[test]
fn parity_fin2_valid() {
    check_fixture("fin2_valid.json");
}

#[test]
fn parity_fin3_valid() {
    check_fixture("fin3_valid.json");
}

#[test]
fn parity_fin4_valid() {
    check_fixture("fin4_valid.json");
}

#[test]
fn parity_fin5_valid() {
    check_fixture("fin5_valid.json");
}

#[test]
fn parity_fin3_nontrivial_filter() {
    check_fixture("fin3_nontrivial_filter.json");
}

#[test]
fn parity_counterexample_33() {
    check_fixture("counterexample_33.json");
}
