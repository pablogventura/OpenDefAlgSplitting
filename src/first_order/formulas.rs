use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// Variable in first-order logic
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Variable {
    pub sym: String,
}

impl Variable {
    pub fn new(sym: impl Into<String>) -> Self {
        Self { sym: sym.into() }
    }

    pub fn from_index(i: i32) -> Self {
        Self {
            sym: format!("x{}", subscript(i)),
        }
    }
}

fn subscript(n: i32) -> String {
    const SUB: &[char] = &[
        '₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉',
    ];
    n.to_string()
        .chars()
        .map(|c| {
            if c == '-' {
                '₋'
            } else if let Some(d) = c.to_digit(10) {
                SUB[d as usize]
            } else {
                c
            }
        })
        .collect()
}

impl std::fmt::Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.sym)
    }
}

/// Operation symbol
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OpSym {
    pub op: String,
    pub arity: usize,
}

impl OpSym {
    pub fn new(op: impl Into<String>, arity: usize) -> Self {
        Self {
            op: op.into(),
            arity,
        }
    }
}

impl std::fmt::Display for OpSym {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.op)
    }
}

/// Term in first-order logic
#[derive(Debug, Clone)]
pub enum Term {
    Variable(Variable),
    OpTerm { sym: OpSym, args: Vec<Term> },
}

impl Term {
    pub fn free_vars(&self) -> HashSet<Variable> {
        match self {
            Term::Variable(v) => {
                let mut s = HashSet::new();
                s.insert(v.clone());
                s
            }
            Term::OpTerm { args, .. } => args.iter().flat_map(|t| t.free_vars()).collect(),
        }
    }

    pub fn grade(&self) -> usize {
        match self {
            Term::Variable(_) => 0,
            Term::OpTerm { args, .. } => {
                1 + args.iter().map(|t| t.grade()).max().unwrap_or(0)
            }
        }
    }

    pub fn evaluate(
        &self,
        operations: &HashMap<String, crate::first_order::relops::Operation>,
        vector: &HashMap<Variable, i64>,
    ) -> Result<i64, String> {
        match self {
            Term::Variable(v) => vector
                .get(v)
                .copied()
                .ok_or_else(|| format!("Variable {} not bound", v.sym)),
            Term::OpTerm { sym, args } => {
                let args_vals: Result<Vec<i64>, _> =
                    args.iter().map(|t| t.evaluate(operations, vector)).collect();
                let args_vals = args_vals?;
                let op = operations
                    .get(&sym.op)
                    .ok_or_else(|| format!("Operation {} not found", sym.op))?;
                op.call(&args_vals)
                    .ok_or_else(|| format!("Operation {} undefined for args {:?}", sym.op, args_vals))
            }
        }
    }
}

impl PartialEq for Term {
    fn eq(&self, other: &Self) -> bool {
        self.grade() == other.grade() && format!("{:?}", self) == format!("{:?}", other)
    }
}
impl Eq for Term {}

impl Hash for Term {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Term::Variable(v) => v.hash(state),
            Term::OpTerm { sym, args } => {
                sym.hash(state);
                for a in args {
                    a.hash(state);
                }
            }
        }
    }
}

impl PartialOrd for Term {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Term {
    fn cmp(&self, other: &Self) -> Ordering {
        self.grade().cmp(&other.grade()).then_with(|| {
            format!("{:?}", self).cmp(&format!("{:?}", other))
        })
    }
}

impl std::fmt::Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Term::Variable(v) => write!(f, "{}", v),
            Term::OpTerm { sym, args } => {
                write!(f, "{}(", sym)?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", a)?;
                }
                write!(f, ")")
            }
        }
    }
}

/// Formula in first-order logic
#[derive(Debug, Clone)]
pub enum Formula {
    True(HashSet<Variable>),
    False(HashSet<Variable>),
    Eq(Term, Term),
    Neg(Box<Formula>),
    And(HashSet<Formula>),
    Or(HashSet<Formula>),
}

impl Formula {
    /// Cantidad de nodos del AST (métrica de tamaño).
    pub fn ast_size(&self) -> usize {
        match self {
            Formula::True(_) | Formula::False(_) | Formula::Eq(_, _) => 1,
            Formula::Neg(f) => 1 + f.ast_size(),
            Formula::And(fs) | Formula::Or(fs) => {
                1 + fs.iter().map(|f| f.ast_size()).sum::<usize>()
            }
        }
    }

    /// Simplificación estructural ligera (True/False en And/Or, doble negación).
    pub fn simplify_ast(&self) -> Formula {
        match self {
            Formula::True(v) => Formula::True(v.clone()),
            Formula::False(v) => Formula::False(v.clone()),
            Formula::Eq(a, b) => {
                if a == b {
                    Formula::True(a.free_vars())
                } else {
                    Formula::Eq(a.clone(), b.clone())
                }
            }
            Formula::Neg(f) => {
                let s = f.simplify_ast();
                match s {
                    Formula::True(v) => Formula::False(v),
                    Formula::False(v) => Formula::True(v),
                    Formula::Neg(inner) => *inner,
                    other => Formula::Neg(Box::new(other)),
                }
            }
            Formula::And(fs) => {
                let mut out = HashSet::new();
                let mut vars = HashSet::new();
                for f in fs {
                    match f.simplify_ast() {
                        Formula::True(v) => {
                            vars.extend(v);
                        }
                        Formula::False(v) => return Formula::False(v),
                        Formula::And(inner) => {
                            for g in inner {
                                out.insert(g);
                            }
                        }
                        other => {
                            vars.extend(other.free_vars());
                            out.insert(other);
                        }
                    }
                }
                if out.is_empty() {
                    Formula::True(vars)
                } else if out.len() == 1 {
                    out.into_iter().next().unwrap()
                } else {
                    Formula::And(out)
                }
            }
            Formula::Or(fs) => {
                let mut out = HashSet::new();
                let mut vars = HashSet::new();
                for f in fs {
                    match f.simplify_ast() {
                        Formula::False(v) => {
                            vars.extend(v);
                        }
                        Formula::True(v) => return Formula::True(v),
                        Formula::Or(inner) => {
                            for g in inner {
                                out.insert(g);
                            }
                        }
                        other => {
                            vars.extend(other.free_vars());
                            out.insert(other);
                        }
                    }
                }
                if out.is_empty() {
                    Formula::False(vars)
                } else if out.len() == 1 {
                    out.into_iter().next().unwrap()
                } else {
                    Formula::Or(out)
                }
            }
        }
    }

    pub fn free_vars(&self) -> HashSet<Variable> {
        match self {
            Formula::True(v) | Formula::False(v) => v.clone(),
            Formula::Eq(t1, t2) => t1.free_vars().union(&t2.free_vars()).cloned().collect(),
            Formula::Neg(f) => f.free_vars(),
            Formula::And(fs) | Formula::Or(fs) => {
                fs.iter().flat_map(|f| f.free_vars()).collect()
            }
        }
    }

    pub fn implied_declaration(&self) -> Vec<Variable> {
        let mut v: Vec<_> = self.free_vars().into_iter().collect();
        v.sort_by(|a, b| a.sym.cmp(&b.sym));
        v
    }

    pub fn satisfy(
        &self,
        model: &crate::first_order::models::Model,
        vector: &HashMap<Variable, i64>,
    ) -> bool {
        match self {
            Formula::True(_) => true,
            Formula::False(_) => false,
            Formula::Eq(t1, t2) => {
                let v1 = t1.evaluate(&model.operations, vector).unwrap_or(i64::MIN);
                let v2 = t2.evaluate(&model.operations, vector).unwrap_or(i64::MIN);
                v1 == v2
            }
            Formula::Neg(f) => !f.satisfy(model, vector),
            Formula::And(fs) => fs.iter().all(|f| f.satisfy(model, vector)),
            Formula::Or(fs) => fs.iter().any(|f| f.satisfy(model, vector)),
        }
    }

    pub fn extension(
        &self,
        model: &crate::first_order::models::Model,
        arity: Option<usize>,
    ) -> HashSet<Vec<i64>> {
        let vs = self.implied_declaration();
        let a = arity.unwrap_or(vs.len());
        match self {
            Formula::True(_) => {
                if let Some(arity) = arity {
                    cartesian_product(&model.universe, arity).into_iter().collect()
                } else {
                    HashSet::new()
                }
            }
            Formula::False(_) => HashSet::new(),
            _ => {
                if vs.is_empty() {
                    return HashSet::new();
                }
                let mut result = HashSet::new();
                for tuple in cartesian_product(&model.universe, vs.len()) {
                    let vector: HashMap<Variable, i64> =
                        vs.iter().cloned().zip(tuple.iter().cloned()).collect();
                    if self.satisfy(model, &vector) {
                        let t: Vec<i64> = vs.iter().map(|v| vector[v]).collect();
                        result.insert(t);
                    }
                }
                if a != vs.len() {
                    let mut adjusted = HashSet::new();
                    for t in result {
                        adjusted.insert(t.into_iter().take(a).collect());
                    }
                    return adjusted;
                }
                result
            }
        }
    }
}

fn cartesian_product(universe: &[i64], n: usize) -> Vec<Vec<i64>> {
    let mut result = vec![vec![]];
    for _ in 0..n {
        let mut next = Vec::new();
        for r in &result {
            for &u in universe {
                let mut row = r.clone();
                row.push(u);
                next.push(row);
            }
        }
        result = next;
    }
    result
}

impl PartialEq for Formula {
    fn eq(&self, other: &Self) -> bool {
        formula_hash(self) == formula_hash(other)
    }
}
impl Eq for Formula {}

impl Hash for Formula {
    fn hash<H: Hasher>(&self, state: &mut H) {
        formula_hash(self).hash(state);
    }
}

fn formula_hash(f: &Formula) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    let mut h = DefaultHasher::new();
    match f {
        Formula::True(v) => {
            "T".hash(&mut h);
            for x in v {
                x.hash(&mut h);
            }
        }
        Formula::False(v) => {
            "F".hash(&mut h);
            for x in v {
                x.hash(&mut h);
            }
        }
        Formula::Eq(t1, t2) => {
            "eq".hash(&mut h);
            t1.hash(&mut h);
            t2.hash(&mut h);
        }
        Formula::Neg(g) => {
            "neg".hash(&mut h);
            formula_hash(g).hash(&mut h);
        }
        Formula::And(fs) => {
            "and".hash(&mut h);
            let mut arr: Vec<_> = fs.iter().map(formula_hash).collect();
            arr.sort();
            for x in arr {
                x.hash(&mut h);
            }
        }
        Formula::Or(fs) => {
            "or".hash(&mut h);
            let mut arr: Vec<_> = fs.iter().map(formula_hash).collect();
            arr.sort();
            for x in arr {
                x.hash(&mut h);
            }
        }
    }
    h.finish()
}

impl std::fmt::Display for Formula {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Formula::True(v) => {
                let vars: Vec<_> = v.iter().map(|x| x.sym.as_str()).collect();
                write!(f, "⊤({})", vars.join(","))
            }
            Formula::False(v) => {
                let vars: Vec<_> = v.iter().map(|x| x.sym.as_str()).collect();
                write!(f, "⊥({})", vars.join(","))
            }
            Formula::Eq(t1, t2) => write!(f, "{} == {}", t1, t2),
            Formula::Neg(g) => write!(f, "¬{}", g),
            Formula::And(fs) => {
                let parts: Vec<_> = fs.iter().map(|x| format!("{}", x)).collect();
                write!(f, "({})", parts.join(" ∧ "))
            }
            Formula::Or(fs) => {
                let parts: Vec<_> = fs.iter().map(|x| format!("{}", x)).collect();
                write!(f, "({})", parts.join(" ∨ "))
            }
        }
    }
}

/// Constructor functions
pub fn variables(indices: &[i32]) -> Vec<Variable> {
    indices.iter().map(|&i| Variable::from_index(i)).collect()
}

pub fn eq(t1: Term, t2: Term) -> Formula {
    if t1 == t2 {
        let mut v = t1.free_vars();
        v.extend(t2.free_vars());
        Formula::True(v)
    } else {
        Formula::Eq(t1, t2)
    }
}

pub fn true_formula(orphan_vars: Option<HashSet<Variable>>) -> Formula {
    Formula::True(orphan_vars.unwrap_or_default())
}

pub fn false_formula(orphan_vars: Option<HashSet<Variable>>) -> Formula {
    Formula::False(orphan_vars.unwrap_or_default())
}

impl Formula {
    pub fn and_formula(&self, other: &Formula) -> Formula {
        match (self, other) {
            (Formula::True(_), f) | (f, Formula::True(_)) => f.clone(),
            (Formula::False(v1), Formula::False(v2)) => {
                Formula::False(v1.union(v2).cloned().collect())
            }
            (Formula::False(v), _) | (_, Formula::False(v)) => Formula::False(v.clone()),
            (Formula::Neg(a), b) | (b, Formula::Neg(a)) if a.as_ref() == b => {
                Formula::False(self.free_vars().union(&other.free_vars()).cloned().collect())
            }
            (Formula::And(fs1), Formula::And(fs2)) => {
                let mut s = fs1.clone();
                s.extend(fs2.clone());
                Formula::And(s)
            }
            (Formula::And(fs), o) | (o, Formula::And(fs)) => {
                let mut s = fs.clone();
                s.insert(o.clone());
                Formula::And(s)
            }
            _ => {
                let mut s = HashSet::new();
                s.insert(self.clone());
                s.insert(other.clone());
                Formula::And(s)
            }
        }
    }

    pub fn or_formula(&self, other: &Formula) -> Formula {
        match (self, other) {
            (Formula::False(_), f) | (f, Formula::False(_)) => f.clone(),
            (Formula::True(v1), Formula::True(v2)) => {
                Formula::True(v1.union(v2).cloned().collect())
            }
            (Formula::True(v), _) | (_, Formula::True(v)) => Formula::True(v.clone()),
            (Formula::Neg(a), b) | (b, Formula::Neg(a)) if a.as_ref() == b => {
                Formula::True(self.free_vars().union(&other.free_vars()).cloned().collect())
            }
            (Formula::Or(fs1), Formula::Or(fs2)) => {
                let mut s = fs1.clone();
                s.extend(fs2.clone());
                Formula::Or(s)
            }
            (Formula::Or(fs), o) | (o, Formula::Or(fs)) => {
                let mut s = fs.clone();
                s.insert(o.clone());
                Formula::Or(s)
            }
            _ => {
                let mut s = HashSet::new();
                s.insert(self.clone());
                s.insert(other.clone());
                Formula::Or(s)
            }
        }
    }

    pub fn neg(&self) -> Formula {
        match self {
            Formula::True(v) => Formula::False(v.clone()),
            Formula::False(v) => Formula::True(v.clone()),
            Formula::Neg(f) => (**f).clone(),
            _ => Formula::Neg(Box::new(self.clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::first_order::models::Model;
    use std::collections::HashMap;

    #[test]
    fn test_eq_symmetry() {
        let m = Model::new(vec![0i64, 1], HashMap::new(), HashMap::new());
        let x = Term::Variable(Variable::new("x"));
        let y = Term::Variable(Variable::new("y"));
        let eq_xy = eq(x.clone(), y.clone());
        let eq_yx = eq(y, x);
        let ext_xy = eq_xy.extension(&m, Some(2));
        let ext_yx = eq_yx.extension(&m, Some(2));
        assert_eq!(ext_xy, ext_yx);
    }

    #[test]
    fn test_true_extension_total() {
        let m = Model::new(vec![0i64, 1, 2], HashMap::new(), HashMap::new());
        let f = true_formula(None);
        let ext = f.extension(&m, Some(2));
        let expected: std::collections::HashSet<Vec<i64>> = (0..3)
            .flat_map(|a| (0..3).map(move |b| vec![a, b]))
            .collect();
        assert_eq!(ext, expected);
    }

    #[test]
    fn test_false_extension_vacia() {
        let m = Model::new(vec![0i64, 1], HashMap::new(), HashMap::new());
        let f = false_formula(None);
        let ext = f.extension(&m, Some(2));
        assert!(ext.is_empty());
    }
}
