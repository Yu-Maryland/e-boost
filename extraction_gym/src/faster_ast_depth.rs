use rustc_hash::{FxHashMap, FxHashSet};
use crate::*;
pub const U32INFINITY: u32 = std::u32::MAX-1;
/// 一个基于 AST 大小（节点总数）的快速提取器
pub struct FasterAstSizeExtractor;

impl Extractor for FasterAstSizeExtractor {
    fn extract(&self, egraph: &EGraph, _roots: &[ClassId]) -> ExtractionResult {
        // 构造每个等价类对应的父节点列表
        let mut parents = IndexMap::<ClassId, Vec<NodeId>>::with_capacity(egraph.classes().len());
        let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);
        let mut analysis_pending = UniqueQueue::default();

        for class in egraph.classes().values() {
            parents.insert(class.id.clone(), Vec::new());
        }

        // 遍历所有节点，建立子节点到父节点的映射，并将叶节点加入待分析队列
        for class in egraph.classes().values() {
            for node in &class.nodes {
                for child in &egraph[node].children {
                    parents[child].push(node.clone());
                }
                if egraph[node].is_leaf() {
                    analysis_pending.insert(node.clone());
                }
            }
        }

        let mut result = ExtractionResult::default();
        let mut costs = FxHashMap::<ClassId, u32>::with_capacity_and_hasher(
            egraph.classes().len(),
            Default::default(),
        );  

        // 通过工作队列迭代更新成本
        while let Some(node_id) = analysis_pending.pop() {
            let class_id = n2c(&node_id);
            let node = &egraph[&node_id];
            let prev_cost = costs.get(class_id).unwrap_or(&U32INFINITY);
            // 计算当前节点的代价：1 + 所有子节点代价之和
            let cost = 1 + node.children.iter().fold(0, |max, child_id| {
                max.max(*costs.get(&child_id).unwrap_or(&U32INFINITY))
            });
            // println!("node_id:{:?},cost:{:?}",node_id.0,cost);
            // for nodee in &node.children {
            //     println!("{:?},{:?}",nodee,costs.get(&nodee).unwrap_or(&U32INFINITY));
            // }
            if cost < *prev_cost {
                result.choose(class_id.clone(), node_id.clone());
                costs.insert(class_id.clone(), cost);
                // 将父节点加入待处理队列，因为其成本可能会因当前节点成本降低而下降
                analysis_pending.extend(parents[class_id].iter().cloned());
            }
        }

        result
    }
}

/// 保证队列中元素唯一的队列结构，实现了 O(1) 期望均摊插入/弹出复杂度。
#[derive(Clone)]
#[cfg_attr(feature = "serde-1", derive(Serialize, Deserialize))]
pub(crate) struct UniqueQueue<T>
where
    T: Eq + std::hash::Hash + Clone,
{
    set: FxHashSet<T>,
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
        for t in iter {
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
