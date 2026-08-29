use std::collections::{HashMap, HashSet};

use crate::stone::discriminator::discriminator;
use crate::stone::spec::{Elem, PartialIso, StoneSpec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoneOp {
    Discriminator,
    FBar { bar: Vec<Elem>, b: Elem },
    GD { domain: Vec<Elem> },
}

impl StoneOp {
    pub fn arity(&self) -> usize {
        match self {
            StoneOp::Discriminator => 3,
            StoneOp::FBar { bar, .. } => bar.len(),
            StoneOp::GD { domain } => domain.len(),
        }
    }

    pub fn label(&self) -> String {
        match self {
            StoneOp::Discriminator => "discriminator".into(),
            StoneOp::FBar { bar, b } => format!("f_[{}]_{}", bar.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "), b),
            StoneOp::GD { domain } => {
                let mut d = domain.clone();
                d.sort_unstable();
                format!("g_[{}]", d.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
            }
        }
    }
}

fn args_match_iso(bar: &[Elem], args: &[Elem], iso: &crate::stone::spec::PartialIso) -> bool {
    bar.len() == args.len()
        && bar.iter().all(|&x| iso.dom.contains(&x))
        && bar
            .iter()
            .zip(args)
            .all(|(&x, &a)| iso.apply(x) == a)
}

pub fn eval_op(spec: &StoneSpec, op: &StoneOp, args: &[Elem]) -> Elem {
    match op {
        StoneOp::Discriminator => {
            if args.len() >= 3 {
                discriminator(args[0], args[1], args[2])
            } else {
                args.first().copied().unwrap_or(0)
            }
        }
        StoneOp::FBar { bar, b } => {
            if let Some(iso) = spec
                .sub_isos
                .iter()
                .find(|iso| args_match_iso(bar, args, iso))
            {
                iso.apply(*b)
            } else {
                args.first().copied().unwrap_or(0)
            }
        }
        StoneOp::GD { domain } => {
            if spec
                .sub_isos
                .iter()
                .any(|iso| args_match_iso(domain, args, iso))
            {
                args.first().copied().unwrap_or(0)
            } else {
                args.get(1)
                    .copied()
                    .unwrap_or_else(|| args.first().copied().unwrap_or(0))
            }
        }
    }
}

/// All tuples of length `arity` with entries in `sub` (includes `[[]]` if arity = 0).
fn arg_tuples(sub: &HashSet<Elem>, arity: usize) -> Vec<Vec<Elem>> {
    if arity == 0 {
        return vec![vec![]];
    }
    let mut elems: Vec<Elem> = sub.iter().copied().collect();
    elems.sort_unstable();
    if elems.is_empty() {
        return vec![];
    }
    let mut out = vec![vec![]];
    for _ in 0..arity {
        let mut next = Vec::new();
        for prefix in &out {
            for &x in &elems {
                let mut row = prefix.clone();
                row.push(x);
                next.push(row);
            }
        }
        out = next;
    }
    out
}

/// Full preservation check: every tuple from U evaluates inside U.
pub fn preserves_op_full(spec: &StoneSpec, op: &StoneOp, sub: &HashSet<Elem>) -> bool {
    arg_tuples(sub, op.arity())
        .into_iter()
        .all(|args| sub.contains(&eval_op(spec, op, &args)))
}

/// Legacy weak check (constant min tuple) kept for older tests.
pub fn preserves_op(spec: &StoneSpec, op: &StoneOp, sub: &HashSet<Elem>) -> bool {
    if sub.is_empty() {
        return true;
    }
    let mut elems: Vec<Elem> = sub.iter().copied().collect();
    elems.sort_unstable();
    let a0 = elems[0];
    let args = vec![a0; op.arity()];
    sub.contains(&eval_op(spec, op, &args))
}

pub fn not_preserves_sub_sq(spec: &StoneSpec, op: &StoneOp, subs: &[HashSet<Elem>]) -> bool {
    subs.iter().any(|sub| !preserves_op(spec, op, sub))
}

pub fn fails_preserve_on(spec: &StoneSpec, op: &StoneOp, u: &HashSet<Elem>) -> bool {
    !preserves_op_full(spec, op, u)
}

/// Subuniverses of the current operation list (Alg. 1 incremental).
/// Includes the empty set (needed so nullary `f` constants are forced).
pub fn subuniverses_of_ops(spec: &StoneSpec, ops: &[StoneOp]) -> Vec<HashSet<Elem>> {
    let n = spec.universe.len();
    let mask_count = 1usize << n;
    (0..mask_count)
        .filter_map(|mask| {
            let s: HashSet<Elem> = (0..n).filter(|i| (mask >> i) & 1 == 1).collect();
            if ops.iter().all(|op| preserves_op_full(spec, op, &s)) {
                Some(s)
            } else {
                None
            }
        })
        .collect()
}

pub fn not_preserves_current(spec: &StoneSpec, op: &StoneOp, ops: &[StoneOp]) -> bool {
    subuniverses_of_ops(spec, ops)
        .iter()
        .any(|u| fails_preserve_on(spec, op, u))
}

fn sorted_elems(s: &HashSet<Elem>) -> Vec<Elem> {
    let mut v: Vec<Elem> = s.iter().copied().collect();
    v.sort_unstable();
    v
}

/// `op` no preserva el grafo del subiso parcial `gamma` (Alg. 2).
pub fn fails_preserve_iso(spec: &StoneSpec, op: &StoneOp, gamma: &PartialIso) -> bool {
    arg_tuples(&gamma.dom, op.arity()).into_iter().any(|args| {
        let ev = eval_op(spec, op, &args);
        let mapped: Vec<Elem> = args.iter().map(|&x| gamma.apply(x)).collect();
        !gamma.dom.contains(&ev) || gamma.apply(ev) != eval_op(spec, op, &mapped)
    })
}

fn ops_preserve_iso_b(spec: &StoneSpec, ops: &[StoneOp], gamma: &PartialIso) -> bool {
    ops.iter().all(|op| !fails_preserve_iso(spec, op, gamma))
}

/// Subiso parcial por listas posicionales (identidad fuera del dominio).
pub fn partial_iso_of_lists(dom_list: &[Elem], cod_list: &[Elem]) -> PartialIso {
    if dom_list.len() != cod_list.len() {
        return PartialIso::id_on(&dom_list.iter().copied().collect());
    }
    let mut seen_d = HashSet::new();
    let mut seen_c = HashSet::new();
    for &x in dom_list {
        if !seen_d.insert(x) {
            return PartialIso::id_on(&dom_list.iter().copied().collect());
        }
    }
    for &y in cod_list {
        if !seen_c.insert(y) {
            return PartialIso::id_on(&dom_list.iter().copied().collect());
        }
    }
    let map: HashMap<Elem, Elem> = dom_list
        .iter()
        .zip(cod_list.iter())
        .map(|(&x, &y)| (x, y))
        .collect();
    PartialIso {
        dom: seen_d,
        cod: seen_c,
        map,
    }
}

fn permutations(items: &[Elem]) -> Vec<Vec<Elem>> {
    if items.is_empty() {
        return vec![vec![]];
    }
    let mut out = Vec::new();
    fn rec(rest: &mut Vec<Elem>, prefix: &mut Vec<Elem>, out: &mut Vec<Vec<Elem>>) {
        if rest.is_empty() {
            out.push(prefix.clone());
            return;
        }
        for i in 0..rest.len() {
            let x = rest.remove(i);
            prefix.push(x);
            rec(rest, prefix, out);
            let x = prefix.pop().expect("prefix non-empty");
            rest.insert(i, x);
        }
    }
    rec(&mut items.to_vec(), &mut Vec::new(), &mut out);
    out
}

/// Candidatos a subiso entre subuniversos de igual cardinalidad.
pub fn candidate_isos(subs: &[HashSet<Elem>]) -> Vec<PartialIso> {
    let mut out = Vec::new();
    for u in subs {
        let u_list = sorted_elems(u);
        for v in subs {
            if u.len() != v.len() {
                continue;
            }
            let v_list = sorted_elems(v);
            for vs in permutations(&v_list) {
                out.push(partial_iso_of_lists(&u_list, &vs));
            }
        }
    }
    out
}

/// Subisos del algebra actual: candidatos que respetan el grafo de todas las ops.
pub fn current_sub_isos(spec: &StoneSpec, ops: &[StoneOp]) -> Vec<PartialIso> {
    let subs = subuniverses_of_ops(spec, ops);
    candidate_isos(&subs)
        .into_iter()
        .filter(|gamma| ops_preserve_iso_b(spec, ops, gamma))
        .collect()
}

/// Alg. 2 incremental: fallo 1D o fallo en grafo de algun subiso actual.
pub fn not_preserves_sub_sq_current(spec: &StoneSpec, op: &StoneOp, ops: &[StoneOp]) -> bool {
    not_preserves_current(spec, op, ops)
        || current_sub_isos(spec, ops)
            .iter()
            .any(|gamma| fails_preserve_iso(spec, op, gamma))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stone::spec::StoneSpec;
    use std::collections::HashSet;

    #[test]
    fn discriminator_preserves_singleton() {
        let json = include_str!("../../../qfdef/test/fixtures/stone/fin2_valid.json");
        let spec = StoneSpec::parse_json(json).expect("parse");
        let mut t = HashSet::new();
        t.insert(0);
        assert!(preserves_op_full(
            &spec,
            &StoneOp::Discriminator,
            &t
        ));
    }
}
