use rustc_hash::{FxHashMap, FxHashSet};
use dashmap::DashMap;
use std::sync::Arc;
use rayon::prelude::*;

use crate::*;

/// A faster bottom up extractor inspired by the faster-greedy-dag extractor.
/// It should return an extraction result with the same cost as the bottom-up extractor.
///
/// Bottom-up extraction works by iteratively computing the current best cost of each
/// node in the e-graph based on the current best costs of its children.
/// Extraction terminates when our estimates of the best cost for each node
/// reach a fixed point.
/// The baseline bottom-up implementation visits every node during each iteration
/// of the fixed point.
/// This algorithm instead only visits the nodes whose current cost estimate may change:
/// it does this by tracking parent-child relationships and storing relevant nodes
/// in a work list (UniqueQueue).
pub struct FasterBottomUpExtractor;

impl Extractor for FasterBottomUpExtractor {
    fn extract(&self, egraph: &EGraph, _roots: &[ClassId]) -> ExtractionResult {
        let mut parents = IndexMap::<ClassId, Vec<NodeId>>::with_capacity(egraph.classes().len());
        let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);
        let mut analysis_pending = UniqueQueue::default();

        for class in egraph.classes().values() {
            parents.insert(class.id.clone(), Vec::new());
        }

        for class in egraph.classes().values() {
            for node in &class.nodes {
                for c in &egraph[node].children {
                    // compute parents of this enode
                    parents[c].push(node.clone());
                }

                // start the analysis from leaves
                if egraph[node].is_leaf() {
                    analysis_pending.insert(node.clone());
                }
            }
        }

        let mut result = ExtractionResult::default();
        // let mut costs = FxHashMap::<ClassId, Cost>::with_capacity_and_hasher(
        //     egraph.classes().len(),
        //     Default::default(),
        // );

        let costs_all: Arc<DashMap<ClassId, (NodeId,Cost)>> = Arc::new(DashMap::with_capacity_and_hasher(
            egraph.classes().len(), Default::default()));


        while !analysis_pending.is_empty() {
            let vec_node_id = analysis_pending.pop_32();
            let costs_all_clone: Arc<DashMap<ClassId, (NodeId,Cost)>> = Arc::clone(&costs_all);
            let should_insert: Vec<_> = vec_node_id.into_par_iter().map(|node_id| {
                let costs_all = Arc::clone(&costs_all_clone);
                let class_id = n2c(&node_id);
                let node = &egraph[&node_id];
                let prev_cost = costs_all
                    .get(&class_id)
                    .map(|r| r.1)
                    .unwrap_or(INFINITY);
                let children_sum = node
                    .children
                    .iter()
                    .map(|child_id| {
                        costs_all_clone
                            .get(&child_id)
                            .map(|r| r.1)
                            .unwrap_or(INFINITY)
                    })
                    .sum::<Cost>();
                let cost = node.cost + children_sum;
                if cost < prev_cost {
                    (cost,node_id)
                } else {
                    (INFINITY,node_id)
                }
            }).collect();
            let mut grouped: FxHashMap<ClassId, (NodeId,Cost)> = FxHashMap::default();
            should_insert.into_iter().for_each(|map| {
                let key = n2c(&map.1);
                let value = map.0;
                if value != INFINITY {
                    grouped.entry(*key)
                        .and_modify(|existing| {
                            if value < existing.1 {
                                *existing = (map.1,value);
                            }
                        })
                        .or_insert((map.1,value));
                }
            });
            for (cid, cost_set) in grouped {
                costs_all.insert(cid, cost_set);
                analysis_pending.extend(parents[&cid].iter().cloned());
            }
        }

        for entry in costs_all.iter() {
            let cid = entry.key();
            let cost_set = entry.value();
            result.choose(cid.clone(), cost_set.0);
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

    pub fn pop_32(&mut self) -> Vec<T> {
        let k = 4096*2;
        let mut popped_items = Vec::with_capacity(k);
        
        for _ in 0..k {
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
