// Calculates the cost where shared nodes are just costed once,
// For example (+ (* x x ) (* x x )) has one mulitplication
// included in the cost.

use crate::*;
use rpds::queue;
use std::collections::HashSet;
use rustc_hash::{FxHashMap, FxHashSet};
use std::time::Instant;
use rayon::prelude::*;
use std::sync::{Arc, Mutex,RwLock};
use dashmap::DashMap;
use rand::seq::SliceRandom;
use flurry::HashMap as FlurryHashMap;


#[derive(Clone, Debug)]
struct CostSet {
    // It's slightly faster if this is an HashMap rather than an fxHashMap.
    costs: HashMap<ClassId, Cost>,
    total: Cost,
    choice: NodeId,
}

#[derive(Clone, Debug)]
struct CostSet1 {
    // It's slightly faster if this is an HashMap rather than an fxHashMap.
    costs: HashMap<ClassId, Cost>,
    total: Cost,
    choice: NodeId,
}

fn sort_by_total(vec: &mut Vec<(CostSet, CostSet)>) {
    vec.sort_by(|(cost_set1, _), (cost_set2, _)| cost_set1.total.cmp(&cost_set2.total));
}

pub struct FasterGreedyDagExtractor;

impl FasterGreedyDagExtractor {
    fn calculate_cost_set(
        egraph: &EGraph,
        node_id: NodeId,
        costs_all: &Arc<DashMap::<ClassId, (Arc<CostSet>, Arc<CostSet>)>>,
        best_cost: Cost,
    ) -> Arc<CostSet> {
        let node = &egraph[&node_id];
        let cid = egraph.nid_to_cid(&node_id);

        if node.children.is_empty() {
            return Arc::new(CostSet {
                costs: HashMap::from([(cid.clone(), node.cost)]),
                total: node.cost,
                choice: node_id.clone(),
            });
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
            return Arc::new(CostSet {
                costs: Default::default(),
                total: -INFINITY,
                choice: node_id.clone(),
            });
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

        return Arc::new(CostSet {
            costs: result,
            total: result_cost,
            choice: node_id.clone(),
        });
    }
}

fn combined_costset(costset1: &CostSet, cid2: &ClassId, costs_all: &Arc<DashMap::<ClassId, (Arc<CostSet>, Arc<CostSet>)>>, mode: bool) -> (Cost,Arc<CostSet>) {

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
        prev_costs2 = Arc::new(CostSet {
            costs: Default::default(),
            total: -INFINITY,
            choice: NodeId::new(),
        });
    }
    let mut combined_costs = prev_costs1.clone();
    for (key, value) in prev_costs2.costs.iter() {
        combined_costs.insert(key.clone(), *value);
    }

    // let cost1 = costset1.total;
    // let cost2 = costs_all.get(cid2).unwrap().0.total;
    return (combined_costs.values().sum(),prev_costs2);
}

fn process_item(egraph: &EGraph, node_id:&NodeId, costs_all: Arc<DashMap::<ClassId, (Arc<CostSet>, Arc<CostSet>)>>,snd_op_class: &FxHashSet<&ClassId>,fst_nmap: &FxHashMap<NodeId, NodeId>,snd_nmap: &FxHashMap<NodeId, NodeId>,fst_op_class: &FxHashSet<&ClassId>) -> FxHashMap<ClassId, Arc<CostSet>> {
    let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);
    let class_id = n2c(&node_id);
    let node = &egraph[node_id];
    let mut should_insert = FxHashMap::default();
    if node.children.iter().all(|c| costs_all.contains_key(n2c(c))) {
        let lookup = costs_all.get(class_id);
        // let mut prev_cost = -INFINITY;
        // let mut prev_costset0 = CostSet {
        //     costs: Default::default(),
        //     total: -INFINITY,
        //     choice: NodeId::new(),
        // };

        let prev_costset0;
        if let Some(l) = lookup {
            prev_costset0 = Arc::clone(&l.0);
        }
        else{
            let default_costset = Arc::new(CostSet {
                costs: Default::default(),
                total: -INFINITY,
                choice: NodeId::new(),
            });
            prev_costset0 = Arc::clone(&default_costset);
            costs_all.insert(class_id.clone(), (default_costset.clone(),default_costset.clone()));
        }
        

        let cost_set = FasterGreedyDagExtractor::calculate_cost_set(egraph, node_id.clone(), &costs_all, prev_costset0.total);

        if cost_set.total > prev_costset0.total {
            should_insert.insert(class_id.clone(), cost_set);
        }

    }
    should_insert
}



impl Extractor for FasterGreedyDagExtractor {
    fn extract(&self, egraph: &EGraph, _roots: &[ClassId]) -> ExtractionResult {
        let main_start = Instant::now();
        let mut parents: IndexMap<&ClassId, Vec<NodeId>> = IndexMap::<&ClassId, Vec<NodeId>>::with_capacity(egraph.classes().len());
        let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);
        let mut analysis_pending = UniqueQueue::default();

        for class in egraph.classes().values() {
            parents.insert(&class.id, Vec::new());
        }


        let mut fst_nmap: FxHashMap<NodeId, NodeId> = FxHashMap::default();
        let mut fst_op_class: FxHashSet<&ClassId> = FxHashSet::default();
        let mut snd_nmap: FxHashMap<NodeId, NodeId> = FxHashMap::default();
        let mut snd_op_class: FxHashSet<&ClassId> = FxHashSet::default();
        {
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

            for xor in &xor_op {
                for maj in &maj_op {
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
        }

        for class in egraph.classes().values() {
            for node in &class.nodes {
                for c in &egraph[node].children {
                    // compute parents of this enode
                    parents[n2c(c)].push(node.clone());
                }
            }
        }

        

        let arc_queue = Arc::new(Mutex::new(analysis_pending));
        let mut result = ExtractionResult::default();
        let costs_all = Arc::new(DashMap::<ClassId, (Arc<CostSet>, Arc<CostSet>)>::with_capacity_and_hasher(
            egraph.classes().len(),
            Default::default(),
        ));
        let default_costset = Arc::new(CostSet {
            costs: Default::default(),
            total: -INFINITY,
            choice: NodeId::new(),
        });
        // let arc_costs_all = Arc::new(RwLock::new(costs_all));

        // let costs_all = Arc::new(DashMap::<ClassId, (CostSet, CostSet)>::with_capacity_and_hasher(
        //     egraph.classes().len(),
        //     Default::default(),
        // ));
        

        // println!("fst_op_class: {:?}", fst_op_class);
        // println!("snd_op_class: {:?}", snd_op_class);

        for i in 0..4 {
            let mut classes: Vec<&Class> = egraph.classes().values().collect();
            classes.shuffle(&mut rand::thread_rng());
            for class in classes {
                for node in &class.nodes {
                    if i == 0{
                        if egraph[node].is_leaf() {
                            let mut queue = arc_queue.lock().unwrap();
                            queue.insert(node.clone());
                        }
                    }
                    else{
                        let mut queue = arc_queue.lock().unwrap();
                        queue.insert(node.clone());
                    }
                }
            }

            while {
                let queue = arc_queue.lock().unwrap();
                !queue.is_empty()
            } {
                let single_node_id = {
                    let mut queue = arc_queue.lock().unwrap();
                    queue.pop_32()
                };
    
                let costs_all_clone = Arc::clone(&costs_all); // 在外部克隆一次，避免在闭包中多次克隆
    
    

                let should_insert: Vec<(FxHashMap<ClassId, Arc<CostSet>>)> = single_node_id
                .into_par_iter() // 转换为并行迭代器
                .map(|node_id| {
                    // 克隆 Arc 以在多个线程中共享
                    let costs_all = Arc::clone(&costs_all_clone);
                    process_item(egraph, &node_id, costs_all, &snd_op_class, &fst_nmap, &snd_nmap, &fst_op_class)
                })
                .collect();


                let mut grouped: FxHashMap<ClassId,Arc<CostSet>> = FxHashMap::default();
                should_insert.into_iter().for_each(|map| {
                    for (key, value) in map {
                        // 判断是否已经存在该 ClassId 的条目
                        grouped.entry(key)
                            .and_modify(|existing| {
                                // 如果存在，比较现有的和新的值，保留 total 更大的 (CostSet, CostSet)
                                if value.total > existing.total {
                                    *existing = value.clone();
                                }
                            })
                            .or_insert(value); // 如果不存在该 ClassId，直接插入
                    }
                });

                for (cid, cost_set) in grouped {

                    // if non_arc_cost_set.total > NotNan::new(0.0).unwrap() {
                    //     println!("cid: {:?}, non_arc_cost_set: {:?}", cid, non_arc_cost_set.total);
                    // }
                    // let cost_set=Arc::new(non_arc_cost_set);
                    let class_id = &cid;
                    let node_id = cost_set.choice.clone();
                    let node = &egraph.nodes[&node_id];
                    let lookup = costs_all.get(class_id);
                    // let mut prev_cost = -INFINITY;
                    // let mut prev_costset0 = CostSet {
                    //     costs: Default::default(),
                    //     total: -INFINITY,
                    //     choice: NodeId::new(),
                    // };

                    // let mut prev_costset1 = CostSet {
                    //     costs: Default::default(),
                    //     total: -INFINITY,
                    //     choice: NodeId::new(),
                    // };

                    let prev_costset0: Arc<CostSet>;
                    let prev_costset1: Arc<CostSet>;


                    
                    if let Some(l) = lookup {
                        prev_costset0 = Arc::clone(&l.0); // Move instead of clone
                        prev_costset1 = Arc::clone(&l.1); // Move instead of clone
                    } else {
                        prev_costset0 = Arc::clone(&default_costset.clone());
                        prev_costset1 = Arc::clone(&default_costset.clone());
                    }

                    let mut inserted = FxHashMap::default();

                    if snd_op_class.contains(&class_id) {
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
                                    inserted.insert(class_id, (Arc::clone(&cost_set),prev_costset1));
                                    arc_queue.lock().unwrap().extend(parents[&class_id].iter().cloned());
                                    let mut costs2=cost_set.costs.clone();
                                    costs2.insert(cid2.clone(), egraph.nodes[&node_id].cost);
                                    costs2.remove(class_id);
                                    let CostSet2=Arc::new(CostSet {
                                        costs: costs2,
                                        total: cost_set.total,
                                        choice: snd_nmap.get(&node_id).unwrap().clone(),
                                    });
                                    inserted.insert(cid2, (CostSet2,costs_all.get(cid2).unwrap().1.clone()));
                                    arc_queue.lock().unwrap().extend(parents[cid2].iter().cloned());
                                    // print!("11 {:?}-{:?} {:?}-{:?}", class_id,cost_set.choice, cid2,CostSet2.choice);
    
                                    if cid4 != n2c(&node_id) {
                                        let costset4=costs_all.get(cid4).unwrap().1.clone();
                                        inserted.insert(cid4, (costset4.clone(),costset4));
                                        arc_queue.lock().unwrap().extend(parents[cid4].iter().cloned());
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
                                    inserted.insert(class_id, (cost_set.clone(),prev_costset1));
                                    arc_queue.lock().unwrap().extend(parents[class_id].iter().cloned());
                                    let mut costs2=cost_set.costs.clone();
                                    costs2.insert(cid2.clone(), egraph.nodes[&node_id].cost);
                                    costs2.remove(class_id);
                                    let CostSet2=Arc::new(CostSet {
                                        costs: costs2,
                                        total: cost_set.total,
                                        choice: snd_nmap.get(&node_id).unwrap().clone(),
                                    });
                                    inserted.insert(cid2, (CostSet2,costs_all.get(cid2).unwrap().1.clone()));
                                    arc_queue.lock().unwrap().extend(parents[cid2].iter().cloned());
                                    // print!("12 {:?}-{:?} {:?}-{:?}", class_id,cost_set.choice, cid2,CostSet2.choice);
                                    if cid2 != cid3 {
                                        let costset3=costs_all.get(cid3).unwrap().1.clone();
                                        inserted.insert(cid3, (costset3.clone(),costset3));
                                        arc_queue.lock().unwrap().extend(parents[cid3].iter().cloned());
                                        // print!(" {:?}-{:?}", cid3,costset3.choice);
                                    }
    
                                    if cid4 != n2c(&node_id) {
                                        let costset4=costs_all.get(cid4).unwrap().1.clone();
                                        inserted.insert(cid4, (costset4.clone(),costset4));
                                        arc_queue.lock().unwrap().extend(parents[cid4].iter().cloned());
                                        // print!(" {:?}-{:?}", cid4,costset4.choice);
                                    }
    
                                    // println!();
                                }

                            }
                        }
                        else if node.op != "fst" {
                            // Case 3: Node is not snd and the previous node 0 is not snd
                            let mut flag = true;
                            if prev_costset0.choice.as_ref() == "None" || egraph.nodes[&prev_costset0.choice].op != "snd"{
                                if cost_set.total > prev_costset0.total {
                                    inserted.insert(class_id, (cost_set.clone(),cost_set.clone()));
                                    arc_queue.lock().unwrap().extend(parents[class_id].iter().cloned());
                                    flag = false;
                                    // println!("13 {:?}-{:?}", class_id,cost_set.choice);
                                }
                            }
                            // Case 4: Node is not snd and the previous node is snd
                            else{
                                let cid2 = n2c(snd_nmap.get(&prev_costset0.choice).unwrap());
                                let (total1,prev_costset2) = combined_costset(&cost_set, cid2, &costs_all, true);
                                if total1 > prev_costset0.total {
                                    inserted.insert(class_id, (cost_set.clone(),cost_set.clone()));
                                    arc_queue.lock().unwrap().extend(parents[class_id].iter().cloned());
                                    // update the costset of xor
                                    let costset2=costs_all.get(cid2).unwrap().1.clone();
                                    inserted.insert(cid2,(costset2.clone(),costset2));
                                    arc_queue.lock().unwrap().extend(parents[cid2].iter().cloned());
                                    flag = false;
                                    // println!("14 {:?}-{:?} {:?}-{:?}", class_id,cost_set.choice, cid2,costset2.choice);
                                }
                            }
                            // If the node total is less than the previous node 0 total, then we need to check the previous node 1, if it is not snd, then we can update the costset.
                            if flag {
                                if prev_costset1.choice.as_ref() == "None" || egraph.nodes[&prev_costset1.choice].op != "snd"{
                                    if cost_set.total > prev_costset1.total {
                                        inserted.insert(class_id, (prev_costset0,cost_set));
                                        arc_queue.lock().unwrap().extend(parents[class_id].iter().cloned());
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
                                if snd_nmap.contains_key(&costs_all.get(&cid2).unwrap().0.choice){
                                    cid4 = n2c(snd_nmap.get(&costs_all.get(&cid2).unwrap().0.choice).unwrap());
                                }
                                let total1 = cost_set.total;
                                let (total2,_) = combined_costset(&prev_costset0, cid2, &costs_all, false);
                                if total1 > total2 {
                                    inserted.insert(class_id, (cost_set.clone(),prev_costset1));
                                    arc_queue.lock().unwrap().extend(parents[class_id].iter().cloned());
                                    let mut costs2=cost_set.costs.clone();
                                    costs2.insert(cid2.clone(), egraph.nodes[&node_id].cost);
                                    costs2.remove(class_id);
                                    let CostSet2=Arc::new(CostSet {
                                        costs: costs2,
                                        total: cost_set.total,
                                        choice: fst_nmap.get(&node_id).unwrap().clone(),
                                    });
                                    inserted.insert(cid2, (CostSet2,costs_all.get(cid2).unwrap().1.clone()));
                                    arc_queue.lock().unwrap().extend(parents[cid2].iter().cloned());
                                    // print!("21 {:?}-{:?} {:?}-{:?}", class_id,cost_set.choice, cid2,CostSet2.choice);
                                    
    
                                    if cid4 != n2c(&node_id) {
                                        let costset4=costs_all.get(cid4).unwrap().1.clone();
                                        inserted.insert(cid4, (costset4.clone(),costset4));
                                        arc_queue.lock().unwrap().extend(parents[cid4].iter().cloned());
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
                                    if snd_nmap.contains_key(&costs_all.get(&cid2).unwrap().0.choice){
                                        cid4 = n2c(snd_nmap.get(&costs_all.get(&cid2).unwrap().0.choice).unwrap());
                                    }
                                    inserted.insert(class_id, (cost_set.clone(),prev_costset1));
                                    arc_queue.lock().unwrap().extend(parents[class_id].iter().cloned());
                                    let mut costs2=cost_set.costs.clone();
                                    costs2.insert(cid2.clone(), egraph.nodes[&node_id].cost);
                                    costs2.remove(class_id);
                                    let CostSet2=Arc::new(CostSet {
                                        costs: costs2,
                                        total: cost_set.total,
                                        choice: fst_nmap.get(&node_id).unwrap().clone(),
                                    });
                                    inserted.insert(cid2, (CostSet2,costs_all.get(cid2).unwrap().1.clone()));
                                    arc_queue.lock().unwrap().extend(parents[cid2].iter().cloned());
                                    // print!("22 {:?}-{:?} {:?}-{:?}", class_id,cost_set.choice, cid2,CostSet2.choice);
                                    if cid2 != cid3 {
                                        let costset3=costs_all.get(cid3).unwrap().1.clone();
                                        inserted.insert(cid3, (costset3.clone(),costset3));
                                        arc_queue.lock().unwrap().extend(parents[cid3].iter().cloned());
                                        // print!(" {:?}-{:?}", cid3,costset3.choice);
                                    }
    
                                    if cid4 != n2c(&node_id) {
                                        let costset4=costs_all.get(cid4).unwrap().1.clone();
                                        inserted.insert(cid4, (costset4.clone(),costset4));
                                        arc_queue.lock().unwrap().extend(parents[cid4].iter().cloned());
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
                                    inserted.insert(class_id, (cost_set.clone(),cost_set.clone()));
                                    arc_queue.lock().unwrap().extend(parents[class_id].iter().cloned());
                                    flag = false;
                                    // println!("23 {:?}-{:?}", class_id,cost_set.choice);
                                }
                            }
                            // Case 4: Node is not fst and the previous node is fst
                            else{
                                let cid2 = n2c(fst_nmap.get(&prev_costset0.choice).unwrap());
                                let (total1,prev_costset2) = combined_costset(&cost_set, cid2, &costs_all, true);
                                if total1 > prev_costset0.total {
                                    inserted.insert(class_id, (cost_set.clone(),cost_set.clone()));
                                    arc_queue.lock().unwrap().extend(parents[class_id].iter().cloned());
                                    // update the costset of xor
                                    let costset2=costs_all.get(cid2).unwrap().1.clone();
                                    inserted.insert(cid2,(costset2.clone(),costset2));
                                    arc_queue.lock().unwrap().extend(parents[cid2].iter().cloned());
                                    flag = false;
                                    // println!("24 {:?}-{:?} {:?}-{:?}", class_id,cost_set.choice, cid2,costset2.choice);
                                }
                            }
                            // If the node total is less than the previous node 0 total, then we need to check the previous node 1, if it is not fst, then we can update the costset.
                            if flag {
                                if prev_costset1.choice.as_ref() == "None" || egraph.nodes[&prev_costset1.choice].op != "fst"{
                                    if cost_set.total > prev_costset1.total {
                                        inserted.insert(class_id, (prev_costset0,cost_set));
                                        arc_queue.lock().unwrap().extend(parents[class_id].iter().cloned());
                                        // println!("25 {:?}-{:?}", class_id,prev_costset0.choice);
                                    }
                                }
                            }
                        }
                    }
                    else if cost_set.total > prev_costset0.total {
                        inserted.insert(class_id, (cost_set,default_costset.clone()));
                        arc_queue.lock().unwrap().extend(parents[class_id].iter().cloned());
                    }

                    
                    for (cid, cost_set) in inserted {
                        costs_all.insert(cid.clone(), cost_set.clone());
                        // let temp =(*cost_set.0).clone();
                        // let default_costset = Arc::new(CostSet1 {
                        //     costs: temp.costs.clone(),
                        //     total: temp.total.clone(),
                        //     choice: temp.choice.clone(),
                        // });
                        // let cost_set_clone = (default_costset,cost_set.0);
                        // costs_all1.insert(cid.clone(), cost_set_clone);
                        // costs_all1.insert(cid.clone(), cost_set);
                        // 24 735276 729924 1037736
                        // 36 33517484 16646896   
                        // 1
                        // total 17165660 total+choice 17355860 total+choice+costs 23302972
                        // 36 i64 total+choice+costs 24091844
                        // key 20038568 19422796 20041448
                        // value 18704132 19751720 19077700
                        // empty 16683812 16567296 16147344
                        // total 23268256
                        // 0
                        // total 26781668 i64 24544688 i16
                    }
                }
            }

        }

        
        for entry in costs_all.iter() {
            let cid = entry.key();
            let cost_set = entry.value();
            result.choose(cid.clone(), cost_set.0.choice.clone());
        }
        // println!("Time elapsed in extraction loop is: {:?}", main_start.elapsed());

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

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn pop(&mut self) -> Option<T> {
        let res = self.queue.pop_front();
        res.as_ref().map(|t| self.set.remove(t));
        res
    }

    pub fn pop_32(&mut self) -> Vec<T> {
        let mut popped_items = Vec::with_capacity(512);
        
        for _ in 0..512 {
            if let Some(item) = self.queue.pop_front() {
                self.set.remove(&item);
                popped_items.push(item);
            } else {
                break; // 队列已空，退出循环
            }
        }
        
        popped_items
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        let r = self.queue.is_empty();
        debug_assert_eq!(r, self.set.is_empty());
        r
    }
}
