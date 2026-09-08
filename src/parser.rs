use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use thiserror::Error;

use crate::first_order::formulas::{self, Formula, OpSym, Term, Variable};
use crate::first_order::models::Model;
use crate::first_order::relops::{Operation, Relation};
use crate::preprocessing;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Line {line} of {path:?}: {message}")]
    Parse {
        line: usize,
        path: Option<String>,
        message: String,
    },
}

pub type ParserError = ParseError;

fn c_input(line: &[u8]) -> String {
    let s = String::from_utf8_lossy(line);
    let s = s.trim();
    if let Some(idx) = s.find('#') {
        s[..idx].trim().to_string()
    } else {
        s.to_string()
    }
}

fn parse_universe(line: &str) -> Vec<i64> {
    line.split_whitespace()
        .filter_map(|s| s.parse::<i64>().ok())
        .collect()
}

fn parse_defrel(line: &str) -> Result<(String, usize, usize), String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() != 3 {
        return Err("Relation declaration needs SYM N ARITY".to_string());
    }
    let sym = parts[0].to_string();
    let ntuples: usize = parts[1].parse().map_err(|_| "Invalid ntuples")?;
    let arity: usize = parts[2].parse().map_err(|_| "Invalid arity")?;
    if arity == 0 {
        return Err("0-arity relation: use 0-arity operation for constants".to_string());
    }
    Ok((sym, ntuples, arity))
}

fn parse_defop(line: &str) -> Result<(String, usize), String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() != 2 {
        return Err("Operation declaration needs SYM ARITY".to_string());
    }
    let sym = parts[0].to_string();
    let arity: usize = parts[1].parse().map_err(|_| "Invalid arity")?;
    Ok((sym, arity))
}

fn parse_tuple(line: &str, universe: &[i64]) -> Result<Vec<i64>, String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let mut tuple = Vec::new();
    for p in parts {
        let x: i64 = p.parse().map_err(|_| format!("Invalid number: {}", p))?;
        if !universe.contains(&x) {
            return Err(format!("{} not in universe {:?}", x, universe));
        }
        tuple.push(x);
    }
    Ok(tuple)
}

/// Parser for formula expressions: eq(x,y), -f, f & g, f | g
fn parse_formula(
    s: &str,
    vars: &HashMap<String, Variable>,
    relations: &HashMap<String, Relation>,
    operations: &HashMap<String, Operation>,
) -> Result<Formula, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Empty formula".to_string());
    }
    // Parse | (disjunction) - lowest precedence
    if let Some(idx) = find_top_level(s, '|') {
        let left = parse_formula(&s[..idx], vars, relations, operations)?;
        let right = parse_formula(&s[idx + 1..], vars, relations, operations)?;
        return Ok(left.or_formula(&right));
    }
    // Parse & (conjunction)
    if let Some(idx) = find_top_level(s, '&') {
        let left = parse_formula(&s[..idx], vars, relations, operations)?;
        let right = parse_formula(&s[idx + 1..], vars, relations, operations)?;
        return Ok(left.and_formula(&right));
    }
    // Parse - (negation)
    let s = s.trim();
    if s.starts_with('-') {
        let inner = parse_formula(&s[1..], vars, relations, operations)?;
        return Ok(inner.neg());
    }
    // Parse eq(t1, t2) - find matching paren and first comma at depth 1
    if s.starts_with("eq(") {
        let _open = 2; // position of '('
        let mut depth = 1i32;
        let mut close = 0usize;
        for (i, c) in s[3..].char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        close = 3 + i;
                        break;
                    }
                }
                _ => {}
            }
        }
        let inner = &s[3..close];
        let comma = inner
            .char_indices()
            .scan(0i32, |depth, (i, c)| {
                match c {
                    '(' => *depth += 1,
                    ')' => *depth -= 1,
                    ',' if *depth == 0 => return Some(Some(i)),
                    _ => {}
                }
                Some(None)
            })
            .find_map(|x| x)
            .ok_or("eq needs two arguments")?;
        let t1_str = inner[..comma].trim();
        let t2_str = inner[comma + 1..].trim();
        let t1 = parse_term(t1_str, vars, operations)?;
        let t2 = parse_term(t2_str, vars, operations)?;
        return Ok(formulas::eq(t1, t2));
    }
    Err(format!("Cannot parse formula: {}", s))
}

fn find_top_level(s: &str, ch: char) -> Option<usize> {
    let mut depth: i32 = 0;
    for (i, c) in s.chars().enumerate() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            c if c == ch && depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

fn parse_term(
    s: &str,
    vars: &HashMap<String, Variable>,
    operations: &HashMap<String, Operation>,
) -> Result<Term, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Empty term".to_string());
    }
    // Variable
    if vars.contains_key(s) {
        return Ok(Term::Variable(vars[s].clone()));
    }
    // Operation application: f(x,y) or f(x)
    if let Some(lp) = s.find('(') {
        let name = &s[..lp];
        if !s.ends_with(')') {
            return Err(format!("Unclosed parenthesis in term: {}", s));
        }
        let args_str = &s[lp + 1..s.len() - 1];
        let args: Vec<Term> = split_top_level(args_str, ',')
            .iter()
            .map(|a| parse_term(a.trim(), vars, operations))
            .collect::<Result<_, _>>()?;
        let arity = args.len();
        if operations.contains_key(name) && operations[name].arity == arity {
            let sym = OpSym::new(name, arity);
            return Ok(Term::OpTerm {
                sym,
                args,
            });
        }
        return Err(format!("Unknown operation {} with arity {}", name, arity));
    }
    Err(format!("Unknown symbol in term: {}", s))
}

fn split_top_level(s: &str, ch: char) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut depth: i32 = 0;
    for c in s.chars() {
        match c {
            '(' => {
                depth += 1;
                current.push(c);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                current.push(c);
            }
            c if c == ch && depth == 0 => {
                result.push(current.trim().to_string());
                current.clear();
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        result.push(current.trim().to_string());
    }
    result
}

fn parse_defformula(
    line: &str,
    universe: &[i64],
    relations: &HashMap<String, Relation>,
    operations: &HashMap<String, Operation>,
) -> Result<Relation, String> {
    if line.contains("==") {
        return Err("Must use 'eq(x,y)' not '=='".to_string());
    }
    let lp = line.find('(').ok_or("Missing (")?;
    let sym = line[..lp].trim().to_string();
    let rest = &line[lp + 1..];
    let rp = rest.find(')').ok_or("Missing )")?;
    let declaration = rest[..rp].trim();
    let formula_str = rest[rp + 1..].trim();
    let decl_vars: Vec<&str> = declaration.split(',').map(|s| s.trim()).collect();
    if decl_vars.len() != decl_vars.iter().collect::<std::collections::HashSet<_>>().len() {
        return Err("Repeated variables in declaration".to_string());
    }
    let vars: HashMap<String, Variable> = decl_vars
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let v = Variable::from_index(i as i32);
            (s.to_string(), v)
        })
        .collect();
    let formula = parse_formula(formula_str, &vars, relations, operations)?;
    let free = formula.free_vars();
    let decl_set: std::collections::HashSet<_> = vars.values().cloned().collect();
    if !free.is_subset(&decl_set) {
        return Err("Formula has free variables not in declaration".to_string());
    }
    let model = Model::new(
        universe.to_vec(),
        relations.clone(),
        operations.clone(),
    );
    let arity = decl_vars.len();
    let ext = formula.extension(&model, Some(arity));
    let decl_list: Vec<Variable> = formula.implied_declaration().into_iter().collect();
    let decl_vars_vars: Vec<Variable> = decl_vars
        .iter()
        .map(|&s| vars[s].clone())
        .collect();
    let mut result = Relation::new(sym.clone(), arity);
    for t in ext {
        let mut reordered = vec![0i64; arity];
        for (j, dv) in decl_vars_vars.iter().enumerate() {
            if let Some(i) = decl_list.iter().position(|v| v == dv) {
                if i < t.len() {
                    reordered[j] = t[i];
                }
            }
        }
        result.add(reordered);
    }
    Ok(result)
}

pub fn parse_model(path: Option<&Path>, preprocess: bool) -> Result<Model, ParserError> {
    let path_str = path.map(|p| p.display().to_string());
    let reader: Box<dyn BufRead> = if let Some(p) = path {
        let f = File::open(p).map_err(|e| ParserError::Parse {
            line: 0,
            path: Some(p.display().to_string()),
            message: format!("File missing: {}", e),
        })?;
        let f = BufReader::new(f);
        if p.extension().map(|e| e.to_string_lossy() == "gz").unwrap_or(false) {
            Box::new(BufReader::new(flate2::bufread::GzDecoder::new(f)))
        } else {
            Box::new(f)
        }
    } else {
        Box::new(BufReader::new(std::io::stdin()))
    };

    let mut universe: Option<Vec<i64>> = None;
    let mut relations: HashMap<String, Relation> = HashMap::new();
    let mut operations: HashMap<String, Operation> = HashMap::new();
    let mut current_rel: Option<(Relation, usize)> = None;
    let mut current_op: Option<(Operation, usize)> = None;
    let mut linenumber: usize = 0;

    for line in reader.lines() {
        let line = line.map_err(|e| ParserError::Parse {
            line: linenumber,
            path: path_str.clone(),
            message: e.to_string(),
        })?;
        let line = c_input(line.as_bytes());
        linenumber += 1;

        if line.is_empty() {
            continue;
        }

        if universe.is_none() {
            universe = Some(parse_universe(&line));
            continue;
        }

        if current_rel.is_none() && current_op.is_none() {
            if line.contains('(') && !line.chars().next().map(|c| c == '@').unwrap_or(false) {
                let rel = parse_defformula(
                    &line,
                    universe.as_ref().unwrap(),
                    &relations,
                    &operations,
                ).map_err(|e| ParserError::Parse {
                    line: linenumber,
                    path: path_str.clone(),
                    message: e,
                })?;
                relations.insert(rel.sym.clone(), rel);
                continue;
            }
            if line.split_whitespace().count() == 2 {
                let (sym, arity) = parse_defop(&line).map_err(|e| ParserError::Parse {
                    line: linenumber,
                    path: path_str.clone(),
                    message: e,
                })?;
                let n = universe.as_ref().unwrap().len();
                let missing = n.pow(arity as u32);
                current_op = Some((Operation::new(sym, arity), missing));
                continue;
            }
            if line.split_whitespace().count() == 3 {
                let (sym, ntuples, arity) = parse_defrel(&line).map_err(|e| ParserError::Parse {
                    line: linenumber,
                    path: path_str.clone(),
                    message: e,
                })?;
                // Empty extension (ntuples == 0): commit immediately. Otherwise the
                // relation is never inserted and targets like T_empty disappear.
                if ntuples == 0 {
                    relations.insert(sym.clone(), Relation::new(sym, arity));
                    current_rel = None;
                } else {
                    current_rel = Some((Relation::new(sym, arity), ntuples));
                }
                continue;
            }
        }

        if let Some((ref mut rel, ref mut missing)) = current_rel {
            if *missing > 0 {
                let t = parse_tuple(&line, universe.as_ref().unwrap()).map_err(|e| {
                    ParserError::Parse {
                        line: linenumber,
                        path: path_str.clone(),
                        message: e,
                    }
                })?;
                rel.add(t);
                *missing -= 1;
            }
            if *missing == 0 {
                relations.insert(rel.sym.clone(), std::mem::replace(rel, Relation::new("", 0)));
                current_rel = None;
            }
            continue;
        }

        if let Some((ref mut op, ref mut missing)) = current_op {
            if *missing > 0 {
                let t = parse_tuple(&line, universe.as_ref().unwrap()).map_err(|e| {
                    ParserError::Parse {
                        line: linenumber,
                        path: path_str.clone(),
                        message: e,
                    }
                })?;
                if t.len() == op.arity + 1 {
                    op.add(t);
                }
                *missing -= 1;
            }
            if *missing == 0 {
                let (sym, _arity) = (op.sym.clone(), op.arity);
                let o = std::mem::replace(op, Operation::new("", 0));
                operations.insert(sym, o);
                current_op = None;
            }
        }
    }

    let universe = universe.ok_or_else(|| ParserError::Parse {
        line: linenumber,
        path: path_str.clone(),
        message: "Universe not defined".to_string(),
    })?;

    if let Some((ref rel, ref missing)) = current_rel {
        if *missing > 0 {
            return Err(ParserError::Parse {
                line: linenumber,
                path: path_str,
                message: format!("Missing tuples for relation {}", rel.sym),
            });
        }
    }

    if current_op.is_some() {
        return Err(ParserError::Parse {
            line: linenumber,
            path: path_str,
            message: "Missing tuples for operation".to_string(),
        });
    }

    if preprocess {
        let target_syms: Vec<String> = relations
            .keys()
            .filter(|s| s.starts_with('T'))
            .cloned()
            .collect();
        for sym in &target_syms {
            let rel = relations.remove(sym).unwrap();
            let prepped = preprocessing::preprocesamiento2(&rel);
            for r in prepped {
                relations.insert(r.sym.clone(), r);
            }
        }
    }

    Ok(Model::new(universe, relations, operations))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fixtures_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testing")
            .join("tests_definibilidad")
            .join("fixtures")
    }

    fn model_examples_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("model_examples")
    }

    #[test]
    fn test_parse_modelo_minimal() {
        let p = fixtures_dir().join("minimal.model");
        if !p.exists() {
            return;
        }
        let m = parse_model(Some(&p), true).expect("parse");
        assert_eq!(m.universe.len(), 2);
        assert!(m.universe.contains(&0) && m.universe.contains(&1));
        assert!(m.operations.contains_key("f"));
        assert!(m.relations.keys().any(|s| s.starts_with('T')));
    }

    #[test]
    fn test_parse_modelo_con_formula() {
        let p = model_examples_dir().join("modeloqueanda.model");
        if !p.exists() {
            return;
        }
        let m = parse_model(Some(&p), true).expect("parse");
        assert_eq!(m.universe.len(), 3);
        assert!(m.relations.keys().any(|s| s.starts_with('T')));
    }

    #[test]
    fn test_parse_modelo_existente() {
        let p = model_examples_dir().join("suma4.model");
        if !p.exists() {
            return;
        }
        let m = parse_model(Some(&p), true).expect("parse");
        assert_eq!(m.universe.len(), 4);
        assert!(m.operations.contains_key("S"));
    }

    #[test]
    fn test_parse_empty_target_relation() {
        let path = std::env::temp_dir().join("opendef_empty_t_test.model");
        std::fs::write(&path, "0 1\n\nT_empty 0 2\n").expect("write");
        let m = parse_model(Some(&path), true).expect("parse empty target");
        let _ = std::fs::remove_file(&path);
        assert!(m.relations.contains_key("T_empty"));
        assert!(m.relations["T_empty"].r.is_empty());
        assert_eq!(m.relations["T_empty"].arity, 2);
    }

    #[test]
    fn test_rechaza_relacion_0_aria() {
        let p = fixtures_dir().join("rel_0arity.model");
        if !p.exists() {
            return;
        }
        let r = parse_model(Some(&p), true);
        assert!(r.is_err());
        let err = r.unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("0-arity") || msg.contains("arity"), "{}", msg);
    }

    #[test]
    fn test_rechaza_igual_en_formula() {
        let p = fixtures_dir().join("formula_con_igual.model");
        if !p.exists() {
            return;
        }
        let r = parse_model(Some(&p), true);
        assert!(r.is_err());
        let msg = format!("{}", r.unwrap_err());
        assert!(msg.contains("eq") || msg.contains("=="), "{}", msg);
    }

    #[test]
    fn test_rechaza_variables_repetidas_en_formula() {
        let p = fixtures_dir().join("formula_vars_repetidas.model");
        if !p.exists() {
            return;
        }
        let r = parse_model(Some(&p), true);
        assert!(r.is_err());
        let msg = format!("{}", r.unwrap_err());
        assert!(msg.to_lowercase().contains("variable") || msg.contains("repeated"), "{}", msg);
    }
}
