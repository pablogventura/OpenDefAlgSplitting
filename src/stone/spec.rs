use std::collections::{HashMap, HashSet};

use serde::Deserialize;

pub type Elem = usize;

#[derive(Debug, Clone, Deserialize)]
pub struct PartialIsoJson {
    pub dom: Vec<Elem>,
    pub cod: Vec<Elem>,
    pub map: HashMap<String, Elem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StoneSpecJson {
    pub universe_size: usize,
    pub closure_sets: Vec<Vec<Elem>>,
    #[serde(default)]
    pub sub_isos: Vec<PartialIsoJson>,
    #[serde(default)]
    pub id_on_family: bool,
}

#[derive(Debug, Clone)]
pub struct PartialIso {
    pub dom: HashSet<Elem>,
    pub cod: HashSet<Elem>,
    pub map: HashMap<Elem, Elem>,
}

impl PartialIso {
    pub fn id_on(members: &HashSet<Elem>) -> Self {
        Self {
            dom: members.clone(),
            cod: members.clone(),
            map: members.iter().copied().map(|x| (x, x)).collect(),
        }
    }

    pub fn apply(&self, x: Elem) -> Elem {
        self.map.get(&x).copied().unwrap_or(x)
    }
}

#[derive(Debug, Clone)]
pub struct StoneSpec {
    pub universe: Vec<Elem>,
    pub closure_sets: Vec<HashSet<Elem>>,
    pub sub_isos: Vec<PartialIso>,
}

impl StoneSpec {
    pub fn from_json(raw: StoneSpecJson) -> Result<Self, String> {
        if raw.universe_size == 0 {
            return Err("universe_size must be positive".into());
        }
        let universe: Vec<Elem> = (0..raw.universe_size).collect();
        let closure_sets: Vec<HashSet<Elem>> = raw
            .closure_sets
            .iter()
            .map(|s| s.iter().copied().collect())
            .collect();
        let sub_isos = if raw.id_on_family {
            Self::id_on_powerset(raw.universe_size)
        } else {
            raw.sub_isos
                .iter()
                .map(|iso| {
                    let dom: HashSet<Elem> = iso.dom.iter().copied().collect();
                    let cod: HashSet<Elem> = iso.cod.iter().copied().collect();
                    let map = iso
                        .map
                        .iter()
                        .filter_map(|(k, v)| k.parse::<Elem>().ok().map(|key| (key, *v)))
                        .collect();
                    PartialIso { dom, cod, map }
                })
                .collect()
        };
        Ok(Self {
            universe,
            closure_sets,
            sub_isos,
        })
    }

    fn id_on_powerset(universe_size: usize) -> Vec<PartialIso> {
        let n = universe_size;
        let mask_count = 1usize << n;
        (0..mask_count)
            .map(|mask| {
                let members: HashSet<Elem> = (0..n).filter(|i| (mask >> i) & 1 == 1).collect();
                PartialIso::id_on(&members)
            })
            .collect()
    }

    pub fn parse_json(text: &str) -> Result<Self, String> {
        let raw: StoneSpecJson = serde_json::from_str(text).map_err(|e| e.to_string())?;
        Self::from_json(raw)
    }

    pub fn algebraic_closure(&self, tuple: &[Elem]) -> HashSet<Elem> {
        let tuple_set: HashSet<Elem> = tuple.iter().copied().collect();
        let candidates: Vec<&HashSet<Elem>> = self
            .closure_sets
            .iter()
            .filter(|s| tuple_set.is_subset(s))
            .collect();
        if let Some(first) = candidates.first() {
            candidates
                .iter()
                .skip(1)
                .fold((*first).clone(), |acc, s| acc.intersection(s).copied().collect())
        } else {
            tuple_set
        }
    }

    pub fn all_nonempty_subsets(&self) -> Vec<HashSet<Elem>> {
        let n = self.universe.len();
        let mask_count = 1usize << n;
        (0..mask_count)
            .filter_map(|mask| {
                let s: HashSet<Elem> = (0..n).filter(|i| (mask >> i) & 1 == 1).collect();
                if s.is_empty() { None } else { Some(s) }
            })
            .collect()
    }

    pub fn nonempty_subuniverses(&self) -> Vec<HashSet<Elem>> {
        self.all_nonempty_subsets()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_id_on_family() {
        let json = r#"{
            "universe_size": 2,
            "closure_sets": [[], [0], [1], [0, 1]],
            "id_on_family": true
        }"#;
        let spec = StoneSpec::parse_json(json).expect("parse");
        assert_eq!(spec.sub_isos.len(), 4);
    }
}
