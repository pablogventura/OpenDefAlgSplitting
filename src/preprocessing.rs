use std::collections::{BTreeSet, HashMap, HashSet};

use crate::first_order::formulas::{self, Formula, Variable};
use crate::first_order::relops::Relation;

/// Pattern for tuple equality classes
#[derive(Debug, Clone)]
pub struct Pattern {
    pub tuple: Vec<i64>,
    pub pruned_tuple: Vec<i64>,
    pub pattern: HashSet<BTreeSet<usize>>,
}

impl Pattern {
    pub fn new(tuple: Vec<i64>) -> Self {
        let mut pattern: HashMap<i64, BTreeSet<usize>> = HashMap::new();
        let mut pruned_list = Vec::new();
        for (i, &a) in tuple.iter().enumerate() {
            pattern.entry(a).or_default().insert(i);
            if pattern.get(&a).map(|s| s.len()).unwrap_or(0) == 1 {
                pruned_list.push(a);
            }
        }
        let pattern_set: HashSet<BTreeSet<usize>> =
            pattern.values().cloned().collect();
        Self {
            tuple,
            pruned_tuple: pruned_list,
            pattern: pattern_set,
        }
    }

    pub fn name(&self) -> String {
        let mut result = String::from("|");
        let mut classes: Vec<_> = self.pattern.iter().collect();
        classes.sort_by(|a, b| a.iter().next().cmp(&b.iter().next()));
        for cls in classes {
            result.push_str(
                &cls.iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            );
            result.push('|');
        }
        result.push_str(&format!("a{}", self.pruned_tuple.len()));
        result
    }

    pub fn preprocessed_formula(&self) -> Formula {
        let vs = formulas::variables(
            &(0..self.tuple.len()).map(|i| i as i32).collect::<Vec<_>>(),
        );
        let mut representatives: Vec<usize> = self
            .pattern
            .iter()
            .map(|cls| *cls.iter().next().unwrap())
            .collect();
        representatives.sort_unstable();
        let vs: Vec<Variable> = representatives.iter().map(|&i| vs[i].clone()).collect();
        let mut f = formulas::true_formula(Some(vs.iter().cloned().collect()));
        if vs.len() == 1 {
            f = f.and_formula(&formulas::eq(
                formulas::Term::Variable(vs[0].clone()),
                formulas::Term::Variable(vs[0].clone()),
            ));
        }
        for (i, v) in vs.iter().enumerate() {
            for (j, w) in vs.iter().enumerate() {
                if i != j {
                    f = f.and_formula(
                        &formulas::eq(
                            formulas::Term::Variable(v.clone()),
                            formulas::Term::Variable(w.clone()),
                        )
                        .neg(),
                    );
                }
            }
        }
        f
    }

    pub fn postprocessed_formula(&self) -> Formula {
        let vs = formulas::variables(
            &(0..self.tuple.len()).map(|i| i as i32).collect::<Vec<_>>(),
        );
        let mut differents: Vec<Variable> = Vec::new();
        let mut f = formulas::true_formula(None);
        let mut classes: Vec<_> = self.pattern.iter().collect();
        classes.sort_by(|a, b| a.iter().next().cmp(&b.iter().next()));
        for cls in classes {
            let cls: Vec<usize> = cls.iter().copied().collect();
            for w in cls.windows(2) {
                f = f.and_formula(&formulas::eq(
                    formulas::Term::Variable(vs[w[0]].clone()),
                    formulas::Term::Variable(vs[w[1]].clone()),
                ));
            }
            let repr = vs[*cls.iter().min().unwrap()].clone();
            for other in &differents {
                f = f.and_formula(
                    &formulas::eq(
                        formulas::Term::Variable(repr.clone()),
                        formulas::Term::Variable(other.clone()),
                    )
                    .neg(),
                );
            }
            differents.push(repr);
        }
        f
    }
}

impl PartialEq for Pattern {
    fn eq(&self, other: &Self) -> bool {
        self.pattern == other.pattern
    }
}
impl Eq for Pattern {}

impl std::hash::Hash for Pattern {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut v: Vec<_> = self.pattern.iter().collect();
        v.sort();
        for p in v {
            for x in p {
                x.hash(state);
            }
        }
    }
}

#[allow(dead_code)]
fn quotient<K: Eq + std::hash::Hash + Clone, V: Clone, F: Fn(&V) -> K>(
    s: &HashSet<V>,
    f: F,
) -> HashMap<K, Vec<V>> {
    let mut result: HashMap<K, Vec<V>> = HashMap::new();
    for a in s {
        let k = f(a);
        result.entry(k).or_default().push(a.clone());
    }
    result
}

#[allow(dead_code)]
fn limpia(t: &[i64]) -> Vec<usize> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for (i, e) in t.iter().enumerate() {
        let first_idx = t.iter().position(|x| x == e).unwrap();
        seen.insert(first_idx);
        if first_idx == i {
            result.push(i);
        }
    }
    result.sort_unstable();
    result
}

pub fn preprocesamiento2(target: &Relation) -> Vec<Relation> {
    let mut pruned_relations: HashMap<Pattern, Vec<Vec<i64>>> = HashMap::new();
    for t in &target.r {
        let pattern = Pattern::new(t.clone());
        pruned_relations
            .entry(pattern.clone())
            .or_default()
            .push(pattern.pruned_tuple.clone());
    }
    let mut result = Vec::new();
    for (pattern, tuples) in pruned_relations {
        let first_tuple = tuples.first().unwrap();
        let arity = first_tuple.len();
        let mut r = Relation::new(
            format!("{}{}", target.sym, pattern.name()),
            arity,
        );
        for t in tuples {
            r.add(t);
        }
        r.pattern = Some(Box::new(pattern));
        r.superrel_sym = Some(target.sym.clone());
        r.superrel = Some(Box::new(target.clone()));
        result.push(r);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::first_order::relops::Relation;

    #[test]
    fn test_preprocesamiento_un_patron() {
        let mut r = Relation::new("T0", 2);
        r.add(vec![0, 1]);
        r.add(vec![1, 0]);
        let preps = preprocesamiento2(&r);
        assert!(!preps.is_empty());
    }

    #[test]
    fn test_preprocesamiento_varios_patrones() {
        let mut r = Relation::new("T0", 2);
        r.add(vec![0, 0]);
        r.add(vec![1, 1]);
        r.add(vec![0, 1]);
        let preps = preprocesamiento2(&r);
        assert!(!preps.is_empty());
    }

    #[test]
    fn test_pattern_pre_post_formula() {
        let t = vec![0, 1, 1];
        let p = Pattern::new(t);
        let pre = p.preprocessed_formula();
        let post = p.postprocessed_formula();
        assert!(!pre.implied_declaration().is_empty() || pre.free_vars().is_empty());
        assert!(!post.implied_declaration().is_empty() || post.free_vars().is_empty());
    }
}
