use crate::utils::language::*;
use crate::utils::random_gen::*;
use egg::*;
use rand::prelude::SliceRandom;
use rand::Rng;
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::de::value;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::fs::{self};
use std::io::Write;
use std::path::Path;
pub struct Extractor2<'a, CF: CostFunction<L>, L: Language, N: Analysis<L>> {
    cost_function: CF,
    //costs: HashMap<Id, (CF::Cost, usize, L)>,
    costs: FxHashMap<Id, (CF::Cost, usize, L)>,
    egraph: &'a EGraph<L, N>,
}

pub struct ParallelWrapper<'a, T>(&'a [T]);

impl<'a, T> IntoParallelIterator for ParallelWrapper<'a, T>
where
    T: Send + Sync,
{
    type Item = &'a T;
    type Iter = rayon::slice::Iter<'a, T>;

    fn into_par_iter(self) -> Self::Iter {
        self.0.par_iter()
    }
}

#[derive(Serialize, Deserialize)]
struct Choices {
    //choices: HashMap<String, String>,
    choices: FxHashMap<String, String>,
}

fn cmp<T: PartialOrd>(a: &Option<T>, b: &Option<T>) -> Ordering {
    // None is high
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => a.partial_cmp(b).unwrap(),
    }
}

impl<'a, CF, L, N> Extractor2<'a, CF, L, N>
where
    CF: CostFunction<L>,
    L: Language,
    N: Analysis<L>,
{
    /// Create a new `Extractor` given an `EGraph` and a
    /// `CostFunction`.
    ///
    /// The extraction does all the work on creation, so this function
    /// performs the greedy search for cheapest representative of each
    /// eclass.
    pub fn new(egraph: &'a EGraph<L, N>, cost_function: CF) -> Self
    where
        <CF as CostFunction<L>>::Cost: Ord,
    {
        //let costs = HashMap::default();
        let costs = FxHashMap::default();
        let mut extractor = Extractor2 {
            costs,
            egraph,
            cost_function,
        };
        extractor.find_costs();
        extractor.costs.iter().for_each(|(id, (cost, index, l))| {
            // println!("Id: {}, Cost: {:?}, Index: {}, L: {:?}", id, cost, index, l);
        });
        extractor
    }

    pub fn new_random(egraph: &'a EGraph<L, N>, cost_function: CF) -> Self
    where
        <CF as CostFunction<L>>::Cost: Ord,
    {
        //let costs = HashMap::default();
        let costs = FxHashMap::default();
        let mut extractor = Extractor2 {
            costs,
            egraph,
            cost_function,
        };
        extractor.find_costs_random();
        extractor.costs.iter().for_each(|(id, (cost, index, l))| {
            // println!("Id: {}, Cost: {:?}, Index: {}, L: {:?}", id, cost, index, l);
        });
        extractor
    }

    pub fn get_node(&self, id: Id) -> &L {
        let random_num = generate_random_float1();
        //println!("random_num{}",random_num);
        if random_num > (0.5 as f64) {
            let eclass = &self.egraph[id];
            let nodes: Vec<&L> = eclass.iter().collect();
            let mut rng = rand::thread_rng();
            let random_index = rng.gen_range(0..nodes.len());
            let random_node = nodes[random_index];
            random_node
        }
        // get random node from class id
        else {
            self.find_best_node(id)
        }
    }
    // pub fn find_best_random(&mut self, eclass: Id) -> (CF::Cost, RecExpr<L>) {
    //     let root = self.costs[&self.egraph.find(eclass)].clone().1;
    //     let expr = root.build_recexpr(|child| self.get_node(child).clone());

    //     let cost = self.cost_function.cost_rec(&expr);
    //     (cost,expr)
    // }
    // pub fn find_cost_best_random(&mut self,eclass: Id) ->CF::Cost{

    // }
    /// Find the cheapest (lowest cost) represented `RecExpr` in the
    /// given eclass.
    pub fn find_best(&self, eclass: Id) -> (CF::Cost, RecExpr<L>) {
        let (cost, index, root) = self.costs[&self.egraph.find(eclass)].clone();
        let expr = root.build_recexpr(|id| self.find_best_node(id).clone());
        //let result = self.record_costs();

        (cost, expr)
    }

    pub fn find_best_no_expr(&self, eclass: Id) -> (CF::Cost, L) {
        let (cost, index, root) = self.costs[&self.egraph.find(eclass)].clone();
        //let expr = root.build_recexpr(|id| self.find_best_node(id).clone());
        //let result = self.record_costs();

        (cost, root)
    }

    /// Find the cheapest e-node in the given e-class.
    pub fn find_best_node(&self, eclass: Id) -> &L {
        &self.costs[&self.egraph.find(eclass)].2
    }

    pub fn record_costs_random(
        &self,
        num_runs: u32,
        random_ratio: f64,
        input_vec_id: Vec<Id>,
        root: L,
    ) -> BTreeMap<u32, RecExpr<L>> {
        let mut rec_expr_map: BTreeMap<u32, RecExpr<L>> = BTreeMap::new();
        for num in 0..num_runs {
            // let mut result: HashMap<String, String> = HashMap::new();
            // let mut result1: HashMap<String, L> = HashMap::new();
            // let mut selected_ids: HashSet<Id> = HashSet::new(); // Used to track selected node Ids
            let mut result: FxHashMap<String, String> = FxHashMap::default();
            let mut result1: FxHashMap<String, L> = FxHashMap::default();
            let mut selected_ids: FxHashSet<Id> = HashSet::default(); // Used to track selected node Ids

            for (id, (_, index, _)) in self.costs.iter() {
                let eclass = &self.egraph[*id];
                let nodes: Vec<&L> = eclass.iter().collect();
                let mut rng = rand::thread_rng();

                //  println!("eclass: {:?}", eclass);  // Print eclass

                if input_vec_id.contains(id) {
                    let value = format!("{}.{}", id, index);
                    let value1 = &eclass.nodes[*index];
                    result.insert(id.to_string(), value);
                    result1.insert(id.to_string(), value1.clone());
                    selected_ids.insert(*id);
                } else if !selected_ids.contains(id)
                    && rng.gen::<f64>() <= random_ratio
                    && nodes.len() > 1
                    && nodes.iter().all(|node| {
                        node.children()
                            .iter()
                            .all(|child_id| selected_ids.contains(child_id))
                    })
                {
                    let random_index = rng.gen_range(0..nodes.len());
                    let value = format!("{}.{}", id, random_index);
                    let value1 = &eclass.nodes[random_index];
                    result.insert(id.to_string(), value);
                    result1.insert(id.to_string(), value1.clone());

                    // Add the selected node ID to the set
                    selected_ids.insert(*id);
                } else {
                    let value = format!("{}.{}", id, index);
                    let value1 = &eclass.nodes[*index];
                    result.insert(id.to_string(), value);
                    result1.insert(id.to_string(), value1.clone());
                }
            }
            // fn has_self_loop<L>(eclass: &Vec<L>) -> bool {
            //     for node in eclass {
            //         if node.id == node.id {
            //             return true;
            //         }
            //     }
            //     false
            // }

            let filename = format!("result{}.json", num);
            let path = format!("random_result/{}", filename);

            // Create directory if it doesn't exist
            if let Err(err) = fs::create_dir_all("random_result") {
                eprintln!("Failed to create directory: {}", err);
                continue; // Skip current iteration if directory creation fails
            }

            if let Ok(mut file) = File::create(path) {
                if let Ok(json) = serde_json::to_string_pretty(&Choices { choices: result }) {
                    if let Err(err) = write!(file, "{}", json) {
                        eprintln!("Failed to write to file: {}", err);
                    }
                } else {
                    eprintln!("Failed to serialize to JSON");
                }
            } else {
                eprintln!("Failed to create file");
            }

            let filename = format!("result{}.json", num);
            let path = format!("random_graph/{}", filename);
            if let Err(err) = fs::create_dir_all("random_graph") {
                eprintln!("Failed to create directory: {}", err);
                continue; // Skip current iteration if directory creation fails
            }
            let expr = root.build_recexpr(|child| self.get_value_by_id(child, &result1).clone());
            rec_expr_map.insert(num, expr.clone());
        }
        rec_expr_map
    }

    //pub fn get_value_by_id(&self, eclass: Id, map: &HashMap<String, L>) -> L {
    pub fn get_value_by_id(&self, eclass: Id, map: &FxHashMap<String, L>) -> L {
        map.get(&eclass.to_string()).cloned().unwrap()
    }

    pub fn record_costs(&self) {
        //let mut result: HashMap<String, String> = HashMap::new();
        let mut result: FxHashMap<String, String> = FxHashMap::default();

        for (id, (_, index, _)) in self.costs.iter() {
            let value = format!("{}.{}", id, index);

            result.insert(id.to_string(), value);
        }

        // print the element of results
        // for (key, value) in result.iter() {
        //     println!("key: {}, value: {}", key, value);
        // }

        // println!("Costs: {:?}", self.costs.iter());
        // for (id, (cost, index, l)) in self.costs.iter() {
        //     println!("Id: {}, Cost: {:?}, Index: {}, L: {:?}", id, cost, index, l);
        // }
        let choices = Choices { choices: result };

        if let Ok(mut file) = File::create("result.json") {
            if let Ok(json) = serde_json::to_string_pretty(&choices) {
                if let Err(err) = write!(file, "{}", json) {
                    eprintln!("Failed to write to file: {}", err);
                }
            } else {
                eprintln!("Failed to serialize to JSON");
            }
        } else {
            eprintln!("Failed to create file");
        }
    }
    /// Find the cost of the term that would be extracted from this e-class.
    // pub fn find_best_cost(&self, eclass: Id) -> CF::Cost {
    //     let (cost, _) = &self.costs[&self.egraph.find(eclass)];
    //     cost.clone()
    // }
    fn node_total_cost(&mut self, node: &L) -> Option<CF::Cost> {
        let eg = &self.egraph;
        let has_cost = |id| self.costs.contains_key(&eg.find(id));
        if node.all(has_cost) {
            let costs = &self.costs;
            let cost_f = |id| costs[&eg.find(id)].0.clone();
            Some(self.cost_function.cost(node, cost_f))
        } else {
            None
        }
    }

    fn find_costs(&mut self)
    where
        <CF as CostFunction<L>>::Cost: Ord,
    {
        let mut did_something = true;
        while did_something {
            did_something = false;
            for class in self.egraph.classes() {
                let pass = self.make_pass(class);
                match (self.costs.get(&class.id), pass) {
                    (None, Some((cost, index, l))) => {
                        self.costs.insert(class.id, (cost, index, l));
                        did_something = true;
                    }
                    (Some((old_cost, _index, _)), Some((new_cost, new_index, l)))
                        if new_cost < *old_cost =>
                    {
                        self.costs.insert(class.id, (new_cost, new_index, l));
                        did_something = true;
                    }
                    _ => (),
                }
            }
        }
    }

    fn make_pass(&mut self, eclass: &EClass<L, N::Data>) -> Option<(CF::Cost, usize, L)>
    where
        <CF as CostFunction<L>>::Cost: Ord,
    {
        let result: Vec<(CF::Cost, usize, L)> = eclass
            .iter()
            .enumerate()
            .filter_map(|(index, n)| match self.node_total_cost(n) {
                Some(cost) => Some((cost, index, n.clone())),
                None => None,
            })
            .collect();

        let min_cost = result.iter().map(|(cost, _, _)| cost).cloned().min();

        if let Some(min_cost) = min_cost {
            let min_cost_tuples: Vec<(CF::Cost, usize, L)> = result
                .iter()
                .filter(|(cost, _, _)| cost == &min_cost)
                .cloned()
                .collect();
            let mut rng = rand::thread_rng();
            if let Some(selected_tuple) = min_cost_tuples.choose(&mut rng) {
                //    println!("Selected Tuple: {:?}", selected_tuple);
                return Some(selected_tuple.clone());
            }
        }

        None
    }

    fn find_costs_random(&mut self)
    where
        <CF as CostFunction<L>>::Cost: Ord,
    {
        let mut did_something = true;
        while did_something {
            did_something = false;
            for class in self.egraph.classes() {
                let pass = self.make_pass_random(class, 0.3);
                match (self.costs.get(&class.id), pass) {
                    (None, Some((cost, index, l))) => {
                        self.costs.insert(class.id, (cost, index, l));
                        did_something = true;
                    }
                    (Some((old_cost, _index, _)), Some((new_cost, new_index, l)))
                        if new_cost < *old_cost =>
                    {
                        self.costs.insert(class.id, (new_cost, new_index, l));
                        did_something = true;
                    }
                    _ => (),
                }
            }
        }
    }

    fn make_pass_random(
        &mut self,
        eclass: &EClass<L, N::Data>,
        random_ratio: f64,
    ) -> Option<(CF::Cost, usize, L)>
    where
        CF: CostFunction<L>,
        CF::Cost: Ord,
        L: Clone,
    {
        let result: Vec<(CF::Cost, usize, L)> = eclass
            .iter()
            .enumerate()
            .filter_map(|(index, n)| match self.node_total_cost(n) {
                Some(cost) => Some((cost, index, n.clone())),
                None => None,
            })
            .collect();

        let mut rng = rand::thread_rng();

        if rng.gen::<f64>() < random_ratio {
            if let Some(selected_tuple) = result.choose(&mut rng) {
                // println!("Selected Tuple: {:?}", selected_tuple);
                return Some(selected_tuple.clone());
            }
        } else {
            let min_cost = result.iter().map(|(cost, _, _)| cost).cloned().min();

            if let Some(min_cost) = min_cost {
                let min_cost_tuples: Vec<(CF::Cost, usize, L)> = result
                    .iter()
                    .filter(|(cost, _, _)| cost == &min_cost)
                    .cloned()
                    .collect();
                let mut rng = rand::thread_rng();
                if let Some(selected_tuple) = min_cost_tuples.choose(&mut rng) {
                    //    println!("Selected Tuple: {:?}", selected_tuple);
                    return Some(selected_tuple.clone());
                }
            }
        }
        None
    }

    // for class in self.egraph.classes() {
    //     if !self.costs.contains_key(&class.id) {
    //         log::warn!(
    //             "Failed to compute cost for eclass {}: {:?}",
    //             class.id,
    //             class.nodes
    //         )
    //     }
    // }
    // }
}

pub struct Extractor1<'a, CF: CostFunction<L>, L: Language, N: Analysis<L>> {
    cost_function: CF,
    costs: HashMap<Id, (CF::Cost, L)>,
    egraph: &'a EGraph<L, N>,
}

impl<'a, CF, L, N> Extractor1<'a, CF, L, N>
where
    CF: CostFunction<L>,
    L: Language,
    N: Analysis<L>,
{
    /// Create a new `Extractor` given an `EGraph` and a
    /// `CostFunction`.
    ///
    /// The extraction does all the work on creation, so this function
    /// performs the greedy search for cheapest representative of each
    /// eclass.
    pub fn new(egraph: &'a EGraph<L, N>, cost_function: CF) -> Self
    where
        <CF as CostFunction<L>>::Cost: Ord,
    {
        let costs = HashMap::default();
        let mut extractor = Extractor1 {
            costs,
            egraph,
            cost_function,
        };
        extractor.find_costs();

        extractor
    }
    pub fn get_node(&self, id: Id) -> &L {
        let random_num = generate_random_float1();
        //println!("random_num{}",random_num);
        if random_num > (0.5 as f64) {
            let eclass = &self.egraph[id];
            let nodes: Vec<&L> = eclass.iter().collect();
            let mut rng = rand::thread_rng();
            let random_index = rng.gen_range(0..nodes.len());
            let random_node = nodes[random_index];
            random_node
        }
        // get random node from class id
        else {
            self.find_best_node(id)
        }
    }
    pub fn find_best_random(&mut self, eclass: Id) -> (CF::Cost, RecExpr<L>) {
        let root = self.costs[&self.egraph.find(eclass)].clone().1;
        let expr = root.build_recexpr(|child| self.get_node(child).clone());

        let cost = self.cost_function.cost_rec(&expr);
        (cost, expr)
    }
    // pub fn find_cost_best_random(&mut self,eclass: Id) ->CF::Cost{

    // }
    /// Find the cheapest (lowest cost) represented `RecExpr` in the
    /// given eclass.
    pub fn find_best(&self, eclass: Id) -> (CF::Cost, RecExpr<L>) {
        let (cost, root) = self.costs[&self.egraph.find(eclass)].clone();
        let expr = root.build_recexpr(|id| self.find_best_node(id).clone());
        (cost, expr)
    }
    /// Find the cheapest e-node in the given e-class.
    pub fn find_best_node(&self, eclass: Id) -> &L {
        &self.costs[&self.egraph.find(eclass)].1
    }
    /// Find the cost of the term that would be extracted from this e-class.
    // pub fn find_best_cost(&self, eclass: Id) -> CF::Cost {
    //     let (cost, _) = &self.costs[&self.egraph.find(eclass)];
    //     cost.clone()
    // }
    fn node_total_cost(&mut self, node: &L) -> Option<CF::Cost> {
        let eg = &self.egraph;
        let has_cost = |id| self.costs.contains_key(&eg.find(id));
        if node.all(has_cost) {
            let costs = &self.costs;
            let cost_f = |id| costs[&eg.find(id)].0.clone();
            Some(self.cost_function.cost(node, cost_f))
        } else {
            None
        }
    }

    fn find_costs(&mut self)
    where
        <CF as CostFunction<L>>::Cost: Ord,
    {
        let mut did_something = true;
        while did_something {
            did_something = false;
            for class in self.egraph.classes() {
                let pass = self.make_pass(class);
                // if alpha<=0.8 {
                match (self.costs.get(&class.id), pass) {
                    (None, Some(new)) => {
                        self.costs.insert(class.id, new);
                        did_something = true;
                    }

                    (Some(old), Some(new)) if new.0 < old.0 => {
                        self.costs.insert(class.id, new);
                        did_something = true;
                    }
                    _ => (),
                }
                // }
                // else{
                //     match (self.costs.get(&class.id), pass) {
                //         (None, Some(new)) => {
                //             self.costs.insert(class.id, new);
                //             did_something = true;
                //         }

                //         (Some(old), Some(new)) if new.0 >= old.0 => {
                //             self.costs.insert(class.id, new);
                //             did_something = true;
                //         }
                //         _ => (),
                //     }
            }
        }
    }

    fn make_pass(&mut self, eclass: &EClass<L, N::Data>) -> Option<(CF::Cost, L)>
    where
        <CF as CostFunction<L>>::Cost: Ord,
    {
        let result: Vec<(CF::Cost, L)> = eclass
            .iter()
            .filter_map(|n| match self.node_total_cost(n) {
                Some(cost) => Some((cost, n.clone())),
                None => None,
            })
            .collect();

        let min_cost = result.iter().map(|(cost, _)| cost).cloned().min();

        if let Some(min_cost) = min_cost {
            let min_cost_tuples: Vec<(CF::Cost, L)> = result
                .iter()
                .filter(|(cost, _)| cost == &min_cost)
                .cloned()
                .collect();
            let mut rng = rand::thread_rng();
            if let Some(selected_tuple) = min_cost_tuples.choose(&mut rng) {
                return Some(selected_tuple.clone());
            }
        }

        None
    }
    // for class in self.egraph.classes() {
    //     if !self.costs.contains_key(&class.id) {
    //         log::warn!(
    //             "Failed to compute cost for eclass {}: {:?}",
    //             class.id,
    //             class.nodes
    //         )
    //     }
    // }
    // }
}
