// Calculates the cost where shared nodes are just costed once,
// For example (+ (* x x ) (* x x )) has one mulitplication
// included in the cost.

use crate::*;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Clone, Debug)]
struct CostSet {
    // It's slightly faster if this is an HashMap rather than an fxHashMap.
    costs: HashMap<ClassId, Cost>,
    total: Cost,
    choice: NodeId,
}

pub struct FasterGreedyDagExtractor;

impl FasterGreedyDagExtractor {
    fn calculate_cost_set(
        egraph: &EGraph,
        node_id: NodeId,
        costs_all: &FxHashMap::<ClassId, (CostSet,CostSet)>,
        best_cost: Cost,
    ) -> CostSet {
        let node = &egraph[&node_id];
        let cid = egraph.nid_to_cid(&node_id);

        if node.children.is_empty() {
            return CostSet {
                costs: HashMap::from([(cid.clone(), node.cost)]),
                total: node.cost,
                choice: node_id.clone(),
            };
        }

        // Get unique classes of children.
        let mut childrens_classes = node
            .children
            .iter()
            .map(|c| egraph.nid_to_cid(&c).clone())
            .collect::<Vec<ClassId>>();
        childrens_classes.sort();
        childrens_classes.dedup();

        let first_cost = costs_all.get(&childrens_classes[0]).unwrap();

        if childrens_classes.contains(cid)
            || (childrens_classes.len() == 1 && (node.cost + first_cost.0.total < best_cost))
        {
            // Shortcut. Can't be cheaper so return junk.
            return CostSet {
                costs: Default::default(),
                total: -INFINITY,
                choice: node_id.clone(),
            };
        }

        // Clone the biggest set and insert the others into it.
        let id_of_biggest = childrens_classes
            .iter()
            .max_by_key(|s| costs_all.get(s).unwrap().0.costs.len())
            .unwrap();
        let mut result = costs_all.get(&id_of_biggest).unwrap().0.costs.clone();
        for child_cid in &childrens_classes {
            if child_cid == id_of_biggest {
                continue;
            }

            let next_cost = &costs_all.get(child_cid).unwrap().0.costs;
            for (key, value) in next_cost.iter() {
                result.insert(key.clone(), value.clone());
            }
        }

        let contains = result.contains_key(&cid);
        result.insert(cid.clone(), node.cost);

        let result_cost = if contains {
            -INFINITY
        } else {
            result.values().sum()
        };

        return CostSet {
            costs: result,
            total: result_cost,
            choice: node_id.clone(),
        };
    }
}

fn combined_costset(costset1: &CostSet, cid2: &ClassId, costs_all: &FxHashMap::<ClassId, (CostSet,CostSet)>, mode: bool) -> (Cost,CostSet) {

    let prev_costs1 = costset1.costs.clone();

    let mut prev_costs2;
    if costs_all.contains_key(cid2) {
        if mode {
            prev_costs2 = costs_all.get(&cid2).unwrap().0.clone();
        }
        else{
            prev_costs2 = costs_all.get(&cid2).unwrap().1.clone();
        }
    }
    else {
        prev_costs2 = CostSet {
            costs: Default::default(),
            total: -INFINITY,
            choice: NodeId::new(),
        };
    }
    let mut combined_costs = prev_costs1.clone();
    for (key, value) in prev_costs2.costs.iter() {
        combined_costs.insert(key.clone(), *value);
    }

    // let cost1 = costset1.total;
    // let cost2 = costs_all.get(cid2).unwrap().0.total;
    return (combined_costs.values().sum(),prev_costs2);
}

impl Extractor for FasterGreedyDagExtractor {
    fn extract(&self, egraph: &EGraph, _roots: &[ClassId]) -> ExtractionResult {
        let mut parents = IndexMap::<ClassId, Vec<NodeId>>::with_capacity(egraph.classes().len());
        let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);
        let mut analysis_pending = UniqueQueue::default();

        let mut xor_op: FxHashSet<NodeId> = FxHashSet::default();
        let mut xor_op_class: FxHashSet<&ClassId> = FxHashSet::default();
        let mut xor_nmap: FxHashMap<NodeId, NodeId> = FxHashMap::default();
        let mut maj_op: FxHashSet<NodeId> = FxHashSet::default();
        let mut maj_op_class: FxHashSet<&ClassId> = FxHashSet::default();
        let mut maj_nmap: FxHashMap<NodeId, NodeId> = FxHashMap::default();
        let mut fa_op: FxHashSet<NodeId> = FxHashSet::default();
        let mut fa_op_class: FxHashSet<&ClassId> = FxHashSet::default();
        let mut fst_op: FxHashSet<NodeId> = FxHashSet::default();
        let mut snd_op: FxHashSet<NodeId> = FxHashSet::default();
        let mut fst_nmap: FxHashMap<NodeId, NodeId> = FxHashMap::default();
        let mut fst_op_class: FxHashSet<&ClassId> = FxHashSet::default();
        let mut snd_nmap: FxHashMap<NodeId, NodeId> = FxHashMap::default();
        let mut snd_op_class: FxHashSet<&ClassId> = FxHashSet::default();

        for (node_id, node) in &egraph.nodes {
            if node.op == "xor3" {
                xor_op.insert(node_id.clone());
                xor_op_class.insert(n2c(node_id));
            } else if node.op == "maj" {
                maj_op.insert(node_id.clone());
                maj_op_class.insert(n2c(node_id));
            } else if node.op == "fa" {
                fa_op.insert(node_id.clone());
                fa_op_class.insert(n2c(node_id));
            } else if node.op == "fst" {
                fst_op.insert(node_id.clone());
                fst_op_class.insert(n2c(node_id));
            } else if node.op == "snd" {
                snd_op.insert(node_id.clone());
                snd_op_class.insert(n2c(node_id));
            }
        }

        let mut i = 0;
        for xor in &xor_op {
            for maj in &maj_op {
                i=i+1;
                if egraph.nodes[xor].children == egraph.nodes[maj].children {
                    xor_nmap.insert(xor.clone(), maj.clone());
                    maj_nmap.insert(maj.clone(), xor.clone());
                }
            }
        }

        for fst in &fst_op {
            for snd in &snd_op {
                if egraph.nodes[fst].children == egraph.nodes[snd].children {
                    fst_nmap.insert(fst.clone(), snd.clone());
                    snd_nmap.insert(snd.clone(), fst.clone());
                }
            }
        }
        // println!("XOR: {}", xor_map);
        // println!("XOR: {}", maj_op.len());
        // println!("XOR: {}", maj_op.len());
        // panic!("XOR: {:?}", xor_map);

        for class in egraph.classes().values() {
            parents.insert(class.id.clone(), Vec::new());
        }

        for class in egraph.classes().values() {
            for node in &class.nodes {
                for c in &egraph[node].children {
                    // compute parents of this enode
                    parents[n2c(c)].push(node.clone());
                }

                // start the analysis from leaves
                if egraph[node].is_leaf() {
                    analysis_pending.insert(node.clone());
                }
            }
        }

        let mut result = ExtractionResult::default();
        let mut costs_all = FxHashMap::<ClassId, (CostSet,CostSet)>::with_capacity_and_hasher(
            egraph.classes().len(),
            Default::default(),
        );

        // println!("fst_op_class: {:?}", fst_op_class);
        // println!("snd_op_class: {:?}", snd_op_class);

        while let Some(node_id) = analysis_pending.pop() {
            let class_id = n2c(&node_id);
            let node = &egraph[&node_id];
            // println!("{:?}", node.op);
            if node.children.iter().all(|c| costs_all.contains_key(n2c(c))) {
                let lookup = costs_all.get(class_id);
                // let mut prev_cost = -INFINITY;
                let mut prev_costset0 = CostSet {
                    costs: Default::default(),
                    total: -INFINITY,
                    choice: NodeId::new(),
                };

                if lookup.is_some() {
                    prev_costset0 = lookup.unwrap().0.clone();
                }

                let mut prev_costset1 = CostSet {
                    costs: Default::default(),
                    total: -INFINITY,
                    choice: NodeId::new(),
                };

                if lookup.is_some() {
                    prev_costset1 = lookup.unwrap().1.clone();
                }
                let cost_set = Self::calculate_cost_set(egraph, node_id.clone(), &costs_all, prev_costset0.total);
                // if node class is maj_class
                if snd_op_class.contains(class_id) {
                    // println!("{:?}", node.op);
                    if node.op == "snd" {
                        // Case 1: Node is snd and the previous node is not snd
                        if prev_costset0.choice.as_ref() == "None" || egraph.nodes[&prev_costset0.choice].op != "snd"{
                            // let cid2= maj_map.get(n2c(&node_id)).unwrap();
                            let cid2 = n2c(snd_nmap.get(&node_id).unwrap());
                            let mut cid4 = n2c(&node_id);
                            if fst_nmap.contains_key(&costs_all.get(&cid2).unwrap().0.choice){
                                cid4 = n2c(fst_nmap.get(&costs_all.get(&cid2).unwrap().0.choice).unwrap());
                            }
                            let total1 = cost_set.total;
                            let (total2,prev_costset2) = combined_costset(&prev_costset0, cid2, &costs_all, false);
                            if total1 > total2 {
                                costs_all.insert(class_id.clone(), (cost_set.clone(),prev_costset1.clone()));
                                analysis_pending.extend(parents[class_id].iter().cloned());
                                let mut costs2=cost_set.clone().costs;
                                costs2.insert(cid2.clone(), egraph.nodes[&node_id].cost);
                                costs2.remove(class_id);
                                let CostSet2=CostSet {
                                    costs: costs2,
                                    total: cost_set.total,
                                    choice: snd_nmap.get(&node_id).unwrap().clone(),
                                };
                                costs_all.insert(cid2.clone(), (CostSet2.clone(),costs_all.get(cid2).unwrap().1.clone()));
                                analysis_pending.extend(parents[cid2].iter().cloned());
                                // print!("11 {:?}-{:?} {:?}-{:?}", class_id,cost_set.choice, cid2,CostSet2.choice);

                                if cid4 != n2c(&node_id) {
                                    let costset4=costs_all.get(cid4).unwrap().1.clone();
                                    costs_all.insert(cid4.clone(), (costset4.clone(),costset4.clone()));
                                    analysis_pending.extend(parents[cid4].iter().cloned());
                                    // print!(" {:?}-{:?}", cid4,costset4.choice);
                                }

                                // println!();
                            }
                        }
                        // Case 2: Node is snd and the previous node is snd
                        else{
                            if cost_set.total > prev_costset0.total {
                                let cid2 = n2c(snd_nmap.get(&node_id).unwrap());
                                let cid3 = n2c(snd_nmap.get(&prev_costset0.choice).unwrap());
                                let mut cid4 = n2c(&node_id);
                                if fst_nmap.contains_key(&costs_all.get(&cid2).unwrap().0.choice){
                                    cid4 = n2c(fst_nmap.get(&costs_all.get(&cid2).unwrap().0.choice).unwrap());
                                }
                                costs_all.insert(class_id.clone(), (cost_set.clone(),prev_costset1.clone()));
                                analysis_pending.extend(parents[class_id].iter().cloned());
                                let mut costs2=cost_set.clone().costs;
                                costs2.insert(cid2.clone(), egraph.nodes[&node_id].cost);
                                costs2.remove(class_id);
                                let CostSet2=CostSet {
                                    costs: costs2,
                                    total: cost_set.total,
                                    choice: snd_nmap.get(&node_id).unwrap().clone(),
                                };
                                costs_all.insert(cid2.clone(), (CostSet2.clone(),costs_all.get(cid2).unwrap().1.clone()));
                                analysis_pending.extend(parents[cid2].iter().cloned());
                                // print!("12 {:?}-{:?} {:?}-{:?}", class_id,cost_set.choice, cid2,CostSet2.choice);
                                if cid2 != cid3 {
                                    let costset3=costs_all.get(cid3).unwrap().1.clone();
                                    costs_all.insert(cid3.clone(), (costset3.clone(),costset3.clone()));
                                    analysis_pending.extend(parents[cid3].iter().cloned());
                                    // print!(" {:?}-{:?}", cid3,costset3.choice);
                                }

                                if cid4 != n2c(&node_id) {
                                    let costset4=costs_all.get(cid4).unwrap().1.clone();
                                    costs_all.insert(cid4.clone(), (costset4.clone(),costset4.clone()));
                                    analysis_pending.extend(parents[cid4].iter().cloned());
                                    // print!(" {:?}-{:?}", cid4,costset4.choice);
                                }

                                // println!();
                            }
                            // let cid1 = n2c(&node_id);
                            // let cid2= maj_map.get(n2c(&node_id)).unwrap();
                            // let cid3 = maj_map.get(n2c(&prev_costset0.choice)).unwrap();
                            // if cid2 == cid3 {
                            //     if cost_set.total > prev_costset0.total {
                            //         costs_all.insert(class_id.clone(), (cost_set.clone(),prev_costset1.clone()));
                            //         analysis_pending.extend(parents[class_id].iter().cloned());
                            //         let mut costs2=cost_set.clone().costs;
                            //         costs2.insert(cid2.clone(), egraph.nodes[&node_id].cost);
                            //         costs2.remove(class_id);
                            //         let CostSet2=CostSet {
                            //             costs: costs2,
                            //             total: cost_set.total,
                            //             choice: snd_nmap.get(&node_id).unwrap().clone(),
                            //         };
                            //         costs_all.insert(cid2.clone(), (CostSet2.clone(),costs_all.get(cid2).unwrap().1.clone()));
                            //         analysis_pending.extend(parents[cid2].iter().cloned());
                            //         println!("12 {:?}-{:?} {:?}-{:?}", class_id,cost_set.choice, cid2,CostSet2.choice);
                            //     }
                            // }
                            // else {
                            // }
                        }
                    }
                    else if node.op != "fst" {
                        // Case 3: Node is not snd and the previous node 0 is not snd
                        let mut flag = true;
                        if prev_costset0.choice.as_ref() == "None" || egraph.nodes[&prev_costset0.choice].op != "snd"{
                            if cost_set.total > prev_costset0.total {
                                costs_all.insert(class_id.clone(), (cost_set.clone(),cost_set.clone()));
                                analysis_pending.extend(parents[class_id].iter().cloned());
                                flag = false;
                                // println!("13 {:?}-{:?}", class_id,cost_set.choice);
                            }
                        }
                        // Case 4: Node is not snd and the previous node is snd
                        else{
                            let cid2 = n2c(snd_nmap.get(&prev_costset0.choice).unwrap());
                            let (total1,prev_costset2) = combined_costset(&cost_set, cid2, &costs_all, true);
                            if total1 > prev_costset0.total {
                                costs_all.insert(class_id.clone(), (cost_set.clone(),cost_set.clone()));
                                analysis_pending.extend(parents[class_id].iter().cloned());
                                // update the costset of xor
                                let costset2=costs_all.get(cid2).unwrap().1.clone();
                                costs_all.insert(cid2.clone(),(costset2.clone(),costset2.clone()));
                                analysis_pending.extend(parents[cid2].iter().cloned());
                                flag = false;
                                // println!("14 {:?}-{:?} {:?}-{:?}", class_id,cost_set.choice, cid2,costset2.choice);
                            }
                        }
                        // If the node total is less than the previous node 0 total, then we need to check the previous node 1, if it is not snd, then we can update the costset.
                        if flag {
                            if prev_costset1.choice.as_ref() == "None" || egraph.nodes[&prev_costset1.choice].op != "snd"{
                                if cost_set.total > prev_costset1.total {
                                    costs_all.insert(class_id.clone(), (prev_costset0.clone(),cost_set));
                                    // println!("15 {:?}-{:?}", class_id,prev_costset0.choice);
                                }
                            }
                        }
                    }
                }
                else if fst_op_class.contains(class_id) {
                    // println!("{:?}", node.op);
                    if node.op == "fst" {
                        // Case 1: Node is fst and the previous node is not fst
                        if prev_costset0.choice.as_ref() == "None" || egraph.nodes[&prev_costset0.choice].op != "fst"{
                            let cid2 = n2c(fst_nmap.get(&node_id).unwrap());
                            let mut cid4 = n2c(&node_id);
                            if fst_nmap.contains_key(&costs_all.get(&cid2).unwrap().0.choice){
                                cid4 = n2c(fst_nmap.get(&costs_all.get(&cid2).unwrap().0.choice).unwrap());
                            }
                            let total1 = cost_set.total;
                            let (total2,_) = combined_costset(&prev_costset0, cid2, &costs_all, false);
                            if total1 > total2 {
                                costs_all.insert(class_id.clone(), (cost_set.clone(),prev_costset1.clone()));
                                analysis_pending.extend(parents[class_id].iter().cloned());
                                let mut costs2=cost_set.clone().costs;
                                costs2.insert(cid2.clone(), egraph.nodes[&node_id].cost);
                                costs2.remove(class_id);
                                let CostSet2=CostSet {
                                    costs: costs2,
                                    total: cost_set.total,
                                    choice: fst_nmap.get(&node_id).unwrap().clone(),
                                };
                                costs_all.insert(cid2.clone(), (CostSet2.clone(),costs_all.get(cid2).unwrap().1.clone()));
                                analysis_pending.extend(parents[cid2].iter().cloned());
                                // print!("21 {:?}-{:?} {:?}-{:?}", class_id,cost_set.choice, cid2,CostSet2.choice);
                                

                                if cid4 != n2c(&node_id) {
                                    let costset4=costs_all.get(cid4).unwrap().1.clone();
                                    costs_all.insert(cid4.clone(), (costset4.clone(),costset4.clone()));
                                    analysis_pending.extend(parents[cid4].iter().cloned());
                                    // print!(" {:?}-{:?}", cid4,costset4.choice);
                                }

                                // println!();
                            }
                        }
                        // Case 2: Node is fst and the previous node is fst
                        else{
                            if cost_set.total > prev_costset0.total {
                                let cid2 = n2c(fst_nmap.get(&node_id).unwrap());
                                let cid3 = n2c(fst_nmap.get(&prev_costset0.choice).unwrap());
                                let mut cid4 = n2c(&node_id);
                                if fst_nmap.contains_key(&costs_all.get(&cid2).unwrap().0.choice){
                                    cid4 = n2c(fst_nmap.get(&costs_all.get(&cid2).unwrap().0.choice).unwrap());
                                }
                                costs_all.insert(class_id.clone(), (cost_set.clone(),prev_costset1.clone()));
                                analysis_pending.extend(parents[class_id].iter().cloned());
                                let mut costs2=cost_set.clone().costs;
                                costs2.insert(cid2.clone(), egraph.nodes[&node_id].cost);
                                costs2.remove(class_id);
                                let CostSet2=CostSet {
                                    costs: costs2,
                                    total: cost_set.total,
                                    choice: fst_nmap.get(&node_id).unwrap().clone(),
                                };
                                costs_all.insert(cid2.clone(), (CostSet2.clone(),costs_all.get(cid2).unwrap().1.clone()));
                                analysis_pending.extend(parents[cid2].iter().cloned());
                                // print!("22 {:?}-{:?} {:?}-{:?}", class_id,cost_set.choice, cid2,CostSet2.choice);
                                if cid2 != cid3 {
                                    let costset3=costs_all.get(cid3).unwrap().1.clone();
                                    costs_all.insert(cid3.clone(), (costset3.clone(),costset3.clone()));
                                    analysis_pending.extend(parents[cid3].iter().cloned());
                                    // print!(" {:?}-{:?}", cid3,costset3.choice);
                                }

                                if cid4 != n2c(&node_id) {
                                    let costset4=costs_all.get(cid4).unwrap().1.clone();
                                    costs_all.insert(cid4.clone(), (costset4.clone(),costset4.clone()));
                                    analysis_pending.extend(parents[cid4].iter().cloned());
                                    // print!(" {:?}-{:?}", cid4,costset4.choice);
                                }

                                // println!();
                            }
                        }

                    }
                    else if node.op != "snd"{
                        // Case 3: Node is not fst and the previous node 0 is not fst
                        let mut flag = true;
                        if prev_costset0.choice.as_ref() == "None" || egraph.nodes[&prev_costset0.choice].op != "fst"{
                            if cost_set.total > prev_costset0.total {
                                costs_all.insert(class_id.clone(), (cost_set.clone(),cost_set.clone()));
                                analysis_pending.extend(parents[class_id].iter().cloned());
                                flag = false;
                                // println!("23 {:?}-{:?}", class_id,cost_set.choice);
                            }
                        }
                        // Case 4: Node is not fst and the previous node is fst
                        else{
                            let cid2 = n2c(fst_nmap.get(&prev_costset0.choice).unwrap());
                            let (total1,prev_costset2) = combined_costset(&cost_set, cid2, &costs_all, true);
                            if total1 > prev_costset0.total {
                                costs_all.insert(class_id.clone(), (cost_set.clone(),cost_set.clone()));
                                analysis_pending.extend(parents[class_id].iter().cloned());
                                // update the costset of xor
                                let costset2=costs_all.get(cid2).unwrap().1.clone();
                                costs_all.insert(cid2.clone(),(costset2.clone(),costset2.clone()));
                                analysis_pending.extend(parents[cid2].iter().cloned());
                                flag = false;
                                // println!("24 {:?}-{:?} {:?}-{:?}", class_id,cost_set.choice, cid2,costset2.choice);
                            }
                        }
                        // If the node total is less than the previous node 0 total, then we need to check the previous node 1, if it is not fst, then we can update the costset.
                        if flag {
                            if prev_costset1.choice.as_ref() == "None" || egraph.nodes[&prev_costset1.choice].op != "fst"{
                                if cost_set.total > prev_costset1.total {
                                    costs_all.insert(class_id.clone(), (prev_costset0.clone(),cost_set));
                                    // println!("25 {:?}-{:?}", class_id,prev_costset0.choice);
                                }
                            }
                        }
                    }
                }
                else if cost_set.total > prev_costset0.total {
                    costs_all.insert(class_id.clone(), (cost_set,prev_costset0));
                    analysis_pending.extend(parents[class_id].iter().cloned());
                }
            }
        }

        for (cid, cost_set) in costs_all {
            result.choose(cid, cost_set.0.choice);
        }


        result
    }
}

/** A data structure to maintain a queue of unique elements.

Notably, insert/pop operations have O(1) expected amortized runtime complexity.

Thanks @Bastacyclop for the implementation!
*/
#[derive(Clone)]
#[cfg_attr(feature = "serde-1", derive(Serialize, Deserialize))]
pub(crate) struct UniqueQueue<T>
where
    T: Eq + std::hash::Hash + Clone,
{
    set: FxHashSet<T>, // hashbrown::
    queue: std::collections::VecDeque<T>,
}

impl<T> Default for UniqueQueue<T>
where
    T: Eq + std::hash::Hash + Clone,
{
    fn default() -> Self {
        UniqueQueue {
            set: Default::default(),
            queue: std::collections::VecDeque::new(),
        }
    }
}

impl<T> UniqueQueue<T>
where
    T: Eq + std::hash::Hash + Clone,
{
    pub fn insert(&mut self, t: T) {
        if self.set.insert(t.clone()) {
            self.queue.push_back(t);
        }
    }

    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for t in iter.into_iter() {
            self.insert(t);
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        let res = self.queue.pop_front();
        res.as_ref().map(|t| self.set.remove(t));
        res
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        let r = self.queue.is_empty();
        debug_assert_eq!(r, self.set.is_empty());
        r
    }
}