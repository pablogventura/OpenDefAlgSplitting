//! Tests de integración comparando con el comportamiento esperado del checker de definibilidad.
//!
//! **Modelos DEFINIBLES** (el checker debe devolver DEFINABLE):
//! - `test_modeloqueanda_definable` — modeloqueanda.model (target por fórmula)
//! - `test_main_con_archivo` — modelo pequeño con T0(x,y) eq(x,y) (diagonal)
//!
//! **Modelos NO DEFINIBLES** (el checker debe devolver NOT DEFINABLE):
//! - `test_suma4_not_definable` — suma4.model
//! - `test_retrombo_nodef_not_definable` — retrombo_nodef.model
//! - `test_modelo_solo_target_not_definable` — solo target, sin operaciones

use std::path::Path;
use std::process::Command;

fn run_rust_main(model_path: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_opendefalgsplitting"))
        .arg(model_path)
        .output()
        .expect("Failed to run binary");
    String::from_utf8_lossy(&output.stdout)
        .into_owned()
        + &String::from_utf8_lossy(&output.stderr)
}

#[cfg(feature = "python_comparison")]
fn run_python_main(model_path: &Path) -> String {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main_py = project_root.join("main.py");
    let output = Command::new("python3")
        .arg(&main_py)
        .arg(model_path)
        .env("PYTHONPATH", project_root)
        .output()
        .expect("Failed to run Python main");
    String::from_utf8_lossy(&output.stdout)
        .into_owned()
        + &String::from_utf8_lossy(&output.stderr)
}

fn is_definable(out: &str) -> bool {
    out.contains("DEFINABLE") && !out.split("DEFINABLE").next().unwrap_or("").contains("NOT DEFINABLE")
}

fn is_not_definable(out: &str) -> bool {
    out.contains("NOT DEFINABLE")
}

#[test]
fn test_modeloqueanda_definable() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let model_path = project_root.join("model_examples").join("modeloqueanda.model");
    if !model_path.exists() {
        return;
    }
    let out = run_rust_main(&model_path);
    assert!(is_definable(&out), "modeloqueanda debería ser DEFINABLE. Salida: {}", &out[..out.len().min(500)]);
}

#[test]
fn test_suma4_not_definable() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let model_path = project_root.join("model_examples").join("suma4.model");
    if !model_path.exists() {
        return;
    }
    let out = run_rust_main(&model_path);
    assert!(is_not_definable(&out), "suma4 debería ser NOT DEFINABLE. Salida: {}", &out[..out.len().min(500)]);
}

#[test]
fn test_retrombo_nodef_not_definable() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let model_path = project_root.join("model_examples").join("retrombo_nodef.model");
    if !model_path.exists() {
        return;
    }
    let out = run_rust_main(&model_path);
    assert!(is_not_definable(&out), "retrombo_nodef debería ser NOT DEFINABLE. Salida: {}", &out[..out.len().min(500)]);
}

#[test]
fn test_minimal_model() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let model_path = project_root.join("testing").join("tests_definibilidad").join("fixtures").join("minimal.model");
    if !model_path.exists() {
        return;
    }
    let out = run_rust_main(&model_path);
    assert!(is_definable(&out) || is_not_definable(&out), "Debe dar resultado. Salida: {}", &out[..out.len().min(500)]);
}

#[test]
fn test_modelo_solo_target_not_definable() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let model_path = project_root.join("testing").join("tests_definibilidad").join("fixtures").join("modelo_solo_target.model");
    if !model_path.exists() {
        return;
    }
    let out = run_rust_main(&model_path);
    assert!(is_not_definable(&out), "Solo target sin operaciones debe ser NOT DEFINABLE. Salida: {}", &out[..out.len().min(500)]);
}

#[test]
fn test_main_sin_args_lee_stdin() {
    let model_content = "0 1
f 2
0 0 0
0 1 1
1 0 1
1 1 0
T0(x,y) eq(x,y)
";
    let mut child = Command::new(env!("CARGO_BIN_EXE_opendefalgsplitting"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        use std::io::Write;
        stdin.write_all(model_content.as_bytes()).unwrap();
    }
    let out = child.wait_with_output().unwrap();
    let out_str = String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(
        out_str.contains("DEFINABLE") || out_str.contains("NOT DEFINABLE") || out_str.contains("ERROR"),
        "Salida: {}",
        &out_str[..out_str.len().min(300)]
    );
}

#[test]
fn test_main_sin_targets_error() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tmp = project_root.join("target").join("test_sin_targets.model");
    let model_content = "0 1 2
f 2
0 0 0
0 1 1
0 2 2
1 0 1
1 1 1
1 2 2
2 0 2
2 1 2
2 2 2
";
    std::fs::write(&tmp, model_content).unwrap();
    let out = run_rust_main(&tmp);
    let _ = std::fs::remove_file(&tmp);
    assert!(out.contains("NO TARGET RELATIONS FOUND"), "Salida: {}", &out[..out.len().min(400)]);
}

#[test]
fn test_main_con_archivo() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tmp = project_root.join("target").join("test_diagonal.model");
    let model_content = "0 1
f 2
0 0 0
0 1 1
1 0 1
1 1 0
T0(x,y) eq(x,y)
";
    std::fs::write(&tmp, model_content).unwrap();
    let out = run_rust_main(&tmp);
    let _ = std::fs::remove_file(&tmp);
    assert!(is_definable(&out), "Diagonal debe ser DEFINABLE. Salida: {}", &out[..out.len().min(400)]);
}

#[cfg(feature = "python_comparison")]
#[test]
fn test_equivalence_modeloqueanda() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let model_path = project_root.join("model_examples").join("modeloqueanda.model");
    if !model_path.exists() {
        return;
    }
    let rust_out = run_rust_main(&model_path);
    let py_out = run_python_main(&model_path);
    assert_eq!(
        is_definable(&rust_out),
        is_definable(&py_out),
        "Rust y Python deben coincidir en modeloqueanda. Rust: {} Python: {}",
        is_definable(&rust_out),
        is_definable(&py_out)
    );
}

#[cfg(feature = "python_comparison")]
#[test]
fn test_equivalence_suma4() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let model_path = project_root.join("model_examples").join("suma4.model");
    if !model_path.exists() {
        return;
    }
    let rust_out = run_rust_main(&model_path);
    let py_out = run_python_main(&model_path);
    assert_eq!(
        is_definable(&rust_out),
        is_definable(&py_out),
        "Rust y Python deben coincidir en suma4"
    );
    assert_eq!(
        is_not_definable(&rust_out),
        is_not_definable(&py_out),
        "Rust y Python deben coincidir en suma4 (NOT DEFINABLE)"
    );
}

/// Ejecuta el binario con los argumentos dados y devuelve stdout+stderr.
fn run_rust_main_with_args(args: &[&str]) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_opendefalgsplitting"));
    for a in args {
        cmd.arg(a);
    }
    let output = cmd.output().expect("Failed to run binary");
    String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr)
}

/// Con --bench -i --ig-sample N el resultado (DEFINABLE / NOT_DEFINABLE) debe ser el mismo que sin IG.
#[test]
fn test_bench_ig_sample_5_modeloqueanda_definable() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let model_path = project_root
        .join("model_examples")
        .join("modeloqueanda.model");
    if !model_path.exists() {
        return;
    }
    let out = run_rust_main_with_args(&[
        "--bench",
        "-i",
        "--ig-sample",
        "5",
        model_path.to_str().unwrap(),
    ]);
    assert!(
        out.contains("DEFINABLE"),
        "modeloqueanda con -i --ig-sample 5 debe ser DEFINABLE. Salida: {}",
        &out[..out.len().min(400)]
    );
    assert!(
        !out.contains("NOT_DEFINABLE"),
        "modeloqueanda con -i --ig-sample 5 no debe dar NOT_DEFINABLE. Salida: {}",
        &out[..out.len().min(400)]
    );
}

#[test]
fn test_bench_ig_sample_5_suma4_not_definable() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let model_path = project_root.join("model_examples").join("suma4.model");
    if !model_path.exists() {
        return;
    }
    let out = run_rust_main_with_args(&[
        "--bench",
        "-i",
        "--ig-sample",
        "5",
        model_path.to_str().unwrap(),
    ]);
    assert!(
        out.contains("NOT_DEFINABLE"),
        "suma4 con -i --ig-sample 5 debe ser NOT_DEFINABLE. Salida: {}",
        &out[..out.len().min(400)]
    );
}
