use indexmap::IndexMap;
use rustc_hash::{FxHashMap, FxHashSet};
use std::{collections::HashMap, default};
use egraph_serialize::*;
use ordered_float::NotNan;
pub const INFINITY: Cost = unsafe { NotNan::new_unchecked(std::f64::INFINITY) };
pub mod bottom_up;
pub mod faster_ast_depth;
pub mod faster_ast_depth_mt;
pub mod faster_bottom_up;
pub mod faster_bottom_up_mt;
pub mod faster_greedy_dag;
pub mod faster_greedy_dag_mt1;
pub mod faster_greedy_dag_mt2;
pub mod my_ilp;
// pub mod faster_greedy_dag_fa;
// pub mod faster_greedy_dag_fa_mt;
#[cfg(feature = "ilp-cbc")]
pub mod faster_ilp_cbc;
pub mod global_greedy_dag;
pub mod greedy_dag;
#[cfg(feature = "ilp-cbc")]
pub mod ilp_cbc;

// Allowance for floating point values to be considered equal
pub const EPSILON_ALLOWANCE: f64 = 0.00001;

pub trait Extractor: Sync {
    fn extract(&self, egraph: &EGraph, roots: &[ClassId]) -> ExtractionResult;

    fn boxed(self) -> Box<dyn Extractor>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

pub trait MapGet<K, V> {
    fn get(&self, key: &K) -> Option<&V>;
}

impl<K, V> MapGet<K, V> for HashMap<K, V>
where
    K: Eq + std::hash::Hash,
{
    fn get(&self, key: &K) -> Option<&V> {
        HashMap::get(self, key)
    }
}

impl<K, V> MapGet<K, V> for FxHashMap<K, V>
where
    K: Eq + std::hash::Hash,
{
    fn get(&self, key: &K) -> Option<&V> {
        FxHashMap::get(self, key)
    }
}

impl<K, V> MapGet<K, V> for IndexMap<K, V>
where
    K: Eq + std::hash::Hash,
{
    fn get(&self, key: &K) -> Option<&V> {
        IndexMap::get(self, key)
    }
}



#[derive(Default, Clone)]
pub struct ExtractionResult {
    pub choices: IndexMap<ClassId, NodeId>,
    pub cost: HashMap<NodeId, Cost>,
}

#[derive(Clone, Copy)]
enum Status {
    Doing,
    Done,
}

impl ExtractionResult {

    pub fn find_shortest_cycle(&self, egraph: &EGraph, roots: &[ClassId]) -> Option<Vec<ClassId>> {
        let mut shortest_cycle: Option<Vec<ClassId>> = None;
        let mut status = IndexMap::<ClassId, Status>::default();
        let mut stack = Vec::<ClassId>::new();
        for root in roots {
            self.cycle_dfs_shortest_path(egraph, root, &mut status, &mut stack, &mut shortest_cycle);
        }
        shortest_cycle
    }

    pub fn new_empty() -> Self {
        Self {
            choices: IndexMap::<ClassId, NodeId>::default(),
            cost: HashMap::new(),
        }
    }

    pub fn new(choices:IndexMap<ClassId, NodeId>) -> Self {
        Self {
            choices: choices,
            cost: HashMap::new(),
        }
    }

    pub fn check(&self, egraph: &EGraph) {
        // should be a root
        assert!(!egraph.root_eclasses.is_empty());

        // All roots should be selected.
        for cid in egraph.root_eclasses.iter() {
            // println!("cid:{:?}",cid);
            assert!(self.choices.contains_key(cid));
        }


        // Nodes should match the class they are selected into.
        for (cid, nid) in &self.choices {
            let node = &egraph[nid];
            assert!(node.eclass == *cid);
        }

        // All the nodes the roots depend upon should be selected.
        let mut todo: Vec<ClassId> = egraph.root_eclasses.to_vec();
        let mut visited: FxHashSet<ClassId> = Default::default();
        while let Some(cid) = todo.pop() {
            if !visited.insert(cid.clone()) {
                continue;
            }
            assert!(self.choices.contains_key(&cid));

            for child in &egraph[&self.choices[&cid]].children {
                todo.push(child.clone());
            }
        }


        if !self.find_cycles(&egraph, &egraph.root_eclasses).is_empty() {
            if let Some(shortest_cycle) = self.find_shortest_cycle(&egraph, &egraph.root_eclasses) {
                println!("shortest cycle: {:?}", shortest_cycle);
            }
            assert!(false);
        }
        
    }

    pub fn choose(&mut self, class_id: ClassId, node_id: NodeId) {
        self.choices.insert(class_id, node_id);
    }

    pub fn find_cycles(&self, egraph: &EGraph, roots: &[ClassId]) -> Vec<ClassId> {
        // let mut status = vec![Status::Todo; egraph.classes().len()];
        let mut status = IndexMap::<ClassId, Status>::default();
        let mut cycles = vec![];
        for root in roots {
            // let root_index = egraph.classes().get_index_of(root).unwrap();
            self.cycle_dfs(egraph, root, &mut status, &mut cycles)
        }
        cycles
    }

    fn cycle_dfs_shortest_path(
        &self,
        egraph: &EGraph,
        class_id: &ClassId,
        status: &mut IndexMap<ClassId, Status>,
        stack: &mut Vec<ClassId>, // 记录当前遍历路径
        shortest_cycle: &mut Option<Vec<ClassId>>, // 记录目前找到的最短环路径
    ) {
        match status.get(class_id).cloned() {
            Some(Status::Done) => (),
            Some(Status::Doing) => {
                // 找到环，提取完整路径
                if let Some(pos) = stack.iter().position(|x| x == class_id) {
                    let cycle = stack[pos..].to_vec();
                    // 如果还没有找到环，或当前环比之前的更短，则更新
                    if shortest_cycle.is_none() || cycle.len() < shortest_cycle.as_ref().unwrap().len() {
                        *shortest_cycle = Some(cycle);
                    }
                }
            }
            None => {
                status.insert(class_id.clone(), Status::Doing);
                stack.push(class_id.clone()); // 记录访问路径
                let node_id = &self.choices[class_id];
                let node = &egraph[node_id];
    
                for child in &node.children {
                    self.cycle_dfs_shortest_path(egraph, child, status, stack, shortest_cycle);
                }
    
                stack.pop(); // 回溯
                status.insert(class_id.clone(), Status::Done);
            }
        }
    }
    

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
                    // let child_cid = egraph.nid_to_cid(child);
                    self.cycle_dfs(egraph, child, status, cycles)
                }
                status.insert(class_id.clone(), Status::Done);
            }
        }
    }

    pub fn depth_cost(&self, egraph: &EGraph, roots: &[ClassId]) -> u32 {
        let mut memo = HashMap::<ClassId, u32>::new();
        roots
            .iter()
            .map(|cid| self.depth_cost_rec(egraph, cid, &mut memo))
            .max()
            .unwrap_or(0)
    }

    // 递归计算某个等价类的深度成本（深度），使用 memo 进行记忆化计算。
    fn depth_cost_rec(
        &self,
        egraph: &EGraph,
        cid: &ClassId,
        memo: &mut HashMap<ClassId, u32>,
    ) -> u32 {
        if let Some(&cost) = memo.get(cid) {
            return cost;
        }
        let node_id = &self.choices[cid];
        let node = &egraph[node_id];
        // 对于当前节点，深度 = 1 + (其所有子节点深度的最大值)
        let child_max = node
            .children
            .iter()
            .map(|child_cid| self.depth_cost_rec(egraph, child_cid, memo))
            .max()
            .unwrap_or(0);
        let cost = 1 + child_max;
        memo.insert(cid.clone(), cost);
        // println!("nid:{:?},cost:{:?}",node_id,cost);
        cost
    }

    pub fn tree_cost(&self, egraph: &EGraph, roots: &[ClassId]) -> Cost {
        let node_roots = roots
            .iter()
            .map(|cid| cid.clone())
            .collect::<Vec<ClassId>>();
        self.tree_cost_rec(egraph, &node_roots, &mut HashMap::new())
    }

    
    pub fn activate_nodes(&self, egraph: &EGraph, roots: &[ClassId]) -> FxHashSet<NodeId> {
        let node_roots = roots
        .iter()
        .map(|cid| cid.clone())
        .collect::<Vec<ClassId>>();
        let mut memo = FxHashSet::default();
        self.activate_nodes_rec(egraph, &node_roots, &mut memo);
        memo
    }


    fn activate_nodes_rec(
        &self,
        egraph: &EGraph,
        roots: &[ClassId],
        memo: &mut FxHashSet<NodeId>,
    ) {
        for root in roots {
            let node = &egraph[&self.choices[root]];
            if let Some(c) = memo.get(&node.id) {
                continue;
            }
            memo.insert(node.id);
            self.activate_nodes_rec(egraph, &node.children, memo);
        }
    }


    fn tree_cost_rec(
        &self,
        egraph: &EGraph,
        roots: &[ClassId],
        memo: &mut HashMap<ClassId, Cost>,
    ) -> Cost {
        let mut cost = Cost::default();
        for root in roots {
            if let Some(c) = memo.get(root) {
                cost += *c;
                continue;
            }
            // let class = egraph.nid_to_cid(root);
            let node = &egraph[&self.choices[root]];
            let inner = node.cost + self.tree_cost_rec(egraph, &node.children, memo);
            memo.insert(root.clone(), inner);
            cost += inner;
        }
        cost
    }

    // this will loop if there are cycles
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
                todo.push(child.clone());
            }
        }
        costs.values().sum()
    }

    pub fn node_sum_cost<M>(&self, egraph: &EGraph, node: &Node, costs: &M) -> Cost
    where
        M: MapGet<ClassId, Cost>,
    {
        node.cost
            + node
                .children
                .iter()
                .map(|n| {
                    // let cid = egraph.nid_to_cid(n);
                    costs.get(n).unwrap_or(&INFINITY)
                })
                .sum::<Cost>()
    }
}
