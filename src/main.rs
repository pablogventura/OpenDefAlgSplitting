use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::time::Instant;

use colored::Colorize;
use rayon::prelude::*;
use opendefalgsplitting::{
    first_order::formulas,
    hit::{is_open_def, ExploreOrder, HitConfig, Counterexample, reset_run_stats, run_stats_snapshot},
    parse_model,
    audit_unary_targets,
    select_strategy_explained, StrategyDecision,
    check_engine, EngineKind, FragmentKind,
};

/// Resultado de ejecutar el checker sobre un modelo (para benchmarks).
pub enum BenchResult {
    Definable,
    NotDefinable,
}

struct CliOptions {
    bench_mode: bool,
    unary_audit_mode: bool,
    paths: Vec<String>,
    repeat: u32,
    hit_config: HitConfig,
    class_mode: bool,
    ablation_csv: bool,
    use_auto: bool,
    explain_strategy: bool,
    fragment: String,
    engine: String,
    max_depth: usize,
    max_k: usize,
}

/// Ejecuta el checker sobre un modelo ya cargado; devuelve DEFINABLE o el primer contraejemplo. No imprime nada.
fn run_checker(
    model: &opendefalgsplitting::first_order::models::Model,
    targets_by_arity: &HashMap<usize, Vec<opendefalgsplitting::first_order::relops::Relation>>,
    hit_config: HitConfig,
    use_auto: bool,
) -> Result<BenchResult, Counterexample> {
    let mut arities: Vec<_> = targets_by_arity.keys().cloned().collect();
    arities.sort();
    for &arity in &arities {
        let targets_rels = targets_by_arity.get(&arity).unwrap();
        if targets_rels.is_empty() {
            continue;
        }
        let results: Vec<_> = targets_rels
            .par_iter()
            .map(|target| decide_target(model, target, hit_config, use_auto, false))
            .collect();
        for res in results {
            if let Err(ce) = res {
                return Err(ce);
            }
        }
    }
    Ok(BenchResult::Definable)
}

fn decide_target(
    model: &opendefalgsplitting::first_order::models::Model,
    target: &opendefalgsplitting::first_order::relops::Relation,
    hit_config: HitConfig,
    use_auto: bool,
    explain: bool,
) -> Result<opendefalgsplitting::first_order::formulas::Formula, Counterexample> {
    let config = if use_auto {
        let (decision, explanation) = select_strategy_explained(model, target);
        if explain {
            eprintln!("--- strategy for {} ---\n{}", target.sym, explanation);
        }
        match decision {
            StrategyDecision::RejectPattern { reason }
            | StrategyDecision::RejectUnary { reason } => {
                if explain {
                    eprintln!("reject: {}", reason);
                }
                return Err(Counterexample(
                    target.r.iter().take(1).cloned().collect(),
                ));
            }
            StrategyDecision::AcceptUnary { reason } => {
                if explain {
                    eprintln!("accept unary: {}", reason);
                }
                // Sin ops: ∅ -> false, A -> true (Lean qfDefinable_unary_noFunctions).
                if target.r.is_empty() {
                    return Ok(formulas::false_formula(None));
                }
                return Ok(formulas::true_formula(None));
            }
            StrategyDecision::Run { config, .. } => config,
        }
    } else {
        if explain {
            eprintln!(
                "--- strategy for {} ---\nmanual config (no auto): {:?}",
                target.sym, hit_config
            );
        }
        hit_config
    };
    is_open_def(model, vec![target.clone()], config)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() >= 2 && args[1] == "stone-filter" {
        run_stone_filter(&args[2..]);
        return;
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }
    let opts = parse_args(&args);

    if opts.bench_mode {
        run_bench_mode(
            &opts.paths,
            opts.repeat,
            opts.hit_config,
            opts.ablation_csv,
            opts.use_auto,
            &opts.fragment,
            &opts.engine,
            opts.max_depth,
            opts.max_k,
        );
        return;
    }
    if opts.unary_audit_mode {
        run_unary_audit_mode(&opts.paths);
        return;
    }
    if opts.class_mode {
        run_class_mode(&opts.paths, opts.hit_config, opts.use_auto);
        return;
    }

    let frag = match FragmentKind::parse(&opts.fragment) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let eng = match EngineKind::parse(&opts.engine) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let use_partition_engine = !matches!(frag, FragmentKind::Qf)
        || matches!(eng, EngineKind::Merge);

    let path = opts.paths.first().map(|s| Path::new(s.as_str()));
    let mut model = match parse_model(path, true) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    let target_syms: Vec<String> = model
        .relations
        .keys()
        .filter(|s| s.starts_with('T'))
        .cloned()
        .collect();

    if target_syms.is_empty() {
        eprintln!("ERROR: NO TARGET RELATIONS FOUND");
        std::process::exit(1);
    }

    if use_partition_engine {
        let start = Instant::now();
        let mut targets: Vec<_> = target_syms
            .iter()
            .filter_map(|s| model.relations.remove(s))
            .collect();
        targets.sort_by(|a, b| a.sym.cmp(&b.sym));
        // Historical MergingAlgorithm: all T* jointly (IsoType + polarity).
        if frag == opendefalgsplitting::FragmentKind::Qf
            && eng == opendefalgsplitting::EngineKind::Merge
        {
            let refs: Vec<_> = targets.iter().collect();
            match opendefalgsplitting::engines::megahit::check_qf_megahit_merge_multi(
                &model, &refs,
            ) {
                Ok(out) => {
                    if out.definable {
                        println!("{}", "DEFINABLE".green());
                    } else {
                        println!("{}", "NOT DEFINABLE".red());
                    }
                    let joined = targets
                        .iter()
                        .map(|t| t.sym.as_str())
                        .collect::<Vec<_>>()
                        .join(",");
                    println!(
                        "# meta: {{\"fragment\":\"{}\",\"engine\":\"{}\",\"target\":\"{}\"}}",
                        out.fragment, out.engine, joined
                    );
                }
                Err(e) => {
                    eprintln!("engine error: {e}");
                    std::process::exit(1);
                }
            }
            println!("Elapsed time: {:?}", start.elapsed());
            return;
        }
        for target in targets {
            match check_engine(
                &model,
                &target,
                frag,
                eng,
                opts.max_depth,
                opts.max_k,
            ) {
                Ok(out) => {
                    if out.definable {
                        println!("{}", "DEFINABLE".green());
                    } else {
                        println!("{}", "NOT DEFINABLE".red());
                    }
                    println!(
                        "# meta: {{\"fragment\":\"{}\",\"engine\":\"{}\",\"target\":\"{}\"}}",
                        out.fragment, out.engine, target.sym
                    );
                    if !out.definable {
                        println!("Elapsed time: {:?}", start.elapsed());
                        return;
                    }
                }
                Err(e) => {
                    eprintln!("engine error: {e}");
                    std::process::exit(1);
                }
            }
        }
        println!("Elapsed time: {:?}", start.elapsed());
        return;
    }

    // Legacy HIT path (qf split/hit/auto)
    let mut targets_by_arity: HashMap<usize, Vec<_>> = HashMap::new();
    for sym in &target_syms {
        let rel = model.relations.remove(sym).unwrap();
        targets_by_arity
            .entry(rel.arity)
            .or_default()
            .push(rel);
    }

    println!("{}", "********************".bold());
    println!("Deciding definability for subrelations");
    if opts.use_auto {
        println!("(strategy: auto)");
    }

    let mut formula = formulas::false_formula(None);
    let start = Instant::now();
    let mut arities: Vec<_> = targets_by_arity.keys().cloned().collect();
    arities.sort();

    for &arity in &arities {
        let targets_rels = targets_by_arity.get_mut(&arity).unwrap();
        if targets_rels.is_empty() {
            continue;
        }
        targets_rels.sort_by(|a, b| a.sym.cmp(&b.sym));
    }

    for arity in arities {
        let targets_rels = targets_by_arity.get(&arity).unwrap();
        if targets_rels.is_empty() {
            continue;
        }
        let results: Vec<_> = targets_rels
            .par_iter()
            .map(|target| {
                let res = decide_target(
                    &model,
                    target,
                    opts.hit_config,
                    opts.use_auto,
                    opts.explain_strategy,
                );
                (target, res)
            })
            .collect();

        for (target, res) in results {
            match res {
                Ok(f) => {
                    println!("\t{} is definable", target.sym.green());
                    println!("by {}", f);
                    let piece = match &target.pattern {
                        Some(p) => f.and_formula(&p.postprocessed_formula()),
                        None => f,
                    };
                    formula = formula.or_formula(&piece);
                }
                Err(Counterexample(tuples)) => {
                    println!("{}", "NOT DEFINABLE".red());
                    println!("\tCounterexample: {:?}", tuples);
                    println!("Elapsed time: {:?}", start.elapsed());
                    return;
                }
            }
        }
    }

    println!("{}", "DEFINABLE".green());
    if let Some(t) = targets_by_arity.values().flat_map(|v| v.first()).next() {
        println!("\t{} := {}", t.sym, formula);
    }
    println!("Elapsed time: {:?}", start.elapsed());

    // Check formula against the *original* target (superrel), not the preprocessed one
    if let Some(target) = targets_by_arity
        .values()
        .flat_map(|v| v.first())
        .next()
    {
        let (check_arity, check_r) = if let Some(superrel) = &target.superrel {
            (superrel.arity, &superrel.r)
        } else {
            (target.arity, &target.r)
        };
        let ext = formula.extension(&model, Some(check_arity));
        let target_set: std::collections::HashSet<_> = check_r.iter().cloned().collect();
        let ext_set: std::collections::HashSet<_> = ext.into_iter().collect();
        if target_set == ext_set {
            println!("{}", colored::Colorize::green("Formula successfully checked"));
        } else {
            println!("{}", colored::Colorize::red("Formula failed!"));
            println!("Extension len: {}", ext_set.len());
            println!("Target len:    {}", target_set.len());
        }
    }
}

fn load_model_targets(
    path: &Path,
) -> Result<
    (
        opendefalgsplitting::first_order::models::Model,
        HashMap<usize, Vec<opendefalgsplitting::first_order::relops::Relation>>,
    ),
    String,
> {
    let mut model = parse_model(Some(path), true).map_err(|e| e.to_string())?;
    let target_syms: Vec<String> = model
        .relations
        .keys()
        .filter(|s| s.starts_with('T'))
        .cloned()
        .collect();
    if target_syms.is_empty() {
        return Err("NO TARGET RELATIONS FOUND".into());
    }
    let mut targets_by_arity: HashMap<usize, Vec<_>> = HashMap::new();
    for sym in &target_syms {
        let rel = model.relations.remove(sym).unwrap();
        targets_by_arity.entry(rel.arity).or_default().push(rel);
    }
    Ok((model, targets_by_arity))
}

fn run_bench_mode(
    paths: &[String],
    repeat: u32,
    hit_config: HitConfig,
    ablation_csv: bool,
    use_auto: bool,
    fragment: &str,
    engine: &str,
    max_depth: usize,
    max_k: usize,
) {
    if paths.is_empty() {
        eprintln!("--bench requiere al menos un archivo .model");
        std::process::exit(1);
    }
    let frag = match FragmentKind::parse(fragment) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let eng = match EngineKind::parse(engine) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let use_partition_engine =
        !matches!(frag, FragmentKind::Qf) || matches!(eng, EngineKind::Merge);

    if ablation_csv {
        println!("model,ms,steps,skipped,approx_reject,result,formula_size");
    } else {
        println!("model\tms\tresult\tengine");
    }
    for path in paths {
        let path = Path::new(path);
        let mut times_ms: Vec<f64> = Vec::with_capacity(repeat as usize);
        let mut result_str = String::new();
        let mut engine_str = String::new();
        let mut steps = 0u64;
        let mut skipped = 0u64;
        let mut approx_r = 0u64;
        let formula_size = 0usize;
        for _ in 0..repeat {
            let (model, targets_by_arity) = match load_model_targets(path) {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("{}: {}", path.display(), e);
                    std::process::exit(1);
                }
            };
            reset_run_stats();
            let start = Instant::now();
            if use_partition_engine {
                let mut all_ok = true;
                let mut last_engine = String::new();
                if matches!(frag, FragmentKind::Qf) && matches!(eng, EngineKind::Merge) {
                    let mut flat: Vec<_> = targets_by_arity
                        .values()
                        .flatten()
                        .cloned()
                        .collect();
                    flat.sort_by(|a, b| a.sym.cmp(&b.sym));
                    let refs: Vec<_> = flat.iter().collect();
                    match opendefalgsplitting::engines::megahit::check_qf_megahit_merge_multi(
                        &model, &refs,
                    ) {
                        Ok(out) => {
                            last_engine = out.engine.clone();
                            all_ok = out.definable;
                        }
                        Err(e) => {
                            eprintln!("{}: {}", path.display(), e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    let mut arities: Vec<_> = targets_by_arity.keys().cloned().collect();
                    arities.sort();
                    for &arity in &arities {
                        for target in targets_by_arity.get(&arity).into_iter().flatten() {
                            match check_engine(&model, target, frag, eng, max_depth, max_k) {
                                Ok(out) => {
                                    last_engine = out.engine.clone();
                                    if !out.definable {
                                        all_ok = false;
                                    }
                                }
                                Err(e) => {
                                    eprintln!("{}: {}", path.display(), e);
                                    std::process::exit(1);
                                }
                            }
                        }
                    }
                }
                result_str = if all_ok {
                    "DEFINABLE".into()
                } else {
                    "NOT_DEFINABLE".into()
                };
                engine_str = last_engine;
            } else {
                let res = run_checker(&model, &targets_by_arity, hit_config, use_auto);
                engine_str = "hit_split".into();
                match &res {
                    Ok(BenchResult::Definable) => result_str = "DEFINABLE".into(),
                    Ok(BenchResult::NotDefinable) | Err(_) => {
                        result_str = "NOT_DEFINABLE".into()
                    }
                }
            }
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            times_ms.push(elapsed);
            let (s, _c, sk, ar) = run_stats_snapshot();
            steps = s;
            skipped = sk;
            approx_r = ar;
            let _ = formula_size;
        }
        let mean = times_ms.iter().sum::<f64>() / times_ms.len() as f64;
        if ablation_csv {
            println!(
                "{},{:.3},{},{},{},{},{}",
                path.display(),
                mean,
                steps,
                skipped,
                approx_r,
                result_str,
                formula_size
            );
        } else if repeat > 1 {
            let var =
                times_ms.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / times_ms.len() as f64;
            let std = var.sqrt();
            println!(
                "{}\t{:.3}±{:.3}\t{}\t{}",
                path.display(),
                mean,
                std,
                result_str,
                engine_str
            );
        } else {
            println!(
                "{}\t{:.3}\t{}\t{}",
                path.display(),
                mean,
                result_str,
                engine_str
            );
        }
    }
}

fn run_class_mode(paths: &[String], hit_config: HitConfig, use_auto: bool) {
    if paths.len() < 2 {
        eprintln!("--class requiere al menos 2 archivos .model");
        std::process::exit(1);
    }
    println!("model\tresult");
    let mut results = Vec::new();
    for path in paths {
        let path = Path::new(path);
        let (model, targets_by_arity) = match load_model_targets(path) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("{}: {}", path.display(), e);
                std::process::exit(1);
            }
        };
        let res = run_checker(&model, &targets_by_arity, hit_config, use_auto);
        let label = match res {
            Ok(BenchResult::Definable) => "DEFINABLE",
            Ok(BenchResult::NotDefinable) | Err(_) => "NOT_DEFINABLE",
        };
        println!("{}\t{}", path.display(), label);
        results.push(label);
    }
    let all_same = results.windows(2).all(|w| w[0] == w[1]);
    if all_same {
        println!("CLASS_VERDICT\tUNIFORM\t{}", results[0]);
    } else {
        println!("CLASS_VERDICT\tMIXED");
    }
}


fn csv_field(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "'"))
    } else {
        value.to_string()
    }
}

fn run_unary_audit_mode(paths: &[String]) {
    if paths.is_empty() {
        eprintln!("--unary-audit requires at least one model path");
        std::process::exit(1);
    }
    println!(
        "model,target,arity,unary_decision,has_nonunary_ops,would_early_exit,reason"
    );
    for path_str in paths {
        let path = Path::new(path_str);
        let model = match parse_model(Some(path), true) {
            Ok(m) => m,
            Err(e) => {
                eprintln!(
                    "{},{},{},{},{},{},{}",
                    csv_field(&path.display().to_string()),
                    csv_field(""),
                    csv_field("0"),
                    csv_field("ERROR"),
                    csv_field("false"),
                    csv_field("false"),
                    csv_field(&e.to_string()),
                );
                continue;
            }
        };
        let rows = audit_unary_targets(&model);
        if rows.is_empty() {
            println!(
                "{},{},{},{},{},{},{}",
                csv_field(&path.display().to_string()),
                csv_field(""),
                csv_field("0"),
                csv_field("NoTargets"),
                csv_field("false"),
                csv_field("false"),
                csv_field("no T... relations"),
            );
            continue;
        }
        for row in rows {
            println!(
                "{},{},{},{},{},{},{}",
                csv_field(&path.display().to_string()),
                csv_field(&row.target_sym),
                csv_field(&row.arity.to_string()),
                csv_field(&row.decision),
                csv_field(if row.has_nonunary_ops { "true" } else { "false" }),
                csv_field(if row.would_early_exit { "true" } else { "false" }),
                csv_field(&row.reason),
            );
        }
    }
}

fn print_help() {
    let name = std::env::args().next().unwrap_or_else(|| "opendefalgsplitting".into());
    eprintln!(
        "Uso: {name} [OPCIONES] [ARCHIVO.model]

Decide definibilidad QF de relaciones T... a partir de las operaciones del modelo.

Por defecto (sin flags de estrategia) usa selector automático: skip+simplify,
approx solo bajo presupuesto, y rechazo por patrón sin ops.

Opciones:
  -h, --help                 Ayuda
  --no-auto                  Desactiva el selector (usa HitConfig por defecto o flags)
  --explain-strategy         Imprime la decisión del selector (stderr)
  --no-skip-useless          Desactiva skip_useless (desactiva auto)
  -i, --information-gain     Calcula IG (legacy: no cambia el orden de splits)
  --ig-experimental          Aplica de verdad el mejor candidato IG (puede cambiar veredictos)
  --ig-sample N              Candidatos a muestrear con -i (0 = todos; default 20)
  --skip-useless             Forzar skip (desactiva auto)
  --approx-precheck          Forzar rechazo temprano approx (desactiva auto)
  --simplify                 Simplificar fórmula resultado (desactiva auto)
  --bfs                      Explorar el árbol en BFS (desactiva auto)
  --max-steps N              Tope de pasos de splitting (desactiva auto)
  --class M1 M2 ...          Modo clase: corre cada modelo y reporta veredictos
  --bench [MODELOS...]       Benchmark de tiempo
  --unary-audit [MODELOS...] CSV de cobertura del backend unario por target
  --repeat N                 Repeticiones en --bench (default 1)
  --ablation-csv             Con --bench, imprime CSV: model,ms,steps,skipped,result,...

Env (HIT):
  HIT_EAGER_ISOTYPE=0         Desactiva IsoType/TMH inmediato en bloques impuros
  HIT_MAX_RSS_MIB=N          Tope blando de RSS (KiB internos = N*1024)
  HIT_TMH_STATS=1            Contadores TMH / finished_impure en stderr
  HIT_TMH_* / HIT_PRE_FINISHED  Ver docs/TMH_FACTORS.md

Subcomando StoneGral (Alg. 1-2):
  {name} stone-filter --spec FIXTURE.json [--json]

Ejemplos:
  {name} model_examples/modeloqueanda.model
  {name} --explain-strategy model_examples/suma4.model
  {name} --no-auto --no-skip-useless model_examples/suma4.model
  {name} --skip-useless --simplify model_examples/suma4.model
  {name} --bench --repeat 3 --ablation-csv model_examples/modeloqueanda.model
"
    );
}


fn run_stone_filter(args: &[String]) {
    let mut spec_path: Option<String> = None;
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--spec" => {
                i += 1;
                if i < args.len() {
                    spec_path = Some(args[i].clone());
                }
            }
            "--json" => json_out = true,
            "--help" | "-h" => {
                eprintln!("Uso: stone-filter --spec FIXTURE.json [--json]");
                return;
            }
            other => {
                eprintln!("Opción desconocida: {other}");
                return;
            }
        }
        i += 1;
    }
    let Some(path) = spec_path else {
        eprintln!("Falta --spec FIXTURE.json");
        return;
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("No se pudo leer {path}: {e}");
            return;
        }
    };
    let spec = match opendefalgsplitting::StoneSpec::parse_json(&text) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Spec inválido: {e}");
            return;
        }
    };
    let report = opendefalgsplitting::filtering_functions(&spec);
    if json_out {
        match serde_json::to_string_pretty(&report) {
            Ok(s) => println!("{s}"),
            Err(e) => eprintln!("JSON error: {e}"),
        }
    } else {
        println!(
            "raw={} filtered={} ops={}",
            report.raw_op_count,
            report.filtered_op_count,
            report.operations.len()
        );
        for op in &report.operations {
            println!("  {op}");
        }
    }
}

fn parse_args(args: &[String]) -> CliOptions {
    let mut bench_mode = false;
    let mut unary_audit_mode = false;
    let mut class_mode = false;
    let mut paths = Vec::new();
    let mut repeat = 1u32;
    let defaults = HitConfig::default();
    let mut use_ig = false;
    let mut ig_experimental = false;
    let mut ig_sample = defaults.ig_sample;
    let mut skip_useless = defaults.skip_useless_candidates;
    let mut approx_precheck = false;
    if matches!(
        std::env::var("HIT_PRE_FINISHED").ok().as_deref(),
        Some("approx") | Some("APPROX")
    ) {
        approx_precheck = true;
    }
    let mut simplify = false;
    let mut bfs = false;
    let mut max_steps: Option<u64> = None;
    let mut ablation_csv = false;
    let mut no_auto = false;
    let mut explain_strategy = false;
    let mut strategy_manual = false;
    let mut fragment = "qf".to_string();
    let mut engine = "auto".to_string();
    let mut max_depth = 2usize;
    let mut max_k = 1usize;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--" => {
                i += 1;
                if i < args.len() {
                    paths.push(args[i].clone());
                }
            }
            "--bench" => bench_mode = true,
            "--unary-audit" => unary_audit_mode = true,
            "--class" => class_mode = true,
            "--ablation-csv" => ablation_csv = true,
            "--no-auto" => no_auto = true,
            "--explain-strategy" => explain_strategy = true,
            "--fragment" => {
                i += 1;
                if i < args.len() {
                    fragment = args[i].clone();
                }
            }
            "--engine" => {
                i += 1;
                if i < args.len() {
                    engine = args[i].clone();
                }
            }
            "--max-depth" => {
                i += 1;
                if i < args.len() {
                    if let Ok(n) = args[i].parse::<usize>() {
                        max_depth = n;
                    }
                }
            }
            "--max-k" => {
                i += 1;
                if i < args.len() {
                    if let Ok(n) = args[i].parse::<usize>() {
                        max_k = n;
                    }
                }
            }
            "--repeat" => {
                i += 1;
                if i < args.len() {
                    if let Ok(n) = args[i].parse::<u32>() {
                        repeat = n.max(1);
                    }
                }
            }
            "--information-gain" | "-i" => {
                use_ig = true;
                strategy_manual = true;
            }
            "--ig-experimental" => {
                use_ig = true;
                ig_experimental = true;
                strategy_manual = true;
            }
            "--ig-sample" => {
                i += 1;
                if i < args.len() {
                    if let Ok(n) = args[i].parse::<usize>() {
                        ig_sample = if n == 0 { None } else { Some(n) };
                    }
                }
                strategy_manual = true;
            }
            "--skip-useless" => {
                skip_useless = true;
                strategy_manual = true;
            }
            "--no-skip-useless" => {
                skip_useless = false;
                strategy_manual = true;
            }
            "--approx-precheck" => {
                approx_precheck = true;
                strategy_manual = true;
            }
            "--simplify" => {
                simplify = true;
                strategy_manual = true;
            }
            "--bfs" => {
                bfs = true;
                strategy_manual = true;
            }
            "--max-steps" => {
                i += 1;
                if i < args.len() {
                    if let Ok(n) = args[i].parse::<u64>() {
                        max_steps = Some(n);
                    }
                }
                strategy_manual = true;
            }
            s if s.starts_with('-') => {}
            s => paths.push(s.to_string()),
        }
        i += 1;
    }
    if !bench_mode && !class_mode && !unary_audit_mode && paths.len() > 1 {
        paths.truncate(1);
    }
    let use_auto = !no_auto && !strategy_manual;
    let max_rss_kib = std::env::var("HIT_MAX_RSS_MIB")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(|mib| mib.saturating_mul(1024));
    let config = HitConfig {
        use_information_gain: use_ig,
        ig_sample,
        ig_experimental,
        skip_useless_candidates: skip_useless,
        approx_precheck,
        emit_all_constants: true,
        max_steps,
        simplify_formula: simplify,
        explore_order: if bfs {
            ExploreOrder::Bfs
        } else {
            ExploreOrder::Dfs
        },
        max_rayon_children: 1,
        synthesize_formula: !bench_mode,
        max_rss_kib,
        eager_isotype_on_impure: match std::env::var("HIT_EAGER_ISOTYPE").ok().as_deref() {
            Some("0") | Some("false") | Some("FALSE") | Some("no") | Some("NO") => false,
            _ => true,
        },
    };
    CliOptions {
        bench_mode,
        unary_audit_mode,
        paths,
        repeat,
        hit_config: config,
        class_mode,
        ablation_csv,
        use_auto,
        explain_strategy,
        fragment,
        engine,
        max_depth,
        max_k,
    }
}
