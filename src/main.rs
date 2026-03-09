use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::time::Instant;

use colored::Colorize;
use rayon::prelude::*;
use opendefalgsplitting::{
    first_order::formulas,
    hit::{is_open_def, HitConfig, Counterexample},
    parse_model,
};

/// Resultado de ejecutar el checker sobre un modelo (para benchmarks).
pub enum BenchResult {
    Definable,
    NotDefinable,
}

/// Ejecuta el checker sobre un modelo ya cargado; devuelve DEFINABLE o el primer contraejemplo. No imprime nada.
fn run_checker(
    model: &opendefalgsplitting::first_order::models::Model,
    targets_by_arity: &HashMap<usize, Vec<opendefalgsplitting::first_order::relops::Relation>>,
    hit_config: HitConfig,
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
            .map(|target| is_open_def(model, vec![target.clone()], hit_config))
            .collect();
        for res in results {
            if let Err(ce) = res {
                return Err(ce);
            }
        }
    }
    Ok(BenchResult::Definable)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }
    let (bench_mode, paths, repeat, hit_config) = parse_args(&args);

    if bench_mode {
        run_bench_mode(&paths, repeat, hit_config);
        return;
    }

    let path = paths.first().map(|s| Path::new(s.as_str()));
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
                let res = is_open_def(&model, vec![target.clone()], hit_config);
                (target, res)
            })
            .collect();

        for (target, res) in results {
            match res {
                Ok(f) => {
                    println!("\t{} is definable", target.sym.green());
                    println!("by {}", f);
                    formula = formula.or_formula(&{
                        let post = target.pattern.as_ref().unwrap().postprocessed_formula();
                        f.and_formula(&post)
                    });
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

fn run_bench_mode(paths: &[String], repeat: u32, hit_config: HitConfig) {
    println!("model\tms\tresult");
    if paths.is_empty() {
        eprintln!("--bench requiere al menos un archivo .model");
        std::process::exit(1);
    }
    for path in paths {
        let path = Path::new(path);
        let mut times_ms: Vec<f64> = Vec::with_capacity(repeat as usize);
        let mut result_str = String::new();
        for _ in 0..repeat {
            let (model, targets_by_arity) = match load_model_and_targets(path) {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("{}: {}", path.display(), e);
                    result_str = format!("error: {}", e);
                    break;
                }
            };
            let start = Instant::now();
            match run_checker(&model, &targets_by_arity, hit_config) {
                Ok(BenchResult::Definable) => result_str = "DEFINABLE".to_string(),
                Ok(BenchResult::NotDefinable) => result_str = "NOT_DEFINABLE".to_string(),
                Err(_) => result_str = "NOT_DEFINABLE".to_string(),
            }
            times_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        if times_ms.is_empty() {
            println!("{}\t-\t{}", path.display(), result_str);
        } else if times_ms.len() == 1 {
            println!("{}\t{:.2}\t{}", path.display(), times_ms[0], result_str);
        } else {
            let mean = times_ms.iter().sum::<f64>() / times_ms.len() as f64;
            let variance = times_ms.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / times_ms.len() as f64;
            let std = variance.sqrt();
            println!("{}\t{:.2} ± {:.2}\t{}\t(n={})", path.display(), mean, std, result_str, times_ms.len());
        }
    }
}

fn load_model_and_targets(
    path: &Path,
) -> Result<
    (
        opendefalgsplitting::first_order::models::Model,
        HashMap<usize, Vec<opendefalgsplitting::first_order::relops::Relation>>,
    ),
    String,
> {
    let model = parse_model(Some(path), true).map_err(|e| e.to_string())?;
    let target_syms: Vec<String> = model
        .relations
        .keys()
        .filter(|s| s.starts_with('T'))
        .cloned()
        .collect();
    if target_syms.is_empty() {
        return Err("NO TARGET RELATIONS".to_string());
    }
    let mut targets_by_arity: HashMap<usize, Vec<_>> = HashMap::new();
    for sym in &target_syms {
        let rel = model.relations.get(sym).cloned().unwrap();
        targets_by_arity.entry(rel.arity).or_default().push(rel);
    }
    Ok((model, targets_by_arity))
}

fn print_help() {
    let name = env::args().next().unwrap_or_else(|| "opendefalgsplitting".into());
    let name = name.as_str();
    eprintln!(
        "Uso: {} [OPCIONES] [ARCHIVO.model]

Decide si las relaciones objetivo (T...) son definibles en lógica de primer orden
a partir de las operaciones del modelo. Si no se pasa ARCHIVO, lee el modelo por stdin.

Opciones:
  -h, --help              Muestra esta ayuda
  -i, --information-gain  Elige cada paso maximizando information gain (más lento, a veces menos pasos)
  --ig-sample N           Con -i, candidatos a muestrear; 0 = sin límite (por defecto: 20)
  --bench [MODELOS...]     Modo benchmark: mide tiempo por modelo (varios archivos). Ver abajo.
  --repeat N              Con --bench, ejecuta cada modelo N veces y muestra media ± desv. (por defecto: 1)

Ejemplos:
  {} modelo.model
  {} modelo.model --information-gain --ig-sample 30
  {} -i modelo.model
  {} --bench modelo1.model modelo2.model
  {} --bench --repeat 3 -i model_examples/gigante.model
",
        name, name, name, name, name, name
    );
}

/// Parsea argumentos. Devuelve (bench_mode, paths, repeat, hit_config).
/// Con --bench, paths son todos los argumentos posicionales; si no, paths tiene 0 o 1 elemento.
fn parse_args(args: &[String]) -> (bool, Vec<String>, u32, HitConfig) {
    let mut bench_mode = false;
    let mut paths = Vec::new();
    let mut repeat = 1u32;
    let mut use_ig = false;
    let mut ig_sample = HitConfig::default().ig_sample;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--" {
            i += 1;
            if i < args.len() {
                paths.push(args[i].clone());
            }
            i += 1;
            continue;
        }
        if args[i] == "--bench" {
            bench_mode = true;
            i += 1;
            continue;
        }
        if args[i] == "--repeat" {
            i += 1;
            if i < args.len() {
                if let Ok(n) = args[i].parse::<u32>() {
                    repeat = n.max(1);
                }
            }
            i += 1;
            continue;
        }
        if args[i] == "--information-gain" || args[i] == "-i" {
            use_ig = true;
            i += 1;
            continue;
        }
        if args[i] == "--ig-sample" {
            i += 1;
            if i < args.len() {
                if let Ok(n) = args[i].parse::<usize>() {
                    ig_sample = if n == 0 { None } else { Some(n) };
                }
            }
            i += 1;
            continue;
        }
        if args[i].starts_with('-') {
            i += 1;
            continue;
        }
        paths.push(args[i].clone());
        i += 1;
    }
    if !bench_mode && paths.len() > 1 {
        paths.truncate(1);
    }
    let config = HitConfig {
        use_information_gain: use_ig,
        ig_sample,
    };
    (bench_mode, paths, repeat, config)
}
