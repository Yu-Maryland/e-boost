/*
Produces a dag-cost optimal extraction of an Egraph.

This can take >10 hours to run on some egraphs, so there's the option to provide a timeout.

To operate:
1) It simplifies the egraph by removing nodes that can't be selected in the optimal
solution, as well as collapsing other classes down.
2) It then sends the problem to the COIN-OR CBC solver to find an extraction (or timeout).
It allows the solver to generate solutions that contain cycles
3) The solution from the solver is checked, and if the extraction contains a cycle, extra
constraints are added to block the cycle and the solver is called again.

In SAT solving, it's common to call a solver incrementally. Each time you call the SAT
solver with more clauses to the SAT solver (constraining the solution further), and
allowing the SAT solver to reuse its previous work.

So there are two uses of "incremental", one is gradually sending more of the problem to the solver,
and the other is the solver being able to re-use the previous work when it receives additional parts
of the problem. In the case here, we're just referring to sending extra pieces of the problem to
the solver. COIN-OR CBC doesn't provide an interface that allows us to call it and reuse what it
has discovered previously.

In the case of COIN-OR CBC, we're sending extra constraints each time we're solving, these
extra constraints are prohibiting cycles that were found in the solutions that COIN-OR CBC
previously produced.

Obviously, we could add constraints to block all the cycles the first time we call COIN-OR CBC,
so we'd only need to call the solver once. However, for the problems in our test-set, lots of these
constraints don't change the answer, they're removing cycles from high-cost extractions.  These
extra constraints do slow down solving though - and for our test-set it gives a faster runtime when
we incrementally add constraints that break cycles when they occur in the lowest cost extraction.

We've experimented with two ways to break cycles.

One approach is by enforcing a topological sort on nodes. Each node has a level, and each edge
can only connect from a lower level to a higher level node.

Another approach, is by explicity banning cycles. Say in an extraction that the solver generates
we find a cycle A->B->A. Say there are two edges, edgeAB, and edgeBA, which connect A->B, then B->A.
Then any solution that contains both edgeAB, and edgeBA will contain a cycle.  So we add a constraint
that at most one of these two edges can be active. If we check through the whole extraction for cycles,
and ban each cycle that we find, then try solving again, we'll get a new solution which, if it contains
cycles, will not contain any of the cycles we've previously seen. We repeat this until timeout, or until
we get an optimal solution without cycles.


*/

use crate::*;
use indexmap::IndexSet;
use std::fmt;
use std::time::SystemTime;
use std::fs;
use std::fs::File;
use std::io::Write;
use serde_json::json;
use rand::Rng;

#[derive(Debug)]
pub struct Config {
    pub pull_up_costs: bool,
    pub remove_self_loops: bool,
    pub remove_high_cost_nodes: bool,
    pub remove_more_expensive_subsumed_nodes: bool,
    pub remove_unreachable_classes: bool,
    pub pull_up_single_parent: bool,
    pub take_intersection_of_children_in_class: bool,
    pub move_min_cost_of_members_to_class: bool,
    pub find_extra_roots: bool,
    pub remove_empty_classes: bool,
    pub return_improved_on_timeout: bool,
    pub remove_single_zero_cost: bool,
}

impl Config {
    pub const fn default() -> Self {
        Self {
            pull_up_costs: true,
            remove_self_loops: true,
            remove_high_cost_nodes: true,
            remove_more_expensive_subsumed_nodes: true,
            remove_unreachable_classes: true,
            pull_up_single_parent: true,
            take_intersection_of_children_in_class: true,
            move_min_cost_of_members_to_class: false,
            find_extra_roots: true,
            remove_empty_classes: true,
            return_improved_on_timeout: true,
            remove_single_zero_cost: true,
        }
    }
}

struct NodeILP {
    variable: String,
    cost: Cost,
    member: NodeId,
    children_classes: IndexSet<ClassId>,
}

impl fmt::Debug for NodeILP {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "nodeILP[{}] {{ variable: {:?}, cost: {:?}, children: {:?} }}",
            self.member, self.variable, self.cost, self.children_classes
        )
    }
}

struct ClassILP {
    variable: String,
    members: Vec<NodeId>,
    node_variables: Vec<String>,
    costs: Vec<Cost>,
    // Initially this contains the children of each member (respectively), but
    // gets edited during the run, so mightn't match later on.
    childrens_classes: Vec<IndexSet<ClassId>>,
}

impl fmt::Debug for ClassILP {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "classILP[{}] {{ node: {:?}, children: {:?},  cost: {:?} }}",
            self.members(),
            self.members,
            self.childrens_classes,
            self.costs
        )
    }
}

impl ClassILP {
    fn remove(&mut self, idx: usize) {
        self.node_variables.remove(idx);
        self.costs.remove(idx);
        self.members.remove(idx);
        self.childrens_classes.remove(idx);
    }

    fn remove_node(&mut self, node_id: &NodeId) {
        if let Some(idx) = self.members.iter().position(|n| n == node_id) {
            self.remove(idx);
        }
    }

    fn members(&self) -> usize {
        self.node_variables.len()
    }

    fn check(&self) {
        assert_eq!(self.node_variables.len(), self.costs.len());
        assert_eq!(self.node_variables.len(), self.members.len());
        assert_eq!(self.node_variables.len(), self.childrens_classes.len());
    }

    fn as_nodes(&self) -> Vec<NodeILP> {
        self.node_variables
            .iter()
            .zip(&self.costs)
            .zip(&self.members)
            .zip(&self.childrens_classes)
            .map(|(((variable, &cost_), member), children_classes)| NodeILP {
                variable: variable.clone(),
                cost: cost_,
                member: member.clone(),
                children_classes: children_classes.clone(),
            })
            .collect()
    }

    fn get_children_of_node(&self, node_id: &NodeId) -> &IndexSet<ClassId> {
        let idx = self.members.iter().position(|n| n == node_id).unwrap();
        &self.childrens_classes[idx]
    }

    fn get_variable_for_node(&self, node_id: &NodeId) -> Option<String> {
        if let Some(idx) = self.members.iter().position(|n| n == node_id) {
            return Some(self.node_variables[idx].clone());
        }
        None
    }
}

pub struct FasterCbcExtractorWithTimeout<const TIMEOUT_IN_SECONDS: u32>;

// Some problems take >36,000 seconds to optimise.
impl<const TIMEOUT_IN_SECONDS: u32> Extractor
    for FasterCbcExtractorWithTimeout<TIMEOUT_IN_SECONDS>
{
    fn extract(&self, egraph: &EGraph, roots: &[ClassId]) -> ExtractionResult {
        return extract(egraph, roots, &Config::default(), TIMEOUT_IN_SECONDS);
    }
}

/// 对字符串进行简单处理，转换成只含字母数字和下划线的变量名
fn sanitize<T: ToString>(s: &T) -> String {
    s.to_string()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}


pub struct MyExtractor;

impl Extractor for MyExtractor {
    fn extract(&self, egraph: &EGraph, roots: &[ClassId]) -> ExtractionResult {
        return extract(egraph, roots, &Config::default(), std::u32::MAX);
    }
}

fn extract(
    egraph: &EGraph,
    roots_slice: &[ClassId],
    config: &Config,
    timeout: u32,
) -> ExtractionResult {
    // todo from now on we don't use roots_slice - be good to prevent using it any more.
    let mut roots = roots_slice.to_vec();
    roots.sort();
    roots.dedup();

    let simp_start_time = std::time::Instant::now();


    let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);

    let mut vars: IndexMap<ClassId, ClassILP> = egraph
        .classes()
        .iter()
        .map(|(key, class)| {
            let cvars = ClassILP {
                variable: format!("A_{}", key.to_string()),
                node_variables: class.nodes.iter().map(|nid| format!("N_{}",sanitize(&nid))).collect(),
                costs: class.nodes.iter().map(|n| egraph[n].cost).collect(),
                members: class.nodes.clone(),
                childrens_classes: class
                    .nodes
                    .iter()
                    .map(|n| {
                        egraph[n]
                            .children
                            .iter()
                            .map(|c| c.clone())
                            .collect::<IndexSet<ClassId>>()
                    })
                    .collect(),
            };

            (class.id.clone(), cvars)
        })
        .collect();

    let initial_result = super::faster_greedy_dag::FasterGreedyDagExtractor.extract(egraph, &roots);
    let initial_result_cost = initial_result.dag_cost(egraph, &roots);
    save_inital_solution("initial_solution.json", &initial_result);

    // For classes where we know the choice already, we set the nodes early.
    let mut result = ExtractionResult::default();

    
    // This could be much more efficient, but it only takes less than 5 seconds for all our benchmarks.
    // The ILP solver takes the time.
    for _i in 1..3 {
        remove_with_loops(&mut vars, &roots, config);
        remove_high_cost(&mut vars, initial_result_cost, &roots, config);
        remove_more_expensive_subsumed_nodes(&mut vars, config);
        remove_unreachable_classes(&mut vars, &roots, config);
        pull_up_with_single_parent(&mut vars, &roots, config);
        pull_up_costs(&mut vars, &roots, config);
        remove_single_zero_cost(&mut vars, &mut result, &roots, config);
        find_extra_roots(&vars, &mut roots, config);
        remove_empty_classes(&mut vars, config);
    }

    let mut lp = String::new();

    lp.push_str("Minimize\n obj: ");
    let mut obj_terms = Vec::new();

    // 遍历每个 e‑class（这里的 key 为 ClassId）
    for (classid, c_var) in &vars {
        let mut min_cost:f64 = 0.0;

        // 若配置启用了将最小成本上提到类变量，则计算该类候选节点的最小成本
        if config.move_min_cost_of_members_to_class {
            min_cost = c_var
                .costs
                .iter()
                .map(|&c| c.into_inner())
                .fold(f64::INFINITY, f64::min);
            if min_cost == f64::INFINITY {
                min_cost = 0.0;
            }
        }
        // 如果最小成本不为 0，则为该类激活变量（记为 A_<classid>）添加一项
        if (min_cost - 0.0).abs() > 1e-9 {
            obj_terms.push(format!("{} A_{}", min_cost, sanitize(&classid)));
        }

        // 对该类内每个候选节点（变量名称记为 N_<classid>_<i>）添加相应项：
        // 如果 (node_cost - min_cost) 不为 0，则添加该项
        for (i, &node_cost) in c_var.costs.iter().enumerate() {
            let diff = node_cost.into_inner() - min_cost;
            if diff.abs() > 1e-9 {
                obj_terms.push(format!("{} N_{}_{}", diff, sanitize(&classid), i));
            }
        }
    }

    // 将所有项用 " + " 连接
    lp.push_str(&obj_terms.join(" + "));
    lp.push_str("\n\n");

    lp.push_str("Subject To\n");

    let m_const = vars.len() + 1;

    for (classid, class) in &vars {
        // 若该类没有候选节点
        if class.members() == 0 {
            if roots.contains(&classid) {
                // 若是根却无可选节点，则模型不可行（直接添加一个必然矛盾的约束）
                lp.push_str(&format!(
                    "\\* Infeasible: Root {} has no possible children *\\\n",
                    classid
                ));
                lp.push_str(&format!("INFEASIBLE_{}: 1 = 0\n", sanitize(&classid)));
                continue;
            } else {
                // 非根的空类，将其激活变量上界设为 0
                lp.push_str(&format!(
                    "BND_{}: A_{} == 0\n",
                    sanitize(&classid),
                    sanitize(&classid)
                ));
                continue;
            }
        }

        // 如果该类只有一个候选节点、且该候选节点无子节点且成本为0，则直接将该节点作为解输出
        if class.members() == 1 && class.childrens_classes[0].is_empty() && class.costs[0] == 0.0 {
            result.choose(classid.clone(), class.members[0].clone());
            continue;
        }

        // 约束 1：类激活变量等于其所有候选节点变量之和
        // 写成： N_{class}_0 + N_{class}_1 + ... - A_{class} = 0
        let mut node_terms = Vec::new();
        for node_active in &class.node_variables {
            node_terms.push(node_active.clone());
        }
        lp.push_str(&format!(
            "C_ACT_{}: {} - A_{} = 0\n",
            sanitize(&classid),
            node_terms.join(" + "),
            sanitize(&classid)
        ));
        
        // 定义一个辅助函数：给定一组 ClassId，返回其“激活变量”集合（即 A_<childid>）
        fn childrens_classes_vars(cc: &IndexSet<ClassId>) -> IndexSet<String> {
            let mut set = IndexSet::new();
            for cid in cc {
                set.insert(sanitize(cid));
            }
            set
        }

        // 计算所有候选节点的子集的交集（交集中的每个元素都是一个激活变量名称）
        let mut intersection: IndexSet<String> = IndexSet::new();
        if config.take_intersection_of_children_in_class {
            if let Some(first_cc) = class.childrens_classes.get(0) {
                intersection = childrens_classes_vars(first_cc);
            }
        }

        for cc in class.childrens_classes.iter().skip(1) {
            let current = childrens_classes_vars(cc);
            intersection = intersection.intersection(&current).cloned().collect();
        }

        // 约束 2：类被激活 ⇒ 交集中所有子类也被激活，即 A_{class} - A_{child} <= 0
        for child_active in &intersection {
            lp.push_str(&format!(
                "C_INT_{}_{}: A_{} - A_{} <= 0\n",
                sanitize(&classid),
                sanitize(child_active),
                sanitize(&classid),
                child_active
            ));
        }

        // 约束 3：对于每个候选节点（与其对应的子集），若子集中的子类不在交集中，则要求：节点激活 ⇒ 对应子类激活
        // 即写成： N_{class}_{i} - A_{child} <= 0
        for (i, cc) in class.childrens_classes.iter().enumerate() {
            let node_var = format!("N_{}_{}", sanitize(&classid), i);
            let child_vars = childrens_classes_vars(cc);
            for child_active in child_vars {
                if !intersection.contains(&child_active) {
                    lp.push_str(&format!(
                        "C_CHILD_{}_{}_{}: {} - A_{} <= 0\n",
                        sanitize(&classid),
                        i,
                        sanitize(&child_active),
                        node_var,
                        child_active
                    ));
                }
            }
        }

        // 约束4 对于每个候选节点，添加： N + OPP = 1
        for (i, _node_id) in class.members.iter().enumerate() {
            let node_var = format!("N_{}_{}", sanitize(&classid), i);
            let opp_var  = format!("OPP_{}_{}", sanitize(&classid), i);
            lp.push_str(&format!(
                "OPP_{}_{}: {} + {} = 1\n",
                sanitize(&classid),
                i,
                node_var,
                opp_var
            ));
        }

        // 约束5 如果候选节点出现自环（其子集中包含本 e‑class），则直接使该节点变量取 0
        for (i, node_id) in class.members.iter().enumerate() {
            // 假设 class.childrens_classes[i] 为该候选节点的子类集合
            let children_classes = &class.childrens_classes[i];
            if children_classes.contains(classid) {
                let node_var = format!("N_{}_{}", sanitize(&classid), i);
                lp.push_str(&format!(
                    "SELF_LOOP_{}_{}: {} = 0\n",
                    sanitize(&classid),
                    i,
                    node_var
                ));
            }
        }

        // 约束6 对于每个候选节点和其每个非自环的子类，添加层级约束
        // M 取 (#eclass 数 + 1)

        let level_var = format!("L_{}", sanitize(&classid));
        for (i, _node_id) in class.members.iter().enumerate() {
            let opp_var = format!("OPP_{}_{}", sanitize(&classid), i);
            // 对于该候选节点中所有子节点所属的 e‑class（排除自身）
            let child_set = &class.childrens_classes[i];
            for child_cid in child_set {
                if child_cid == classid {
                    continue; // 跳过同一 e‑class
                }
                let child_level = format!("L_{}", sanitize(child_cid));
                lp.push_str(&format!(
                    "LEVEL_{}_{}_{}: {} - {} + {}*{} >= 1\n",
                    sanitize(&classid),
                    i,
                    sanitize(child_cid),
                    child_level,
                    level_var,
                    m_const,
                    opp_var
                ));
            }
        }
    }

    lp.push_str("\nBounds\n");
    // 对于每个根 e‑class，要求激活变量 A_<classid> 的下界为 1，
    // 这里直接写成： 1 <= A_<classid> <= 1
    // （因为 A_<classid> 是二进制变量，所以可以写成等于 1）
    for root in roots {
        lp.push_str(&format!("A_{} == 1\n", sanitize(&root)));
    }

    log::info!(
        "Time spent before solving: {}ms",
        simp_start_time.elapsed().as_millis()
    );

    let mut file = File::create("total1.lp")
        .expect("无法创建 ILP 文件");
    file.write_all(lp.as_bytes())
        .expect("写入 ILP 文件失败");
    println!("ILP written to file:{}","total1.lp");
    let start_time = SystemTime::now();



    panic!("stop here");


    initial_result
}

/*
Using this caused wrong results from the solver. I don't have a good idea why.
*/
// fn set_initial_solution(
//     vars: &IndexMap<ClassId, ClassILP>,
//     model: &mut Model,
//     initial_result: &ExtractionResult,
// ) {
//     for (class, class_vars) in vars {
//         for col in class_vars.variables.clone() {
//             model.set_col_initial_solution(col, 0.0);
//         }

//         if let Some(node_id) = initial_result.choices.get(class) {
//             model.set_col_initial_solution(class_vars.active, 1.0);
//             if let Some(var) = vars[class].get_variable_for_node(node_id) {
//                 model.set_col_initial_solution(var, 1.0);
//             }
//         } else {
//             model.set_col_initial_solution(class_vars.active, 0.0);
//         }
//     }
// }


fn save_inital_solution(
    file_path: &str,
    initial_result: &ExtractionResult,
) {
    let mut rng = rand::thread_rng();
    let mut file = std::fs::File::create(file_path).unwrap();

    // 假设 initial_result.choices 是一个 IndexMap<ClassId, NodeId>
    let modified: IndexMap<String, serde_json::Value> = initial_result.choices
    .iter()
    .map(|(k, v)| {
        let random_num: u8 = rng.gen_range(0..=1);
        (k.to_string(), json!([v, 0]))
    })
    .collect();
    let json = serde_json::to_string_pretty(&modified).unwrap();
    fs::write(file_path, json).expect("Unable to write file");
}

/* If a class has one node, and that node is zero cost, and it has no children, then we
can fill the answer into the extraction result without doing any more work. If it
has children, we need to setup the dependencies.

Intuitively, whenever we find a class that has a single node that is zero cost, our work
is done, we can't do any better for that class, so we can select it. Additionally, we
don't care if any other node depends on this class, because this class is zero cost,
we can ignore all references to it.

This is really like deleting empty classes, except there we delete the parent classes,
and here we delete just children of nodes in the parent classes.

*/
fn remove_single_zero_cost(
    vars: &mut IndexMap<ClassId, ClassILP>,
    extraction_result: &mut ExtractionResult,
    roots: &[ClassId],
    config: &Config,
) {
    if config.remove_single_zero_cost {
        let mut zero: FxHashSet<ClassId> = Default::default();
        for (class_id, details) in &*vars {
            if details.childrens_classes.len() == 1
                && details.childrens_classes[0].is_empty()
                && details.costs[0] == 0.0
                && !roots.contains(&class_id.clone())
            {
                zero.insert(class_id.clone());
            }
        }

        if zero.is_empty() {
            return;
        }

        let mut removed = 0;
        let mut extras = 0;
        let fresh = IndexSet::<ClassId>::new();
        let child_to_parents = child_to_parents(&vars);

        // Remove all references to those in zero.
        for e in &zero {
            let parents = child_to_parents.get(e).unwrap_or(&fresh);
            for parent in parents {
                for i in (0..vars[parent].childrens_classes.len()).rev() {
                    if vars[parent].childrens_classes[i].contains(e) {
                        vars[parent].childrens_classes[i].remove(e);
                        removed += 1;
                    }
                }

                // Like with empty classes, we might have discovered a new candidate class.
                // It's rare in our benchmarks so I haven't implemented it yet.
                if vars[parent].childrens_classes.len() == 1
                    && vars[parent].childrens_classes[0].is_empty()
                    && vars[parent].costs[0] == 0.0
                    && !roots.contains(&e.clone())
                {
                    extras += 1;
                    // this should be called in a loop like we delete empty classes.
                }
            }
        }
        // Add into the extraction result
        for e in &zero {
            extraction_result.choose(e.clone(), vars[e].members[0].clone());
        }

        // Remove the classes themselves.
        vars.retain(|class_id, _| !zero.contains(class_id));

        log::info!(
            "Zero cost & zero children removed: {} links removed: {removed}, extras:{extras}",
            zero.len()
        );
    }
}

fn child_to_parents(vars: &IndexMap<ClassId, ClassILP>) -> IndexMap<ClassId, IndexSet<ClassId>> {
    let mut child_to_parents: IndexMap<ClassId, IndexSet<ClassId>> = IndexMap::new();

    for (class_id, class_vars) in vars.iter() {
        for kids in &class_vars.childrens_classes {
            for child_class in kids {
                child_to_parents
                    .entry(child_class.clone())
                    .or_insert_with(IndexSet::new)
                    .insert(class_id.clone());
            }
        }
    }
    child_to_parents
}

/* If a node in a class has (a) equal or higher cost compared to another in that same class, and (b) its
  children are a superset of the other's, then it can be removed.
*/
fn remove_more_expensive_subsumed_nodes(vars: &mut IndexMap<ClassId, ClassILP>, config: &Config) {
    if config.remove_more_expensive_subsumed_nodes {
        let mut removed = 0;

        for class in vars.values_mut() {
            let mut children = class.as_nodes();
            children.sort_by_key(|e| (e.children_classes.len(), e.cost));

            let mut i = 0;
            while i < children.len() {
                for j in ((i + 1)..children.len()).rev() {
                    let node_b = &children[j];

                    // This removes some extractions with the same cost.
                    if children[i].cost <= node_b.cost
                        && children[i]
                            .children_classes
                            .is_subset(&node_b.children_classes)
                    {
                        class.remove_node(&node_b.member.clone());
                        children.remove(j);
                        removed += 1;
                    }
                }
                i += 1;
            }
        }

        log::info!("Removed more expensive subsumed nodes: {removed}");
    }
}

// Remove any classes that can't be reached from a root.
fn remove_unreachable_classes(
    vars: &mut IndexMap<ClassId, ClassILP>,
    roots: &[ClassId],
    config: &Config,
) {
    if config.remove_unreachable_classes {
        let mut reachable_classes: IndexSet<ClassId> = IndexSet::default();
        reachable(&*vars, roots, &mut reachable_classes);
        let initial_size = vars.len();
        vars.retain(|class_id, _| reachable_classes.contains(class_id));
        log::info!("Unreachable classes: {}", initial_size - vars.len());
    }
}

// Any node that has an empty class as a child, can't be selected, so remove the node,
// if that makes another empty class, then remove its parents
fn remove_empty_classes(vars: &mut IndexMap<ClassId, ClassILP>, config: &Config) {
    if config.remove_empty_classes {
        let mut empty_classes: std::collections::VecDeque<ClassId> = Default::default();
        for (classid, detail) in vars.iter() {
            if detail.members() == 0 {
                empty_classes.push_back(classid.clone());
            }
        }

        let mut removed = 0;
        let fresh = IndexSet::<ClassId>::new();

        let mut child_to_parents: IndexMap<ClassId, IndexSet<ClassId>> = IndexMap::new();

        for (class_id, class_vars) in vars.iter() {
            for kids in &class_vars.childrens_classes {
                for child_class in kids {
                    child_to_parents
                        .entry(child_class.clone())
                        .or_insert_with(IndexSet::new)
                        .insert(class_id.clone());
                }
            }
        }

        let mut done = FxHashSet::<ClassId>::default();

        while let Some(e) = empty_classes.pop_front() {
            if !done.insert(e.clone()) {
                continue;
            }
            let parents = child_to_parents.get(&e).unwrap_or(&fresh);
            for parent in parents {
                for i in (0..vars[parent].childrens_classes.len()).rev() {
                    if vars[parent].childrens_classes[i].contains(&e) {
                        vars[parent].remove(i);
                        removed += 1;
                    }
                }

                if vars[parent].members() == 0 {
                    empty_classes.push_back(parent.clone());
                }
            }
        }

        log::info!("Nodes removed that point to empty classes: {}", removed);
    }
}

// Any class that is a child of each node in a root, is also a root.
fn find_extra_roots(
    vars: &IndexMap<ClassId, ClassILP>,
    roots: &mut Vec<ClassId>,
    config: &Config,
) {
    if config.find_extra_roots {
        let mut extra = 0;
        let mut i = 0;
        // newly added roots will also be processed in one pass through.
        while i < roots.len() {
            let r = roots[i].clone();

            let details = vars.get(&r).unwrap();
            if details.childrens_classes.len() == 0 {
                continue;
            }

            let mut intersection = details.childrens_classes[0].clone();

            for childrens_classes in &details.childrens_classes[1..] {
                intersection = intersection
                    .intersection(childrens_classes)
                    .cloned()
                    .collect();
            }

            for r in &intersection {
                if !roots.contains(r) {
                    roots.push(r.clone());
                    extra += 1;
                }
            }
            i += 1;
        }

        log::info!("Extra roots discovered: {extra}");
    }
}

/*
For each class with one parent, move the minimum costs of the members to each node in the parent that points to it.

if we iterated through these in order, from child to parent, to parent, to parent.. it could be done in one pass.
*/
fn pull_up_costs(vars: &mut IndexMap<ClassId, ClassILP>, roots: &[ClassId], config: &Config) {
    if config.pull_up_costs {
        let mut count = 0;
        let mut changed = true;
        let child_to_parent = classes_with_single_parent(&*vars);

        while (count < 10) && changed {
            log::info!("Classes with a single parent: {}", child_to_parent.len());
            changed = false;
            count += 1;
            for (child, parent) in &child_to_parent {
                if child == parent {
                    continue;
                }
                if roots.contains(child) {
                    continue;
                }
                if vars[child].members() == 0 {
                    continue;
                }

                // Get the minimum cost of members of the children
                let min_cost = vars[child]
                    .costs
                    .iter()
                    .min()
                    .unwrap_or(&Cost::default())
                    .into_inner();

                assert!(min_cost >= 0.0);
                if min_cost == 0.0 {
                    continue;
                }
                changed = true;

                // Now remove it from each member
                for c in &mut vars[child].costs {
                    *c -= min_cost;
                    assert!(c.into_inner() >= 0.0);
                }
                // Add it onto each node in the parent that refers to this class.
                let indices: Vec<_> = vars[parent]
                    .childrens_classes
                    .iter()
                    .enumerate()
                    .filter(|&(_, c)| c.contains(child))
                    .map(|(id, _)| id)
                    .collect();

                assert!(!indices.is_empty());

                for id in indices {
                    vars[parent].costs[id] += min_cost;
                }
            }
        }
    }
}

/* If a class has a single parent class,
then move the children from the child to the parent class.

There could be a long chain of single parent classes - which this handles
(badly) by looping through a few times.

*/

fn pull_up_with_single_parent(
    vars: &mut IndexMap<ClassId, ClassILP>,
    roots: &[ClassId],
    config: &Config,
) {
    if config.pull_up_single_parent {
        for _i in 0..10 {
            let child_to_parent = classes_with_single_parent(&*vars);
            log::info!("Classes with a single parent: {}", child_to_parent.len());

            let mut pull_up_count = 0;
            for (child, parent) in &child_to_parent {
                if child == parent {
                    continue;
                }

                if roots.contains(child) {
                    continue;
                }

                if vars[child].members.len() != 1 {
                    continue;
                }

                if vars[child].childrens_classes.first().unwrap().is_empty() {
                    continue;
                }

                let found = vars[parent]
                    .childrens_classes
                    .iter()
                    .filter(|c| c.contains(child))
                    .count();

                if found != 1 {
                    continue;
                }

                let idx = vars[parent]
                    .childrens_classes
                    .iter()
                    .position(|e| e.contains(child))
                    .unwrap();

                let child_descendants = vars
                    .get(child)
                    .unwrap()
                    .childrens_classes
                    .first()
                    .unwrap()
                    .clone();

                let parent_descendants: &mut IndexSet<ClassId> = vars
                    .get_mut(parent)
                    .unwrap()
                    .childrens_classes
                    .get_mut(idx)
                    .unwrap();

                for e in &child_descendants {
                    parent_descendants.insert(e.clone());
                }

                vars.get_mut(child)
                    .unwrap()
                    .childrens_classes
                    .first_mut()
                    .unwrap()
                    .clear();

                pull_up_count += 1;
            }
            log::info!("Pull up count: {pull_up_count}");
            if pull_up_count == 0 {
                break;
            }
        }
    }
}

// Remove any nodes that alone cost more than the total of a solution.
// For example, if the lowest the sum of roots can be is 12, and we've found an approximate
// solution already that is 15, then any non-root node that costs more than 3 can't be selected
// in the optimal solution.

fn remove_high_cost(
    vars: &mut IndexMap<ClassId, ClassILP>,
    initial_result_cost: NotNan<f64>,
    roots: &[ClassId],
    config: &Config,
) {
    if config.remove_high_cost_nodes {
        debug_assert_eq!(
            roots.len(),
            roots.iter().collect::<std::collections::HashSet<_>>().len(),
            "All ClassId in roots must be unique"
        );

        let lowest_root_cost_sum: Cost = roots
            .iter()
            .filter_map(|root| vars[root].costs.iter().min())
            .sum();

        let mut removed = 0;

        for (class_id, class_details) in vars.iter_mut() {
            for i in (0..class_details.costs.len()).rev() {
                let cost = &class_details.costs[i];
                let this_root: Cost = if roots.contains(class_id) {
                    *class_details.costs.iter().min().unwrap()
                } else {
                    Cost::default()
                };

                if cost
                    > &(initial_result_cost - lowest_root_cost_sum + this_root + EPSILON_ALLOWANCE)
                {
                    class_details.remove(i);
                    removed += 1;
                }
            }
        }
        log::info!("Removed high-cost nodes: {}", removed);
    }
}

// Remove nodes with any (a) child pointing back to its own class,
// or (b) any child pointing to the sole root class.
fn remove_with_loops(vars: &mut IndexMap<ClassId, ClassILP>, roots: &[ClassId], config: &Config) {
    if config.remove_self_loops {
        let mut removed = 0;
        for (class_id, class_details) in vars.iter_mut() {
            for i in (0..class_details.childrens_classes.len()).rev() {
                if class_details.childrens_classes[i]
                    .iter()
                    .any(|cid| *cid == *class_id || (roots.len() == 1 && roots[0] == *cid))
                {
                    class_details.remove(i);
                    removed += 1;
                }
            }
        }

        log::info!("Omitted looping nodes: {}", removed);
    }
}

// Mapping from child class to parent classes
fn classes_with_single_parent(vars: &IndexMap<ClassId, ClassILP>) -> IndexMap<ClassId, ClassId> {
    let mut child_to_parents: IndexMap<ClassId, IndexSet<ClassId>> = IndexMap::new();

    for (class_id, class_vars) in vars.iter() {
        for kids in &class_vars.childrens_classes {
            for child_class in kids {
                child_to_parents
                    .entry(child_class.clone())
                    .or_insert_with(IndexSet::new)
                    .insert(class_id.clone());
            }
        }
    }

    // return classes with only one parent
    child_to_parents
        .into_iter()
        .filter_map(|(child_class, parents)| {
            if parents.len() == 1 {
                Some((child_class, parents.into_iter().next().unwrap()))
            } else {
                None
            }
        })
        .collect()
}

//Set of classes that can be reached from the [classes]
fn reachable(
    vars: &IndexMap<ClassId, ClassILP>,
    classes: &[ClassId],
    is_reachable: &mut IndexSet<ClassId>,
) {
    for class in classes {
        if is_reachable.insert(class.clone()) {
            let class_vars = vars.get(class).unwrap();
            for kids in &class_vars.childrens_classes {
                for child_class in kids {
                    reachable(vars, &[child_class.clone()], is_reachable);
                }
            }
        }
    }
}

// // Adds constraints to stop the cycle.
// fn block_cycle(model: &mut Model, cycle: &Vec<ClassId>, vars: &IndexMap<ClassId, ClassILP>) {
//     if cycle.is_empty() {
//         return;
//     }
//     let mut blocking = Vec::new();
//     for i in 0..cycle.len() {
//         let current_class_id = &cycle[i];
//         let next_class_id = &cycle[(i + 1) % cycle.len()];

//         let mut this_level = Vec::default();
//         for node in &vars[current_class_id].as_nodes() {
//             if node.children_classes.contains(next_class_id) {
//                 this_level.push(node.variable);
//             }
//         }

//         assert!(!this_level.is_empty());

//         if this_level.len() == 1 {
//             blocking.push(this_level[0]);
//         } else {
//             let blocking_var = model.add_binary();
//             blocking.push(blocking_var);
//             for n in this_level {
//                 let row = model.add_row();
//                 model.set_row_upper(row, 0.0);
//                 model.set_weight(row, n, 1.0);
//                 model.set_weight(row, blocking_var, -1.0);
//             }
//         }
//     }

//     //One of the edges between nodes in the cycle shouldn't be activated:
//     let row = model.add_row();
//     model.set_row_upper(row, blocking.len() as f64 - 1.0);
//     for b in blocking {
//         model.set_weight(row, b, 1.0)
//     }
// }

#[derive(Clone)]
enum TraverseStatus {
    Doing,
    Done,
}

/*
Returns the simple cycles possible from the roots.

Because the number of simple cycles can be factorial in the number
of nodes, this can be very slow.

Imagine a 20 node complete graph with one root. From the first node you have
19 choices, then from the second 18 choices, etc.  When you get to the second
last node you go back to the root. There are about 10^17 length 18 cycles.

So we limit how many can be found.
*/
const CYCLE_LIMIT: usize = 1000;

fn find_cycles_in_result(
    extraction_result: &ExtractionResult,
    vars: &IndexMap<ClassId, ClassILP>,
    roots: &[ClassId],
) -> Vec<Vec<ClassId>> {
    let mut status = IndexMap::<ClassId, TraverseStatus>::default();
    let mut cycles = vec![];
    for root in roots {
        let mut stack = vec![];
        cycle_dfs(
            extraction_result,
            vars,
            root,
            &mut status,
            &mut cycles,
            &mut stack,
        )
    }
    cycles
}

fn cycle_dfs(
    extraction_result: &ExtractionResult,
    vars: &IndexMap<ClassId, ClassILP>,
    class_id: &ClassId,
    status: &mut IndexMap<ClassId, TraverseStatus>,
    cycles: &mut Vec<Vec<ClassId>>,
    stack: &mut Vec<ClassId>,
) {
    match status.get(class_id).cloned() {
        Some(TraverseStatus::Done) => (),
        Some(TraverseStatus::Doing) => {
            // Get the part of the stack between the first visit to the class and now.
            let mut cycle = vec![];
            if let Some(pos) = stack.iter().position(|id| id == class_id) {
                cycle.extend_from_slice(&stack[pos..]);
            }
            cycles.push(cycle);
        }
        None => {
            if cycles.len() > CYCLE_LIMIT {
                return;
            }
            status.insert(class_id.clone(), TraverseStatus::Doing);
            stack.push(class_id.clone());
            let node_id = &extraction_result.choices[class_id];
            for child_cid in vars[class_id].get_children_of_node(node_id) {
                cycle_dfs(extraction_result, vars, child_cid, status, cycles, stack)
            }
            let last = stack.pop();
            assert_eq!(*class_id, last.unwrap());
            status.insert(class_id.clone(), TraverseStatus::Done);
        }
    }
}

// mod test {
//     use super::Config;
//     use crate::{
//         faster_ilp_cbc::extract, generate_random_egraph, ELABORATE_TESTING, EPSILON_ALLOWANCE,
//     };
//     use rand::Rng;
//     pub type Cost = ordered_float::NotNan<f64>;

//     pub fn generate_random_config() -> Config {
//         let mut rng = rand::thread_rng();
//         Config {
//             pull_up_costs: rng.gen(),
//             remove_self_loops: rng.gen(),
//             remove_high_cost_nodes: rng.gen(),
//             remove_more_expensive_subsumed_nodes: rng.gen(),
//             remove_unreachable_classes: rng.gen(),
//             pull_up_single_parent: rng.gen(),
//             take_intersection_of_children_in_class: rng.gen(),
//             move_min_cost_of_members_to_class: rng.gen(),
//             find_extra_roots: rng.gen(),
//             remove_empty_classes: rng.gen(),
//             return_improved_on_timeout: rng.gen(),
//             remove_single_zero_cost: rng.gen(),
//         }
//     }

//     fn all_disabled() -> Config {
//         return Config {
//             pull_up_costs: false,
//             remove_self_loops: false,
//             remove_high_cost_nodes: false,
//             remove_more_expensive_subsumed_nodes: false,
//             remove_unreachable_classes: false,
//             pull_up_single_parent: false,
//             take_intersection_of_children_in_class: false,
//             move_min_cost_of_members_to_class: false,
//             find_extra_roots: false,
//             remove_empty_classes: false,
//             return_improved_on_timeout: false,
//             remove_single_zero_cost: false,
//         };
//     }

//     const CONFIGS_TO_TEST: i64 = 150;

//     fn test_configs(config: &Vec<Config>, log_path: impl AsRef<std::path::Path>) {
//         const RANDOM_EGRAPHS_TO_TEST: i64 = if ELABORATE_TESTING {
//             1000000 / CONFIGS_TO_TEST
//         } else {
//             250 / CONFIGS_TO_TEST
//         };

//         for _ in 0..RANDOM_EGRAPHS_TO_TEST {
//             let egraph = generate_random_egraph();

//             if !log_path.as_ref().to_str().unwrap_or("").is_empty() {
//                 egraph.to_json_file(&log_path).unwrap();
//             }

//             let mut results: Option<Cost> = None;
//             for c in config {
//                 let extraction = extract(&egraph, &egraph.root_eclasses, c, u32::MAX);
//                 extraction.check(&egraph);
//                 let dag_cost = extraction.dag_cost(&egraph, &egraph.root_eclasses);
//                 if results.is_some() {
//                     assert!(
//                         (dag_cost.into_inner() - results.unwrap().into_inner()).abs()
//                             < EPSILON_ALLOWANCE
//                     );
//                 }
//                 results = Some(dag_cost);
//             }
//         }
//     }

//     macro_rules! create_tests {
//     ($($name:ident),*) => {
//         $(
//             #[test]
//             fn $name() {
//                 let mut configs = vec![Config::default(), all_disabled()];

//                 for _ in 0..CONFIGS_TO_TEST {
//                     configs.push(generate_random_config());
//                 }
//                 test_configs(&configs, crate::test_save_path(stringify!($name)));
//             }
//         )*
//     }
// }

//     // So the test runner uses more of my cores.
//     create_tests!(
//         random0, random1, random2, random3, random4, random5, random6, random7, random8, random9,
//         random10
//     );
// }
