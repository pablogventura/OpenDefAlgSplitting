use std::collections::HashSet;

use crate::preprocessing::Pattern;

#[derive(Debug, Clone)]
pub struct Relation {
    pub sym: String,
    pub arity: usize,
    pub r: HashSet<Vec<i64>>,
    pub pattern: Option<Box<Pattern>>,
    pub superrel_sym: Option<String>,
    /// Relación original (antes del preprocesamiento); usada para comprobar la fórmula.
    pub superrel: Option<Box<Relation>>,
}

impl Relation {
    pub fn new(sym: impl Into<String>, arity: usize) -> Self {
        Self {
            sym: sym.into(),
            arity,
            r: HashSet::new(),
            pattern: None,
            superrel_sym: None,
            superrel: None,
        }
    }

    pub fn with_tuples(mut self, tuples: impl IntoIterator<Item = Vec<i64>>) -> Self {
        for t in tuples {
            if t.len() == self.arity {
                self.r.insert(t);
            }
        }
        self
    }

    pub fn add(&mut self, t: Vec<i64>) {
        if t.len() == self.arity {
            self.r.insert(t);
        }
    }

    pub fn contains(&self, args: &[i64]) -> bool {
        self.r.contains(args)
    }

    pub fn restrict(&self, subuniverse: &HashSet<i64>) -> Self {
        let mut result = Relation::new(&self.sym, self.arity);
        for t in &self.r {
            if t.iter().all(|x| subuniverse.contains(x)) {
                result.add(t.clone());
            }
        }
        result
    }
}

impl PartialEq for Relation {
    fn eq(&self, other: &Self) -> bool {
        self.sym == other.sym && self.r == other.r
    }
}
impl Eq for Relation {}

impl std::hash::Hash for Relation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.sym.hash(state);
        for t in &self.r {
            for x in t {
                x.hash(state);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Operation {
    pub sym: String,
    pub arity: usize,
    pub op: std::collections::HashMap<Vec<i64>, i64>,
}

impl Operation {
    pub fn new(sym: impl Into<String>, arity: usize) -> Self {
        Self {
            sym: sym.into(),
            arity,
            op: std::collections::HashMap::new(),
        }
    }

    pub fn add(&mut self, t: Vec<i64>) {
        if t.len() >= self.arity + 1 {
            let args: Vec<i64> = t[..self.arity].to_vec();
            let result = t[self.arity];
            self.op.insert(args, result);
        }
    }

    pub fn call(&self, args: &[i64]) -> Option<i64> {
        if args.len() == self.arity {
            self.op.get(args).copied()
        } else {
            None
        }
    }

    pub fn restrict(&self, subuniverse: &HashSet<i64>) -> Self {
        let mut result = Operation::new(&self.sym, self.arity);
        for (t, v) in &self.op {
            if t.iter().all(|x| subuniverse.contains(x)) && subuniverse.contains(v) {
                let mut row = t.clone();
                row.push(*v);
                result.add(row);
            }
        }
        result
    }
}
