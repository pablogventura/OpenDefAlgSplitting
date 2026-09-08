//! Faithful port of OpenDefAlgMerging `isOpenDef` (IsoType + orbit propagation).
//!
//! CLI QF `--engine merge` uses this path. Aut-orbit `iso_merge` stays in `morph.rs`.

use crate::engines::dispatch::EngineOutcome;
use crate::engines::tuple_model_hash::TupleModelHash;
use crate::first_order::models::Model;
use crate::first_order::relops::Relation;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
struct Orbit {
    tuples: Vec<Vec<i64>>,
    polarity: Vec<bool>,
    ty: Option<TupleModelHash>,
}

impl Orbit {
    fn merge(self, other: Self) -> Result<Self, ()> {
        if self.polarity != other.polarity {
            return Err(());
        }
        let ty = self.ty.or(other.ty);
        let mut tuples = self.tuples;
        tuples.extend(other.tuples);
        Ok(Self {
            tuples,
            polarity: self.polarity,
            ty,
        })
    }
}

#[derive(Debug)]
struct TypePartition {
    universe: Vec<i64>,
    arity: usize,
    orbits: HashMap<Vec<i64>, Orbit>,
    types: Vec<(TupleModelHash, Vec<Vec<i64>>)>,
}

impl TypePartition {
    fn new(universe: &[i64], arity: usize, targets: &[&Relation]) -> Self {
        let mut orbits = HashMap::new();
        for t in permutations_star(universe, arity) {
            let polarity: Vec<bool> = targets.iter().map(|tg| tg.contains(&t)).collect();
            orbits.insert(
                t.clone(),
                Orbit {
                    tuples: vec![t],
                    polarity,
                    ty: None,
                },
            );
        }
        Self {
            universe: universe.to_vec(),
            arity,
            orbits,
            types: Vec::new(),
        }
    }

    fn get_orbit_key(&self, tup: &[i64]) -> Option<Vec<i64>> {
        for (rep, orb) in &self.orbits {
            if orb.tuples.iter().any(|x| x.as_slice() == tup) {
                return Some(rep.clone());
            }
        }
        None
    }

    fn set_type(&mut self, tup: &[i64], ty: TupleModelHash) {
        debug_assert!(self.types.iter().all(|(h, _)| h != &ty));
        let key = self.get_orbit_key(tup).expect("orbit for tuple");
        let orb = self.orbits.get_mut(&key).expect("orbit");
        orb.ty = Some(ty.clone());
        self.types.push((ty, orb.tuples.clone()));
    }

    fn has_known_type(&self, tup: &[i64]) -> bool {
        self.get_type(tup).is_some()
    }

    fn get_type(&self, tup: &[i64]) -> Option<&TupleModelHash> {
        for (h, members) in &self.types {
            if members.iter().any(|x| x.as_slice() == tup) {
                return Some(h);
            }
        }
        None
    }

    fn find_stored_type(&self, ty: &TupleModelHash) -> Option<&TupleModelHash> {
        self.types.iter().find(|(h, _)| h == ty).map(|(h, _)| h)
    }

    fn del_orbit_containing(&mut self, tup: &[i64]) -> Orbit {
        let key = self.get_orbit_key(tup).expect("orbit");
        self.orbits.remove(&key).expect("orbit present")
    }

    fn unir(&mut self, t1: &[i64], t2: &[i64]) -> Result<(), ()> {
        if t1 == t2 {
            return Ok(());
        }
        let k1 = self.get_orbit_key(t1).expect("o1");
        let k2 = self.get_orbit_key(t2).expect("o2");
        if k1 == k2 {
            return Ok(());
        }
        let o1 = self.del_orbit_containing(t1);
        let o2 = self.del_orbit_containing(t2);
        let union = o1.merge(o2)?;
        if let Some(ref ty) = union.ty {
            if let Some(slot) = self.types.iter_mut().find(|(h, _)| h == ty) {
                slot.1 = union.tuples.clone();
            }
        }
        self.orbits.insert(t1.to_vec(), union);
        Ok(())
    }

    fn propagar(&mut self, gamma: &HashMap<i64, i64>) -> Result<(), ()> {
        let tuples: Vec<Vec<i64>> = permutations_star(&self.universe, self.arity);
        for t in tuples {
            let tp: Option<Vec<i64>> = t.iter().map(|x| gamma.get(x).copied()).collect();
            if let Some(ref tp) = tp {
                self.unir(&t, tp)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
struct MicroPartition {
    entries: Vec<(TupleModelHash, Vec<i64>)>,
}

impl MicroPartition {
    fn contains_type(&self, h: &TupleModelHash) -> bool {
        self.entries.iter().any(|(ty, _)| ty == h)
    }

    fn representative_type(&self, h: &TupleModelHash) -> Option<&TupleModelHash> {
        self.entries
            .iter()
            .find(|(ty, _)| ty == h)
            .map(|(ty, _)| ty)
    }

    fn new_type(&mut self, t: Vec<i64>, h: TupleModelHash) {
        self.entries.push((h, t));
    }
}

struct StackFrame {
    pending: VecDeque<Vec<i64>>,
    micros: HashMap<usize, MicroPartition>,
}

/// Soft safety for injective enumeration `|U|! / (|U|-k)!`.
pub const MEGAHIT_MAX_INJECTIVE: usize = 200_000;

fn permutations_star(universe: &[i64], arity: usize) -> Vec<Vec<i64>> {
    let n = universe.len();
    let mut out = Vec::new();
    if arity == 0 {
        out.push(vec![]);
        return out;
    }
    if arity > n {
        return out;
    }
    let mut comb: Vec<usize> = (0..arity).collect();
    loop {
        let subset: Vec<i64> = comb.iter().map(|&i| universe[i]).collect();
        out.extend(permute_vec(&subset));
        let mut i = arity;
        loop {
            if i == 0 {
                return out;
            }
            i -= 1;
            if comb[i] < n - arity + i {
                comb[i] += 1;
                for j in i + 1..arity {
                    comb[j] = comb[j - 1] + 1;
                }
                break;
            }
        }
    }
}

fn permute_vec(items: &[i64]) -> Vec<Vec<i64>> {
    let mut items = items.to_vec();
    let mut out = Vec::new();
    fn rec(a: &mut [i64], i: usize, out: &mut Vec<Vec<i64>>) {
        if i == a.len() {
            out.push(a.to_vec());
            return;
        }
        for j in i..a.len() {
            a.swap(i, j);
            rec(a, i + 1, out);
            a.swap(i, j);
        }
    }
    rec(&mut items, 0, &mut out);
    out
}

fn injective_count(n: usize, k: usize) -> Option<usize> {
    if k > n {
        return Some(0);
    }
    let mut v = 1usize;
    for i in 0..k {
        v = v.checked_mul(n - i)?;
    }
    Some(v)
}

fn pending_for_spectrum(universe: &[i64], spectrum: &[usize]) -> VecDeque<Vec<i64>> {
    let mut q = VecDeque::new();
    for &e in spectrum {
        for t in permutations_star(universe, e) {
            q.push_back(t);
        }
    }
    q
}

fn sort_targets(targets: &[&Relation]) -> Vec<Relation> {
    let mut owned: Vec<Relation> = targets.iter().map(|t| (*t).clone()).collect();
    owned.sort_by(|a, b| b.arity.cmp(&a.arity).then_with(|| a.sym.cmp(&b.sym)));
    owned
}

/// QF merge: faithful `isOpenDef` over all targets jointly.
pub fn check_qf_megahit_merge(model: &Model, target: &Relation) -> Result<EngineOutcome, String> {
    check_qf_megahit_merge_multi(model, &[target])
}

pub fn check_qf_megahit_merge_multi(
    model: &Model,
    targets: &[&Relation],
) -> Result<EngineOutcome, String> {
    if targets.is_empty() {
        return Ok(EngineOutcome::basic(true, "qf", "megahit_merge"));
    }
    // Historical main.py deletes all T* before IsoType so R ignores targets.
    let mut ambient = model.clone();
    ambient.relations.retain(|sym, _| !sym.starts_with('T'));
    let sorted = sort_targets(targets);
    let tgs: Vec<&Relation> = sorted.iter().collect();
    match is_open_def_merging(&ambient, &tgs)? {
        true => Ok(EngineOutcome::basic(true, "qf", "megahit_merge")),
        false => Ok(EngineOutcome::basic(false, "qf", "megahit_merge")),
    }
}

/// `Ok(true)` definable, `Ok(false)` counterexample polarity, `Err` on hard limits.
fn is_open_def_merging(ambient: &Model, tgs: &[&Relation]) -> Result<bool, String> {
    let mut spectrum: Vec<usize> = tgs
        .iter()
        .map(|t| t.arity)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    spectrum.sort_unstable_by(|a, b| b.cmp(a));

    for &e in &spectrum {
        let n = ambient.universe.len();
        let count = injective_count(n, e).unwrap_or(usize::MAX);
        if count > MEGAHIT_MAX_INJECTIVE {
            return Err(format!(
                "megahit: P(|U|,{e}) = {count} exceeds MAX={MEGAHIT_MAX_INJECTIVE}"
            ));
        }
    }

    let mut os: HashMap<usize, TypePartition> = HashMap::new();
    for &e in &spectrum {
        os.insert(e, TypePartition::new(&ambient.universe, e, tgs));
    }

    let mut stack: Vec<StackFrame> = vec![StackFrame {
        pending: pending_for_spectrum(&ambient.universe, &spectrum),
        micros: spectrum
            .iter()
            .map(|&e| (e, MicroPartition::default()))
            .collect(),
    }];

    while let Some(mut frame) = stack.pop() {
        while let Some(t) = frame.pending.pop_front() {
            let arity = t.len();
            let r = frame.micros.entry(arity).or_default();
            let o = os.get_mut(&arity).expect("partition for arity");
            if o.has_known_type(&t) {
                continue;
            }
            // Historical: always TupleModelHash(A, t) with ambient A on the stack tip.
            let h = TupleModelHash::compute(ambient, &t);
            let u = h.universe();
            if u.len() == ambient.universe.len() {
                if r.contains_type(&h) {
                    let rep = r.representative_type(&h).expect("rep").clone();
                    let gamma = h.iso_map(&rep).expect("iso");
                    for part in os.values_mut() {
                        if part.propagar(&gamma).is_err() {
                            return Ok(false);
                        }
                    }
                } else {
                    o.set_type(&t, h.clone());
                    r.new_type(t, h);
                }
            } else if let Some(stored) = o.find_stored_type(&h).cloned() {
                let gamma = stored.iso_map(&h).expect("subiso");
                for part in os.values_mut() {
                    if part.propagar(&gamma).is_err() {
                        return Ok(false);
                    }
                }
            } else {
                let resume = StackFrame {
                    pending: frame.pending.clone(),
                    micros: frame.micros.clone(),
                };
                o.set_type(&t, h.clone());

                let mut mps: HashMap<usize, MicroPartition> = HashMap::new();
                let mut mp0 = MicroPartition::default();
                mp0.new_type(t.clone(), h.clone());
                mps.insert(arity, mp0);
                for &e in &spectrum {
                    mps.entry(e).or_default();
                }
                let mut sub_u: Vec<i64> = h.universe().into_iter().collect();
                sub_u.sort_unstable();
                let smaller = StackFrame {
                    pending: pending_for_spectrum(&sub_u, &spectrum),
                    micros: mps,
                };
                stack.push(resume);
                stack.push(smaller);
                break;
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::dispatch::{check_engine, EngineKind, FragmentKind};
    use crate::parser::parse_model;
    use std::path::Path;

    fn merge_definable(path: &Path) -> bool {
        let mut model = parse_model(Some(path), true).expect("parse");
        let targets: Vec<_> = model
            .relations
            .keys()
            .filter(|s| s.starts_with('T'))
            .cloned()
            .collect();
        let mut tgs = Vec::new();
        for s in targets {
            tgs.push(model.relations.remove(&s).expect("T"));
        }
        let refs: Vec<&Relation> = tgs.iter().collect();
        check_qf_megahit_merge_multi(&model, &refs)
            .map(|o| o.definable)
            .unwrap_or(false)
    }

    #[test]
    fn megahit_suma4_not_definable() {
        assert!(!merge_definable(Path::new("model_examples/suma4.model")));
    }

    #[test]
    fn megahit_retrombo_nodef() {
        assert!(!merge_definable(Path::new(
            "model_examples/retrombo_nodef.model"
        )));
    }

    #[test]
    fn megahit_msimple_definable() {
        assert!(merge_definable(Path::new("model_examples/msimple.model")));
    }

    #[test]
    fn megahit_via_dispatch_merge() {
        let path = Path::new("model_examples/msimple.model");
        let model = parse_model(Some(path), true).expect("parse");
        let target = model
            .relations
            .values()
            .find(|r| r.sym.starts_with('T'))
            .expect("T")
            .clone();
        let out = check_engine(
            &model,
            &target,
            FragmentKind::Qf,
            EngineKind::Merge,
            2,
            1,
        )
        .expect("engine");
        assert_eq!(out.engine, "megahit_merge");
        assert!(out.definable);
    }
}
