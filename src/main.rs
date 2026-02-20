use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::time::Instant;

use colored::Colorize;
use opendefalgsplitting::{
    first_order::formulas,
    hit::{is_open_def, Counterexample},
    parse_model,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args.get(1).map(|s| Path::new(s));

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
            match is_open_def(&model, vec![target.clone()]) {
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
