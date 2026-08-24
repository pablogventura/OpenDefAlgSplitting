use std::collections::{BTreeMap, HashMap, HashSet};

use rayon::prelude::*;

use crate::first_order::formulas::{self, Formula, OpSym, Term, Variable};
use crate::first_order::models::Model;
use crate::first_order::relops::{Operation, Relation};

/// Orden de exploración del árbol de bloques.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExploreOrder {
    /// DFS paralelo (Rayon), comportamiento histórico.
    Dfs,
    /// BFS iterativo con cola.
    Bfs,
}

/// Configuración del algoritmo HIT / splitting.
#[derive(Clone, Copy, Debug)]
pub struct HitConfig {
    /// Si true, en cada paso se elige (op, ti) que maximiza information gain.
    pub use_information_gain: bool,
    /// Si use_information_gain, número de candidatos a muestrear (None = todos).
    pub ig_sample: Option<usize>,
    /// Si true, aplica de verdad el mejor candidato IG (experimental; puede cambiar veredictos).
    pub ig_experimental: bool,
    /// No aplicar candidatos que no parten el bloque ni agregan valores nuevos.
    pub skip_useless_candidates: bool,
    /// Precheck: si T no es unión de clases ≈ aproximadas (subálgebra etiquetada), NOT DEFINABLE.
    pub approx_precheck: bool,
    /// Emitir todas las constantes 0-arias al inicio (ya es el comportamiento del generador; flag documentado).
    pub emit_all_constants: bool,
    /// Tope de pasos de splitting (None = sin tope). Si se excede, se trata como no definible.
    pub max_steps: Option<u64>,
    /// Simplificar la fórmula resultado (True/False redundantes).
    pub simplify_formula: bool,
    /// DFS (default) o BFS.
    pub explore_order: ExploreOrder,
}

impl Default for HitConfig {
    fn default() -> Self {
        Self {
            use_information_gain: false,
            ig_sample: Some(20),
            ig_experimental: false,
            skip_useless_candidates: true,
            approx_precheck: false,
            emit_all_constants: true,
            max_steps: None,
            simplify_formula: false,
            explore_order: ExploreOrder::Dfs,
        }
    }
}

/// Contadores globales de una corrida (atómicos para Rayon).
pub struct HitRunStats {
    pub steps: std::sync::atomic::AtomicU64,
    pub candidates_considered: std::sync::atomic::AtomicU64,
    pub candidates_skipped: std::sync::atomic::AtomicU64,
    pub approx_reject: std::sync::atomic::AtomicU64,
}

impl HitRunStats {
    pub fn new() -> Self {
        Self {
            steps: std::sync::atomic::AtomicU64::new(0),
            candidates_considered: std::sync::atomic::AtomicU64::new(0),
            candidates_skipped: std::sync::atomic::AtomicU64::new(0),
            approx_reject: std::sync::atomic::AtomicU64::new(0),
        }
    }
    pub fn snapshot(&self) -> (u64, u64, u64, u64) {
        use std::sync::atomic::Ordering::Relaxed;
        (
            self.steps.load(Relaxed),
            self.candidates_considered.load(Relaxed),
            self.candidates_skipped.load(Relaxed),
            self.approx_reject.load(Relaxed),
        )
    }
}

static RUN_STATS: HitRunStats = HitRunStats {
    steps: std::sync::atomic::AtomicU64::new(0),
    candidates_considered: std::sync::atomic::AtomicU64::new(0),
    candidates_skipped: std::sync::atomic::AtomicU64::new(0),
    approx_reject: std::sync::atomic::AtomicU64::new(0),
};

pub fn reset_run_stats() {
    use std::sync::atomic::Ordering::Relaxed;
    RUN_STATS.steps.store(0, Relaxed);
    RUN_STATS.candidates_considered.store(0, Relaxed);
    RUN_STATS.candidates_skipped.store(0, Relaxed);
    RUN_STATS.approx_reject.store(0, Relaxed);
}

pub fn run_stats_snapshot() -> (u64, u64, u64, u64) {
    RUN_STATS.snapshot()
}

fn entropy(in_count: i64, out_count: i64) -> f64 {
    let n = in_count + out_count;
    if n == 0 {
        return 0.0;
    }
    let p = in_count as f64 / n as f64;
    let q = out_count as f64 / n as f64;
    let h = |x: f64| if x > 0.0 { -x * x.log2() } else { 0.0 };
    h(p) + h(q)
}

pub(crate) fn information_gain_from_counts(
    n: usize,
    in_total: usize,
    partition_counts: &HashMap<(usize, bool), (usize, usize)>,
) -> f64 {
    if n == 0 {
        return 0.0;
    }
    let out_total = n - in_total;
    let h_before = entropy(in_total as i64, out_total as i64);
    let mut h_after = 0.0;
    for (_, (g_in, g_out)) in partition_counts {
        let gs = g_in + g_out;
        h_after += (gs as f64 / n as f64) * entropy(*g_in as i64, *g_out as i64);
    }
    h_before - h_after
}

#[derive(Clone, Debug)]
pub struct Counterexample(pub Vec<Vec<i64>>);

fn cartesian_product_indices(pool: &[usize], n: usize, forced: &HashSet<usize>) -> Vec<Vec<usize>> {
    if n == 0 {
        return vec![vec![]];
    }
    let mut result = Vec::new();
    let mut stack = vec![vec![]];
    while let Some(current) = stack.pop() {
        if current.len() == n {
            if current.iter().any(|i| forced.contains(i)) {
                result.push(current);
            }
        } else {
            for &p in pool {
                let mut next = current.clone();
                next.push(p);
                stack.push(next);
            }
        }
    }
    result
}

#[derive(Clone, Debug)]
pub struct TupleHistory {
    pub t: Vec<i64>,
    pub history: Vec<i64>,
    index_map: HashMap<i64, usize>,
    pub in_target: bool,
    pub has_generated: bool,
}

impl TupleHistory {
    pub fn new(t: Vec<i64>, targets: &[&Relation]) -> Self {
        let history = t.clone();
        let mut index_map = HashMap::new();
        for (i, &x) in t.iter().enumerate() {
            index_map.insert(x, i);
        }
        let in_target = targets.iter().all(|tg| {
            if tg.arity == t.len() {
                tg.contains(&t)
            } else {
                true
            }
        });
        Self {
            t,
            history,
            index_map,
            in_target,
            has_generated: false,
        }
    }

    pub fn step(&mut self, op: &Operation, ti: &[usize]) -> usize {
        let args: Vec<i64> = ti.iter().map(|&i| self.history[i]).collect();
        let x = op.call(&args).unwrap();
        if let Some(&xi) = self.index_map.get(&x) {
            self.has_generated = false;
            return xi;
        }
        let idx = self.history.len();
        self.history.push(x);
        self.index_map.insert(x, idx);
        self.has_generated = true;
        idx
    }

    pub fn simulate_step(&self, op: &Operation, ti: &[usize]) -> (usize, bool) {
        let args: Vec<i64> = ti.iter().map(|&i| self.history[i]).collect();
        let x = op.call(&args).unwrap();
        if let Some(&xi) = self.index_map.get(&x) {
            return (xi, false);
        }
        (self.history.len(), true)
    }
}

impl std::hash::Hash for TupleHistory {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for x in &self.t {
            x.hash(state);
        }
        for x in &self.history {
            x.hash(state);
        }
    }
}
impl PartialEq for TupleHistory {
    fn eq(&self, other: &Self) -> bool {
        self.t == other.t && self.history == other.history
    }
}
impl Eq for TupleHistory {}

#[derive(Clone)]
struct IndicesTupleGenerator {
    ops: BTreeMap<usize, Vec<Operation>>,
    arity: usize,
    viejos: Vec<usize>,
    nuevos: Vec<usize>,
    sintactico: Vec<Term>,
    last_term: Option<Term>,
    forked: bool,
    pub finished: bool,
    state: GenState,
}

#[derive(Clone)]
enum GenState {
    Init,
    Iter {
        arities: Vec<usize>,
        arity_idx: usize,
        op_idx: usize,
        perm_idx: usize,
        perms: Vec<Vec<usize>>,
    },
}

impl IndicesTupleGenerator {
    fn new(
        ops: BTreeMap<usize, Vec<Operation>>,
        arity: usize,
        viejos: Vec<usize>,
        nuevos: Vec<usize>,
    ) -> Self {
        let sintactico: Vec<Term> = (0..=arity + 10) // allow growth
            .map(|i| Term::Variable(Variable::from_index(i as i32)))
            .collect();
        Self {
            ops: ops.clone(),
            arity,
            viejos,
            nuevos,
            sintactico,
            last_term: None,
            forked: false,
            finished: false,
            state: GenState::Init,
        }
    }

    fn step(&mut self) -> Option<(Operation, Vec<usize>)> {
        if self.forked {
            panic!("Generator was forked!");
        }
        loop {
            match &mut self.state {
                GenState::Init => {
                    if let Some(constants) = self.ops.get(&0) {
                        if !constants.is_empty() {
                            let op = constants[0].clone();
                            self.last_term = Some(Term::OpTerm {
                                sym: OpSym::new(&op.sym, 0),
                                args: vec![],
                            });
                            self.state = GenState::Iter {
                                arities: vec![0],
                                arity_idx: 0,
                                op_idx: 1,
                                perm_idx: 0,
                                perms: vec![vec![]],
                            };
                            return Some((op, vec![]));
                        }
                    }
                    let arities: Vec<usize> = self.ops.keys().filter(|&&a| a > 0).cloned().collect();
                    if arities.is_empty() && !self.ops.contains_key(&0) {
                        self.finished = true;
                        return None;
                    }
                    self.state = GenState::Iter {
                        arities: arities.clone(),
                        arity_idx: 0,
                        op_idx: 0,
                        perm_idx: 0,
                        perms: vec![],
                    };
                    if !self.nuevos.is_empty() {
                        let pool: Vec<usize> = self.viejos.iter().chain(&self.nuevos).cloned().collect();
                        let forced: HashSet<usize> = self.nuevos.iter().cloned().collect();
                        if let Some(&a) = arities.first() {
                            let perms = cartesian_product_indices(&pool, a, &forced);
                            self.state = GenState::Iter {
                                arities: arities.clone(),
                                arity_idx: 0,
                                op_idx: 0,
                                perm_idx: 0,
                                perms,
                            };
                        }
                    }
                }
                GenState::Iter {
                    arities,
                    arity_idx,
                    op_idx,
                    perm_idx,
                    perms,
                } => {
                    if arities.is_empty() {
                        if self.nuevos.is_empty() {
                            self.finished = true;
                            return None;
                        }
                        self.viejos.extend(self.nuevos.drain(..));
                        self.state = GenState::Init;
                        continue;
                    }
                    if *arity_idx >= arities.len() {
                        if self.nuevos.is_empty() {
                            self.finished = true;
                            return None;
                        }
                        self.viejos.extend(self.nuevos.drain(..));
                        *arity_idx = 0;
                        let pool: Vec<usize> =
                            self.viejos.iter().chain(&self.nuevos).cloned().collect();
                        let forced: HashSet<usize> = self.nuevos.iter().cloned().collect();
                        *perms = cartesian_product_indices(&pool, arities[0], &forced);
                        continue;
                    }
                    let a = arities[*arity_idx];
                    let op_list = self.ops.get(&a)?;
                    if *op_idx >= op_list.len() {
                        *op_idx = 0;
                        *perm_idx = 0;
                        *arity_idx += 1;
                        if *arity_idx >= arities.len() {
                            if self.nuevos.is_empty() {
                                self.finished = true;
                                return None;
                            }
                            self.viejos.extend(self.nuevos.drain(..));
                            *arity_idx = 0;
                        }
                        let pool: Vec<usize> = self.viejos.iter().chain(&self.nuevos).cloned().collect();
                        let forced: HashSet<usize> = self.nuevos.iter().cloned().collect();
                        *perms = cartesian_product_indices(&pool, arities[*arity_idx], &forced);
                        continue;
                    }
                    if *perm_idx >= perms.len() {
                        *op_idx += 1;
                        *perm_idx = 0;
                        continue;
                    }
                    let op = op_list[*op_idx].clone();
                    let ti = perms[*perm_idx].clone();
                    *perm_idx += 1;
                    let args: Vec<Term> = ti.iter().map(|&i| self.sintactico[i].clone()).collect();
                    self.last_term = Some(Term::OpTerm {
                        sym: OpSym::new(&op.sym, op.arity),
                        args,
                    });
                    return Some((op, ti));
                }
            }
        }
    }

    #[allow(dead_code)]
    fn set_last_term(&mut self, op: &Operation, ti: &[usize]) {
        let args: Vec<Term> = ti.iter().map(|&i| self.sintactico[i].clone()).collect();
        self.last_term = Some(Term::OpTerm {
            sym: OpSym::new(&op.sym, op.arity),
            args,
        });
    }

    /// Avanza el generador hasta haber “consumido” el candidato (op, ti), para que
    /// la siguiente llamada a step() o take_candidates() devuelva el siguiente.
    /// Necesario cuando el candidato se eligió por IG en lugar de por step().
    #[allow(dead_code)]
    fn advance_until(&mut self, op: &Operation, ti: &[usize]) {
        loop {
            match self.step() {
                None => return,
                Some((o, t)) => {
                    if o.sym == op.sym && o.arity == op.arity && t == ti {
                        return;
                    }
                }
            }
        }
    }

    fn formula_diferenciadora(&mut self, index: usize) -> Formula {
        let term = self.last_term.as_ref().unwrap().clone();
        formulas::eq(term, self.sintactico[index].clone())
    }

    fn hubo_nuevo(&mut self) {
        if self.forked {
            panic!("Generator was forked!");
        }
        let new_idx = self.viejos.len() + self.nuevos.len();
        self.nuevos.push(new_idx);
        while self.sintactico.len() <= new_idx {
            self.sintactico
                .push(Term::Variable(Variable::from_index(self.sintactico.len() as i32)));
        }
        self.sintactico[new_idx] = self.last_term.as_ref().unwrap().clone();
    }

    fn fork(&mut self, quantity: usize) -> Vec<IndicesTupleGenerator> {
        if self.forked {
            panic!("Generator was forked!");
        }
        self.forked = true;
        (0..quantity)
            .map(|_| IndicesTupleGenerator {
                ops: self.ops.clone(),
                arity: self.arity,
                viejos: self.viejos.clone(),
                nuevos: self.nuevos.clone(),
                sintactico: self.sintactico.clone(),
                last_term: self.last_term.clone(),
                forked: false,
                finished: self.finished,
                state: GenState::Init,
            })
            .collect()
    }

    /// Candidatos en el mismo orden que step(), para que advance_until quede bien.
    fn enumerate_candidates(&self) -> Vec<(Operation, Vec<usize>)> {
        let mut gen = self.clone();
        std::iter::from_fn(move || gen.step()).collect()
    }

    /// Primeros `limit` candidatos en el mismo orden que step().
    fn take_candidates(&self, limit: usize) -> Vec<(Operation, Vec<usize>)> {
        let mut gen = self.clone();
        (0..limit).filter_map(|_| gen.step()).collect()
    }
}

#[derive(Clone)]
pub struct Block {
    operations: BTreeMap<usize, Vec<Operation>>,
    tuples: Vec<TupleHistory>,
    targets: Vec<Relation>,
    pub formula: Formula,
    arity: usize,
    generator: IndicesTupleGenerator,
    config: HitConfig,
}

impl Block {
    pub fn new(
        operations: BTreeMap<usize, Vec<Operation>>,
        tuples: Vec<TupleHistory>,
        targets: Vec<Relation>,
        formula: Formula,
        config: HitConfig,
    ) -> Self {
        let arity = targets[0].arity;
        let viejos = vec![];
        let nuevos: Vec<usize> = (0..arity).collect();
        let generator = IndicesTupleGenerator::new(operations.clone(), arity, viejos, nuevos);
        Self {
            operations,
            tuples,
            targets,
            formula,
            arity,
            generator,
            config,
        }
    }

    pub fn is_all_in_targets(&self) -> bool {
        self.tuples.iter().all(|th| th.in_target)
    }

    pub fn is_disjunt_to_targets(&self) -> bool {
        self.tuples.iter().all(|th| !th.in_target)
    }

    pub fn finished(&self) -> bool {
        self.generator.finished
    }

    pub fn step(&mut self) -> Result<Vec<Block>, Counterexample> {
        let steps = RUN_STATS.steps.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        if let Some(max) = self.config.max_steps {
            if steps > max {
                return Err(Counterexample(
                    self.tuples.iter().map(|th| th.t.clone()).collect(),
                ));
            }
        }
        let (op, ti) = if self.config.use_information_gain {
            // Con un solo candidato usamos step() para mantener el mismo orden que sin IG y no colgar.
            if self.config.ig_sample == Some(1) {
                match self.generator.step() {
                    Some(x) => x,
                    None => {
                        self.generator.finished = true;
                        return Ok(vec![self.clone()]);
                    }
                }
            } else {
            let cand_list = match self.config.ig_sample {
                Some(n) => self.generator.take_candidates(n),
                None => self.generator.enumerate_candidates(),
            };
            let mut cand_list = cand_list;
            if self.config.skip_useless_candidates {
                let before = cand_list.len();
                cand_list.retain(|(op, ti)| {
                    let mut splits = false;
                    let mut adds = false;
                    let mut first_idx: Option<usize> = None;
                    for th in &self.tuples {
                        let (idx, is_new) = th.simulate_step(op, ti);
                        if is_new {
                            adds = true;
                        }
                        match first_idx {
                            None => first_idx = Some(idx),
                            Some(f) if f != idx => splits = true,
                            _ => {}
                        }
                    }
                    splits || adds
                });
                RUN_STATS.candidates_skipped.fetch_add(
                    (before - cand_list.len()) as u64,
                    std::sync::atomic::Ordering::Relaxed,
                );
            }
            RUN_STATS.candidates_considered.fetch_add(
                cand_list.len() as u64,
                std::sync::atomic::Ordering::Relaxed,
            );
            // Si el sample quedó vacío tras skip, no marcar finished: el generador
            // puede tener más candidatos (legacy -i sigue step(); experimental busca útil).
            if cand_list.is_empty() {
                if self.config.ig_experimental && self.config.skip_useless_candidates {
                    loop {
                        match self.generator.step() {
                            None => {
                                self.generator.finished = true;
                                return Ok(vec![self.clone()]);
                            }
                            Some((op, ti)) => {
                                RUN_STATS
                                    .candidates_considered
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                let mut splits = false;
                                let mut adds = false;
                                let mut first_idx: Option<usize> = None;
                                for th in &self.tuples {
                                    let (idx, is_new) = th.simulate_step(&op, &ti);
                                    if is_new {
                                        adds = true;
                                    }
                                    match first_idx {
                                        None => first_idx = Some(idx),
                                        Some(f) if f != idx => splits = true,
                                        _ => {}
                                    }
                                }
                                if splits || adds {
                                    break (op, ti);
                                }
                                RUN_STATS
                                    .candidates_skipped
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            }
                        }
                    }
                } else {
                    match self.generator.step() {
                        Some(x) => x,
                        None => {
                            self.generator.finished = true;
                            return Ok(vec![self.clone()]);
                        }
                    }
                }
            } else {
            let tuples_clone = self.tuples.clone();
            let n = tuples_clone.len();
            let in_total = tuples_clone.iter().filter(|th| th.in_target).count();

            #[allow(unused_assignments)]
            let mut best: Option<(f64, usize, Operation, Vec<usize>)> = None;
            #[cfg(feature = "cuda")]
            {
                if let Some(((op_c, ti_c), _ig)) =
                    crate::hit_cuda::best_candidate_ig_cuda(&tuples_clone, &cand_list, n, in_total)
                {
                    let idx = cand_list
                        .iter()
                        .position(|(o, t)| o.sym == op_c.sym && o.arity == op_c.arity && t == ti_c)
                        .unwrap_or(0);
                    best = Some((0.0f64, idx, op_c, ti_c));
                }
            }
            if best.is_none() {
                best = cand_list
                .par_iter()
                .enumerate()
                .map(|(idx, (op, ti))| {
                    let part: HashMap<(usize, bool), (usize, usize)> = tuples_clone
                        .par_iter()
                        .map(|th| {
                            let (idx, _) = th.simulate_step(op, ti);
                            let key = (idx, th.in_target);
                            let val = if th.in_target { (1, 0) } else { (0, 1) };
                            let mut m = HashMap::new();
                            m.insert(key, val);
                            m
                        })
                        .reduce(
                            || HashMap::new(),
                            |mut a, b| {
                                for (k, v) in b {
                                    let e = a.entry(k).or_insert((0, 0));
                                    e.0 += v.0;
                                    e.1 += v.1;
                                }
                                a
                            },
                        );
                    let ig = information_gain_from_counts(n, in_total, &part);
                    (ig, idx, op.clone(), ti.clone())
                })
                .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            }
            // Por defecto (-i) se calcula IG pero se sigue el orden step() (legacy).
            // Con ig_experimental se aplica el mejor candidato + advance_until.
            match best {
                Some((_ig, _best_idx, op, ti)) if self.config.ig_experimental => {
                    self.generator.advance_until(&op, &ti);
                    (op, ti)
                }
                Some(_) => {
                    let (op, ti) = self.generator.step().expect("al menos un candidato");
                    (op, ti)
                }
                None => {
                    self.generator.finished = true;
                    return Ok(vec![self.clone()]);
                }
            }
            }
            }
        } else if self.config.skip_useless_candidates {
            loop {
                match self.generator.step() {
                    None => {
                        self.generator.finished = true;
                        return Ok(vec![self.clone()]);
                    }
                    Some((op, ti)) => {
                        RUN_STATS
                            .candidates_considered
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let mut splits = false;
                        let mut adds = false;
                        let mut first_idx: Option<usize> = None;
                        for th in &self.tuples {
                            let (idx, is_new) = th.simulate_step(&op, &ti);
                            if is_new {
                                adds = true;
                            }
                            match first_idx {
                                None => first_idx = Some(idx),
                                Some(f) if f != idx => splits = true,
                                _ => {}
                            }
                        }
                        if splits || adds {
                            break (op, ti);
                        }
                        RUN_STATS
                            .candidates_skipped
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }
        } else {
            match self.generator.step() {
                Some(x) => x,
                None => return Ok(vec![self.clone()]),
            }
        };

        let mut result: HashMap<usize, Vec<TupleHistory>> = HashMap::new();
        let mut any_has_gen = false;
        for mut th in std::mem::take(&mut self.tuples) {
            let idx = th.step(&op, &ti);
            any_has_gen = any_has_gen || th.has_generated;
            result.entry(idx).or_default().push(th);
        }

        if result.len() == 1 {
            if any_has_gen {
                self.generator.hubo_nuevo();
            }
            self.tuples = result.into_values().next().unwrap();
            return Ok(vec![self.clone()]);
        }

        let num_groups = result.len();
        let mut generators = self.generator.fork(num_groups);
        let mut blocks = Vec::new();
        let mut fneg = formulas::true_formula(None);
        let mut negados = Vec::new();
        let mut i = 0;
        for (index, tuples_new) in result {
            if tuples_new.iter().any(|th| th.has_generated) {
                generators[i].hubo_nuevo();
                negados.push((i, index, tuples_new));
            } else {
                let fd = generators[i].formula_diferenciadora(index);
                let f = self.formula.clone().and_formula(&fd);
                fneg = fneg.and_formula(&fd.neg());
                blocks.push(Block {
                    operations: self.operations.clone(),
                    tuples: tuples_new,
                    targets: self.targets.clone(),
                    formula: f,
                    arity: self.arity,
                    generator: generators[i].clone(),
                    config: self.config,
                });
            }
            i += 1;
        }
        for (i, _index, tuples_new) in negados {
            let f = self.formula.clone().and_formula(&fneg);
            blocks.push(Block {
                operations: self.operations.clone(),
                tuples: tuples_new,
                targets: self.targets.clone(),
                formula: f,
                arity: self.arity,
                generator: generators[i].clone(),
                config: self.config,
            });
        }
        Ok(blocks)
    }
}

fn cartesian_product_sized<T: Clone>(items: &[T], n: usize) -> Vec<Vec<T>> {
    if n == 0 {
        return vec![vec![]];
    }
    let mut result = vec![vec![]];
    for _ in 0..n {
        let mut next = Vec::new();
        for r in &result {
            for item in items {
                let mut row = r.clone();
                row.push(item.clone());
                next.push(row);
            }
        }
        result = next;
    }
    result
}

pub fn is_open_def(
    model: &Model,
    targets: Vec<Relation>,
    config: HitConfig,
) -> Result<Formula, Counterexample> {
    reset_run_stats();
    if config.approx_precheck {
        for target in &targets {
            if crate::approx::target_breaks_approx_classes(model, target) {
                RUN_STATS
                    .approx_reject
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Err(Counterexample(
                    target.r.iter().take(1).cloned().collect(),
                ));
            }
        }
    }
    let arity = targets[0].arity;
    let universe = &model.universe;
    let targets_ref: Vec<&Relation> = targets.iter().collect();
    let mut tuples: Vec<TupleHistory> = cartesian_product_sized(universe, arity)
        .into_iter()
        .map(|t| TupleHistory::new(t, &targets_ref))
        .collect();
    tuples.sort_by(|a, b| a.t.cmp(&b.t));
    let mut operations: BTreeMap<usize, Vec<Operation>> = BTreeMap::new();
    for op in model.operations.values() {
        operations.entry(op.arity).or_default().push(op.clone());
    }
    for op_list in operations.values_mut() {
        op_list.sort_by(|a, b| a.sym.cmp(&b.sym));
    }
    let formula = targets[0]
        .pattern
        .as_ref()
        .unwrap()
        .preprocessed_formula();
    let start_block = Block::new(operations, tuples, targets, formula, config);
    let result = match config.explore_order {
        ExploreOrder::Dfs => is_open_def_parallel_recursive(start_block),
        ExploreOrder::Bfs => is_open_def_bfs(start_block),
    };
    match result {
        Ok(f) if config.simplify_formula => Ok(f.simplify_ast()),
        other => other,
    }
}

/// Exploración BFS: cola de bloques; combina fórmulas de hojas con OR.
fn is_open_def_bfs(start: Block) -> Result<Formula, Counterexample> {
    use std::collections::VecDeque;
    let mut queue: VecDeque<Block> = VecDeque::new();
    queue.push_back(start);
    let mut leaf_formulas: Vec<Formula> = Vec::new();
    while let Some(mut block) = queue.pop_front() {
        if block.is_all_in_targets() {
            leaf_formulas.push(block.formula);
            continue;
        }
        if block.is_disjunt_to_targets() {
            leaf_formulas.push(formulas::false_formula(None));
            continue;
        }
        if block.finished() {
            return Err(Counterexample(
                block.tuples.iter().map(|th| th.t.clone()).collect(),
            ));
        }
        match block.step() {
            Err(ce) => return Err(ce),
            Ok(children) => {
                for child in children {
                    queue.push_back(child);
                }
            }
        }
    }
    let mut combined = formulas::false_formula(None);
    for f in leaf_formulas {
        combined = combined.or_formula(&f);
    }
    Ok(combined)
}

/// Explora el árbol de bloques en paralelo cuando hay varios hijos; con un solo hijo
/// sigue en bucle para evitar desbordamiento de pila en cadenas largas.
fn is_open_def_parallel_recursive(mut block: Block) -> Result<Formula, Counterexample> {
    loop {
        if block.is_all_in_targets() {
            return Ok(block.formula);
        }
        if block.is_disjunt_to_targets() {
            return Ok(formulas::false_formula(None));
        }
        if block.finished() {
            return Err(Counterexample(
                block.tuples.iter().map(|th| th.t.clone()).collect(),
            ));
        }
        match block.step() {
            Err(ce) => return Err(ce),
            Ok(children) => {
                if children.len() == 1 {
                    block = children.into_iter().next().unwrap();
                    continue;
                }
                let results: Vec<_> = children
                    .into_par_iter()
                    .map(is_open_def_parallel_recursive)
                    .collect();
                if let Some(ce) = results.iter().find_map(|r| r.as_ref().err()) {
                    return Err(ce.clone());
                }
                let formulas: Vec<Formula> = results
                    .into_iter()
                    .map(|r| r.unwrap())
                    .collect();
                let mut combined = formulas::false_formula(None);
                for f in formulas {
                    combined = combined.or_formula(&f);
                }
                return Ok(combined);
            }
        }
    }
}

#[allow(dead_code)]
fn is_open_def_iterative(block: &mut Block) -> Result<Formula, Counterexample> {
    #[derive(Clone)]
    enum StackItem {
        Block(Block),
        Result(Formula),
    }
    let mut stack = vec![StackItem::Block(block.clone())];
    let mut accumulate: Vec<(Vec<Block>, Vec<Formula>)> = vec![];

    while let Some(item) = stack.pop() {
        match item {
            StackItem::Block(b) => {
                if b.is_all_in_targets() {
                    stack.push(StackItem::Result(b.formula));
                } else if b.is_disjunt_to_targets() {
                    stack.push(StackItem::Result(formulas::false_formula(None)));
                } else if b.finished() {
                    return Err(Counterexample(
                        b.tuples.iter().map(|th| th.t.clone()).collect(),
                    ));
                } else {
                    let mut bmut = b;
                    match bmut.step() {
                        Ok(children) => {
                            if children.len() == 1 {
                                stack.push(StackItem::Block(children.into_iter().next().unwrap()));
                            } else {
                                accumulate.push((children.clone(), vec![]));
                                for child in children.into_iter().rev() {
                                    stack.push(StackItem::Block(child));
                                }
                            }
                        }
                        Err(ce) => return Err(ce),
                    }
                }
            }
            StackItem::Result(formula) => {
                if accumulate.is_empty() {
                    return Ok(formula);
                }
                let (children, results) = accumulate.pop().unwrap();
                let mut results = results;
                results.push(formula);
                if results.len() == children.len() {
                    let mut combined = formulas::false_formula(None);
                    for r in results {
                        combined = combined.or_formula(&r);
                    }
                    stack.push(StackItem::Result(combined));
                } else {
                    let next_idx = results.len();
                    accumulate.push((children.clone(), results));
                    let next_block = children[next_idx].clone();
                    stack.push(StackItem::Block(next_block));
                }
            }
        }
    }
    Ok(formulas::false_formula(None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preprocessing::Pattern;
    use std::collections::{BTreeMap, HashMap};

    fn make_target(arity: usize, tuples: &[Vec<i64>], with_pattern: bool) -> Relation {
        let mut r = Relation::new("T0", arity);
        for t in tuples {
            r.add(t.clone());
        }
        if with_pattern && !tuples.is_empty() {
            r.pattern = Some(Box::new(Pattern::new(tuples[0].clone())));
        }
        r
    }

    #[test]
    fn test_entropy_cero_cero() {
        assert_eq!(entropy(0, 0), 0.0);
    }

    #[test]
    fn test_entropy_mitad_mitad() {
        let h = entropy(5, 5);
        assert!((h - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_entropy_uniforme() {
        let h = entropy(1, 1);
        assert!(h > 0.0 && (h - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_entropy_puro_in() {
        assert!(entropy(10, 0).abs() < 1e-12);
    }

    #[test]
    fn test_entropy_puro_out() {
        assert!(entropy(0, 10).abs() < 1e-12);
    }

    #[test]
    fn test_ig_n_cero() {
        let ig = information_gain_from_counts(0, 0, &HashMap::new());
        assert_eq!(ig, 0.0);
    }

    #[test]
    fn test_ig_particion_perfecta() {
        let mut part = HashMap::new();
        part.insert((0, true), (5, 0));
        part.insert((1, false), (0, 5));
        let ig = information_gain_from_counts(10, 5, &part);
        assert!(ig > 0.9);
    }

    #[test]
    fn test_ig_sin_ganancia() {
        let mut part = HashMap::new();
        part.insert((0, true), (3, 2));
        part.insert((0, false), (2, 3));
        let ig = information_gain_from_counts(10, 5, &part);
        assert!(ig >= 0.0);
    }

    #[test]
    fn test_tuple_history_step_nuevo_elemento() {
        let target = make_target(2, &[vec![0, 0], vec![1, 1]], false);
        let targets = [&target];
        let mut th = TupleHistory::new(vec![0, 0], &targets);
        let mut op = Operation::new("f", 2);
        op.add(vec![0, 0, 1]);
        op.add(vec![0, 1, 0]);
        op.add(vec![1, 0, 0]);
        op.add(vec![1, 1, 0]);
        let idx = th.step(&op, &[0, 1]);
        assert_eq!(idx, 2);
        assert!(th.has_generated);
    }

    #[test]
    fn test_tuple_history_step_existente() {
        let target = make_target(2, &[vec![0, 0], vec![1, 1]], false);
        let targets = [&target];
        let mut th = TupleHistory::new(vec![0, 0], &targets);
        let mut op = Operation::new("id", 1);
        op.add(vec![0, 0]);
        op.add(vec![1, 1]);
        let step_idx = th.step(&op, &[0]);
        assert!(step_idx == 0 || step_idx == 1);
        assert!(!th.has_generated);
    }

    #[test]
    fn test_tuple_history_simulate_step() {
        let target = make_target(2, &[vec![0, 1]], false);
        let targets = [&target];
        let th = TupleHistory::new(vec![0, 0], &targets);
        let mut op = Operation::new("f", 2);
        op.add(vec![0, 0, 1]);
        op.add(vec![0, 1, 0]);
        op.add(vec![1, 0, 0]);
        op.add(vec![1, 1, 1]);
        let (_, has_gen) = th.simulate_step(&op, &[0, 1]);
        assert!(has_gen);
        assert_eq!(th.history.len(), 2);
    }

    #[test]
    fn test_tuple_history_eq_hash() {
        let target = make_target(1, &[vec![0]], false);
        let targets = [&target];
        let a = TupleHistory::new(vec![0], &targets);
        let b = TupleHistory::new(vec![0], &targets);
        assert!(a == b);
        use std::hash::{Hash, Hasher};
        let mut ha = std::collections::hash_map::DefaultHasher::new();
        let mut hb = std::collections::hash_map::DefaultHasher::new();
        a.hash(&mut ha);
        b.hash(&mut hb);
        assert_eq!(ha.finish(), hb.finish());
    }

    #[test]
    fn test_block_is_all_in_targets() {
        let mut op = Operation::new("f", 2);
        op.add(vec![0, 0, 0]);
        op.add(vec![0, 1, 1]);
        op.add(vec![1, 0, 1]);
        op.add(vec![1, 1, 0]);
        let mut ops = BTreeMap::new();
        ops.insert(2, vec![op]);
        let target = make_target(2, &[vec![0, 0], vec![1, 1], vec![0, 1], vec![1, 0]], true);
        let targets = vec![target];
        let th = TupleHistory::new(vec![0, 0], &[&targets[0]]);
        let f = targets[0].pattern.as_ref().unwrap().preprocessed_formula();
        let block = Block::new(ops, vec![th], targets, f, HitConfig::default());
        assert!(block.is_all_in_targets());
    }

    #[test]
    fn test_block_is_disjunt_to_targets() {
        let target = make_target(2, &[vec![0, 1]], true);
        let targets = vec![target.clone()];
        let th = TupleHistory::new(vec![0, 0], &[&target]);
        let f = target.pattern.as_ref().unwrap().preprocessed_formula();
        let block = Block::new(BTreeMap::new(), vec![th], targets, f, HitConfig::default());
        assert!(block.is_disjunt_to_targets());
    }

    #[test]
    fn test_counterexample_raise() {
        let ce = Counterexample(vec![vec![1], vec![2], vec![3]]);
        let s = format!("{:?}", ce);
        assert!(s.contains("1"));
    }
}
