// This module uses the following external crates:
// - indexmap: Provides the IndexMap data structure
// - rustc_hash: Provides the FxHashMap and FxHashSet data structures
// - rand: Provides random number generation functionality
// - serde: Provides serialization and deserialization functionality

use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use core::num;
use std::collections::HashMap;

pub use crate::*;

pub mod bottom_up;
pub mod faster_bottom_up;
pub mod faster_greedy_dag;
pub mod global_greedy_dag;
pub mod greedy_dag;
use rand::Rng;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
#[cfg(feature = "ilp-cbc")]
pub mod ilp_cbc;

// Extractor trait defines the interface for extracting a result from an EGraph
pub trait Extractor: Sync {
    // extract method takes an EGraph, roots, and cost_function as input
    // and returns an ExtractionResult
    fn extract(&self, egraph: &EGraph, roots: &[ClassId], cost_function: &str, random_prob: f64) -> ExtractionResult;

    // boxed method allows creating a boxed instance of the Extractor trait
    fn boxed(self) -> Box<dyn Extractor>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

// MapGet trait defines a generic interface for getting a value from a map-like data structure
pub trait MapGet<K, V> {
    // get method takes a key of type K and returns an optional reference to the corresponding value of type V
    fn get(&self, key: &K) -> Option<&V>;
}

// Implement MapGet for HashMap
impl<K, V> MapGet<K, V> for HashMap<K, V>
where
    K: Eq + std::hash::Hash,
{
    fn get(&self, key: &K) -> Option<&V> {
        HashMap::get(self, key)
    }
}

// Implement MapGet for FxHashMap
impl<K, V> MapGet<K, V> for FxHashMap<K, V>
where
    K: Eq + std::hash::Hash,
{
    fn get(&self, key: &K) -> Option<&V> {
        FxHashMap::get(self, key)
    }
}

// Implement MapGet for IndexMap
impl<K, V> MapGet<K, V> for IndexMap<K, V>
where
    K: Eq + std::hash::Hash,
{
    fn get(&self, key: &K) -> Option<&V> {
        IndexMap::get(self, key)
    }
}

// ExtractionResult struct represents the result of an extraction operation
#[derive(Default, Clone, Deserialize, Serialize)]
pub struct ExtractionResult {
    // choices is an IndexMap that maps ClassId to NodeId
    pub choices: IndexMap<ClassId, NodeId>,
}

// Cost_extract struct is an empty struct (placeholder)
pub struct Cost_extract {}

// Status enum represents the status of a node during cycle detection
#[derive(Clone, Copy)]
enum Status {
    Doing,
    Done,
}

#[derive(Serialize)]
struct Wrapper<T> {
    choices: T,
}


// Implement methods for ExtractionResult
impl ExtractionResult {
    // choose method inserts a mapping from ClassId to NodeId into the choices map
    pub fn choose(&mut self, class_id: ClassId, node_id: NodeId) {
        self.choices.insert(class_id, node_id);
    }

    // find_cycles method finds cycles in the EGraph starting from the given roots
    // and returns a vector of ClassIds representing the cycles
    pub fn find_cycles(&self, egraph: &EGraph, roots: &[ClassId]) -> Vec<ClassId> {
        let mut status = IndexMap::<ClassId, Status>::default();
        let mut cycles = vec![];
        for root in roots {
            self.cycle_dfs(egraph, root, &mut status, &mut cycles)
        }
        cycles
    }

    // cycle_dfs method performs a depth-first search to detect cycles in the EGraph
    fn cycle_dfs(
        &self,
        egraph: &EGraph,
        class_id: &ClassId,
        status: &mut IndexMap<ClassId, Status>,
        cycles: &mut Vec<ClassId>,
    ) {
        match status.get(class_id).cloned() {
            Some(Status::Done) => (),
            Some(Status::Doing) => cycles.push(class_id.clone()),
            None => {
                status.insert(class_id.clone(), Status::Doing);
                let node_id = &self.choices[class_id];
                let node = &egraph[node_id];
                for child in &node.children {
                    let child_cid = egraph.nid_to_cid(child);
                    self.cycle_dfs(egraph, child_cid, status, cycles)
                }
                status.insert(class_id.clone(), Status::Done);
            }
        }
    }

    // tree_cost method calculates the cost of the extracted tree
    pub fn tree_cost(&self, egraph: &EGraph, roots: &[ClassId]) -> Cost {
        let node_roots = roots
            .iter()
            .map(|cid| self.choices[cid].clone())
            .collect::<Vec<NodeId>>();
        self.tree_cost_rec(egraph, &node_roots, &mut HashMap::new())
    }

    // tree_cost_rec method recursively calculates the cost of the extracted tree
    fn tree_cost_rec(
        &self,
        egraph: &EGraph,
        roots: &[NodeId],
        memo: &mut HashMap<NodeId, Cost>,
    ) -> Cost {
        let mut cost = Cost::default();
        for root in roots {
            if let Some(c) = memo.get(root) {
                cost += *c;
                continue;
            }
            let class = egraph.nid_to_cid(root);
            let node = &egraph[&self.choices[class]];
            let inner = node.cost + self.tree_cost_rec(egraph, &node.children, memo);
            memo.insert(root.clone(), inner);
            cost += inner;
        }
        cost
    }

    // dag_cost method calculates the cost of the extracted directed acyclic graph (DAG)
    pub fn dag_cost(&self, egraph: &EGraph, roots: &[ClassId]) -> Cost {
        let mut costs: IndexMap<ClassId, Cost> = IndexMap::new();
        let mut todo: Vec<ClassId> = roots.to_vec();
        while let Some(cid) = todo.pop() {
            let node_id = &self.choices[&cid];
            let node = &egraph[node_id];
            if costs.insert(cid.clone(), node.cost).is_some() {
                continue;
            }
            for child in &node.children {
                todo.push(egraph.nid_to_cid(child).clone());
            }
        }
        costs.values().sum()
    }

    // dag_cost_with_extraction_result method calculates the cost of the extracted DAG
    // and returns the cost along with the extraction result
    pub fn calculate_dag_cost_with_extraction_result(
        &self,
        egraph: &EGraph,
        roots: &[ClassId],
    ) -> (Cost, ExtractionResult) {
        let mut costs: IndexMap<ClassId, Cost> = IndexMap::new();
        let mut todo: Vec<ClassId> = roots.to_vec();
        let mut extraction_result = ExtractionResult {
            choices: IndexMap::new(),
        };

        while let Some(cid) = todo.pop() {
            let node_id = &self.choices[&cid];
            let node = &egraph[node_id];
            if costs.insert(cid.clone(), node.cost).is_some() {
                continue;
            }
            extraction_result
                .choices
                .insert(cid.clone(), node_id.clone());
            for child in &node.children {
                todo.push(egraph.nid_to_cid(child).clone());
            }
        }

        let total_cost = costs.values().sum(); // calculate total cost

        (total_cost, extraction_result)
    }

    // node_sum_cost method calculates the sum of the costs of a node and its children
    pub fn node_sum_cost<M>(&self, egraph: &EGraph, node: &Node, costs: &M) -> Cost
    where
        M: MapGet<ClassId, Cost>,
    {
        node.cost
            + node
                .children
                .iter()
                .map(|n| {
                    let cid = egraph.nid_to_cid(n);
                    costs.get(cid).unwrap_or(&INFINITY)
                })
                .sum::<Cost>()
    }

    // node_depth_cost method calculates the maximum cost among a node and its children
    pub fn node_depth_cost<M>(&self, egraph: &EGraph, node: &Node, costs: &M) -> Cost
    where
        M: MapGet<ClassId, Cost>,
    {
        let child_max_cost = node
            .children
            .iter()
            .map(|n| {
                let cid = egraph.nid_to_cid(n);
                costs.get(cid).unwrap_or(&INFINITY)
            })
            .max()
            .copied()
            .unwrap_or(Cost::default());

        node.cost + child_max_cost
    }

    // record_costs_random method records the costs of random extractions
    pub fn record_costs_random(
        &self,
        num_runs: u32,
        random_ratio: f64,
        egraph: &EGraph,
        dag_cost_with_extraction_result: &ExtractionResult,
    ) {
        let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);

        for num in 0..num_runs {
            // dump dag_cost_with_extraction_result to file
            
            let mut result: FxHashMap<ClassId, NodeId> = FxHashMap::default();
            let mut selected_ids: FxHashSet<ClassId> = HashSet::default(); // used to track selected nodes
            for classid in dag_cost_with_extraction_result.choices.keys() {
                let class = egraph.classes().get(classid).unwrap();
                let nodes = class.nodes.clone();
                let mut rng = rand::thread_rng();

                if !selected_ids.contains(&classid)
                    && rng.gen::<f64>() <= random_ratio
                    && nodes.len() > 1
                {
                    let random_index = rng.gen_range(0..nodes.len());
                    result.insert(classid.clone(), class.nodes[random_index].clone());

                    // Add the selected node ID to the set
                    selected_ids.insert(classid.clone());
                } else {
                    result.insert(
                        class.id.clone(),
                        dag_cost_with_extraction_result.choices[classid].clone(),
                    );
                }
            }

            let filename = format!("result{}.json", num);
            let path = format!("random_result/{}", filename);

            // Create directory if it doesn't exist
            if let Err(err) = fs::create_dir_all("random_result") {
                eprintln!("Failed to create directory: {}", err);
                continue; // Skip current iteration if directory creation fails
            }

            let wrapped_result = Wrapper {
                choices: result,
            };
        

            if let Ok(mut file) = File::create(path) {
                match to_string_pretty(&wrapped_result) {
                    Ok(json_dag_result) => {
                        let _ = write!(file, "{}", json_dag_result);
                    },
                    Err(e) => eprintln!("Failed to serialize data: {}", e),
                }
            }

        }
    }
}