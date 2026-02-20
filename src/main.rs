use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::time::Instant;

use colored::Colorize;
use opendefalgsplitting::{
    first_order::formulas,
    hit::{is_open_def, HitConfig, Counterexample},
    parse_model,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }
    let (path_opt, hit_config) = parse_args(&args);
    let path = path_opt.as_deref().map(Path::new);

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

    for arity in arities {
        let targets_rels = targets_by_arity.get_mut(&arity).unwrap();
        if targets_rels.is_empty() {
            continue;
        }
        let _target_rel = targets_rels.first().unwrap().clone();
        targets_rels.sort_by(|a, b| a.sym.cmp(&b.sym));

        for target in targets_rels {
            match is_open_def(&model, vec![target.clone()], hit_config) {
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

Ejemplos:
  {} modelo.model
  {} modelo.model --information-gain --ig-sample 30
  {} -i modelo.model
",
        name, name, name, name
    );
}

/// Parsea argumentos: primer argumento posicional = path del modelo;
/// --information-gain / -i activa information gain; --ig-sample N fija el muestreo.
fn parse_args(args: &[String]) -> (Option<String>, HitConfig) {
    let mut path = None;
    let mut use_ig = false;
    let mut ig_sample = HitConfig::default().ig_sample;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--" {
            i += 1;
            if i < args.len() && path.is_none() {
                path = Some(args[i].clone());
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
        if path.is_none() {
            path = Some(args[i].clone());
        }
        i += 1;
    }
    let config = HitConfig {
        use_information_gain: use_ig,
        ig_sample,
    };
    (path, config)
}
