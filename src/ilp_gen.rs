use std::fs::File;
use std::io::Write;
use indexmap::IndexMap;
use indexmap::IndexSet;
use egraph_serialize::EGraph as SerializedEGraph;
use egraph_serialize::ClassId;
use egraph_serialize::NodeId;

/// 对字符串进行简单处理，转换成只含字母数字和下划线的变量名
fn sanitize(s: &ClassId) -> String {
    s.to_string().chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

/// 辅助函数：获取某个候选节点的子类集合（封装 node.children 的逻辑）
/// 这里假定 egraph 中每个节点都有 .children 字段，每个子节点可获得其 eclass。
fn node_children_classes(egraph: &SerializedEGraph, node_id: &NodeId) -> IndexSet<ClassId> {
    egraph[node_id].children.iter().cloned().collect()
}

/// 生成 ILP 文件（LP 格式），使得 CPLEX 或 Gurobi 能够调用该文件求解。
/// 这里生成的模型与第一份代码（基于 coin‑cbc 的版本）功能完全等价。
///
/// 参数说明：
/// - `egraph`: 输入的 e-graph 数据结构  
/// - `roots`: 根 eclass 列表  
/// - `file_path`: 要写入的 LP 文件路径
pub fn generate_ilp_file(egraph: &SerializedEGraph, roots: &[ClassId], file_path: &str, warm_start: Option<Vec<NodeId>>) {
    let mut lp = String::new();

    // ============================================
    // 1. 为每个 eclass 和其中每个节点生成变量名称
    // ============================================

    // 每个 eclass 对应一个“激活变量”：A_<class_id>
    let mut class_active_vars: IndexMap<ClassId, String> = IndexMap::new();
    // 每个 eclass 中每个候选节点对应一个二进制变量：N_<class_id>_<node_index>
    let mut node_vars: IndexMap<(ClassId, u32), String> = IndexMap::new();
    // 为 block_cycles 部分，每个 eclass 还需要一个“层级变量”：L_<class_id>
    let mut level_vars: IndexMap<ClassId, String> = IndexMap::new();
    // 每个候选节点还需要一个“opposite”变量，用来配合层级约束：Opp_<class_id>_<node_index>
    let mut opposite_vars: IndexMap<(ClassId, u32), String> = IndexMap::new();

    for class in egraph.classes().values() {
        let cid = class.id.clone();
        let a_var = format!("A_{}", sanitize(&cid));
        class_active_vars.insert(cid.clone(), a_var);
        let l_var = format!("L_{}", sanitize(&cid));
        level_vars.insert(cid.clone(), l_var);
        for (idx, _node_id) in class.nodes.iter().enumerate() {
            let nid = _node_id.0;
            assert!(nid[0] == cid.0);
            let n_var = format!("N_{}_{}", nid[0], nid[1]);
            node_vars.insert((cid.clone(), nid[1]), n_var);
            let opp_var = format!("Opp_{}_{}", nid[0], nid[1]);
            opposite_vars.insert((cid.clone(), nid[1]), opp_var);
        }
    }

    // ============================================
    // 2. 写入目标函数部分（Minimize）
    // ============================================
    lp.push_str("Minimize\n obj: ");
    let mut obj_terms = Vec::new();
    for class in egraph.classes().values() {
        let cid = class.id.clone();
        for (idx, node_id) in class.nodes.iter().enumerate() {
            let nid = node_id.0;
            assert!(nid[0] == cid.0);
            let node = &egraph[node_id];
            let cost = node.cost.into_inner();
            // 只对非零成本项计入目标函数
            if cost != 0.0 {
                let var_name = &node_vars[&(cid.clone(), nid[1])];
                if cost == 1.0 {
                    obj_terms.push(format!("{}", var_name));
                } else {
                    obj_terms.push(format!("{} {}", cost, var_name));
                }
            }
        }
    }
    lp.push_str(&obj_terms.join(" + "));
    lp.push_str("\n\n");

    // ============================================
    // 3. 写入约束部分（Subject To）
    // ============================================
    lp.push_str("Subject To\n");

    // 3.1 每个 eclass 必须满足：其所有候选节点之和等于该类激活变量
    for class in egraph.classes().values() {
        let cid = class.id.clone();
        let active_var = &class_active_vars[&cid];
        let mut sum_terms = Vec::new();
        for (idx, _node_id) in class.nodes.iter().enumerate() {
            let nid = _node_id.0;
            assert!(nid[0] == cid.0);
            let node_var = &node_vars[&(cid.clone(), nid[1])];
            sum_terms.push(node_var.clone());
        }
        // 写成： N_i + N_j + ... - A_class = 0
        let constraint = format!("C_ACT_{}: {} - {} = 0\n",
            sanitize(&cid),
            sum_terms.join(" + "),
            active_var);
        lp.push_str(&constraint);
    }

    // 3.2 每个候选节点的激活必须“传递”到其子节点所在的 eclass：
/// 对于每个候选节点，对于它所有子节点所属的 eclass，
/// 添加约束： N_<class>_<i> - A_<child_class> <= 0
    for class in egraph.classes().values() {
        let cid = class.id.clone();
        for (idx, node_id) in class.nodes.iter().enumerate() {
            let nid = node_id.0;
            assert!(nid[0] == cid.0);
            let node = &egraph[node_id];
            let node_var = &node_vars[&(cid.clone(), nid[1])];
            // 收集当前候选节点所有子节点所在的 eclass（去重）
            let child_classes: IndexSet<ClassId> = node.children.iter().cloned().collect();
            for child_cid in child_classes {
                let child_active = &class_active_vars[&child_cid];
                let constraint = format!("NODE_CHILD_{}_{}_{}: {} - {} <= 0\n",
                    nid[0], nid[1], sanitize(&child_cid),
                    node_var, child_active);
                lp.push_str(&constraint);
            }
        }
    }

    // 3.3 对于每个根 eclass，要求其激活变量下界为 1
    for root in roots {
        let active_var = &class_active_vars[root];
        let constraint = format!("ROOT_{}: {} >= 1\n", sanitize(root), active_var);
        lp.push_str(&constraint);
    }

    // 3.4 额外的交集约束（如果配置启用的话）  
    // 此处可根据 Config 配置来生成针对“children 交集”的约束，
    // 例如：若一个类的所有候选节点共有一部分子类，则该部分子类必须激活。
    // 示例：

    for class in egraph.classes().values() {
        let cid = class.id.clone();
        if class.nodes.is_empty() { continue; }
        // 先取第一个候选节点的子类集合作为初始交集
        let mut intersection = node_children_classes(egraph, &class.nodes[0]);
        for node_id in &class.nodes[1..] {
            let child_set = node_children_classes(egraph, node_id);
            intersection = intersection.intersection(&child_set).cloned().collect();
        }
        for child_cid in intersection {
            let child_active = &class_active_vars[&child_cid];
            lp.push_str(&format!("INTERSECT_{}_{}: {} - {} <= 0\n",
                sanitize(&cid), sanitize(&child_cid),
                class_active_vars[&cid], child_active));
        }
    }

    // 3.4 防止环路的约束（block_cycles 部分）
    // 3.4.1 对于每个候选节点，添加： N + Opp = 1
    for class in egraph.classes().values() {
        let cid = class.id.clone();
        for (idx, _node_id) in class.nodes.iter().enumerate() {
            let nid = _node_id.0;
            assert!(nid[0] == cid.0);
            let node_var = &node_vars[&(cid.clone(), nid[1])];
            let opp_var = &opposite_vars[&(cid.clone(), nid[1])];
            let constraint = format!("OPP_{}_{}: {} + {} = 1\n",
                nid[0], nid[1], node_var, opp_var);
            lp.push_str(&constraint);
        }
    }
    // 3.4.2 如果候选节点出现自环（其子集中包含本类），则直接使该节点变量取 0
    for class in egraph.classes().values() {
        let cid = class.id.clone();
        for (idx, node_id) in class.nodes.iter().enumerate() {
            let nid = node_id.0;
            assert!(nid[0] == cid.0);
            let node = &egraph[node_id];
            let node_var = &node_vars[&(cid.clone(), nid[1])];
            let children_classes: IndexSet<ClassId> = node.children.iter().cloned().collect();
            if children_classes.contains(&cid) {
                let constraint = format!("SELF_LOOP_{}_{}: {} = 0\n", nid[0], nid[1], node_var);
                lp.push_str(&constraint);
            }
        }
    }
    // 3.4.3 对于每个候选节点和其每个非自环的子类，添加层级约束：
    // -L_parent + L_child + M * Opp >= 1
    // 其中 M 取 (#eclass 数 + 1)
    let m_const = egraph.classes().len() + 1;
    for class in egraph.classes().values() {
        let cid = class.id.clone();
        let level_var = &level_vars[&cid];
        for (idx, node_id) in class.nodes.iter().enumerate() {
            let nid = node_id.0;
            assert!(nid[0] == cid.0);
            let node = &egraph[node_id];
            let opp_var = &opposite_vars[&(cid.clone(), nid[1])];
            // 对于该候选节点中所有子节点所属的 eclass（排除与本类相同的情况）
            let child_classes: IndexSet<ClassId> = node.children.iter().cloned()
                .filter(|child_cid| child_cid != &cid)
                .collect();
            for child_cid in child_classes {
                let child_level = &level_vars[&child_cid];
                let constraint = format!(
                    "LEVEL_{}_{}_{}: {} - {} + {} {} >= 1\n",
                    nid[0], nid[1], sanitize(&child_cid),
                    child_level, level_var, m_const, opp_var);
                lp.push_str(&constraint);
            }
        }
    }

    // Start with warm start

    if let Some(warm_start) = warm_start {
        for node_id in warm_start {
            let node = &egraph[&node_id];
            let cid = node_id.0[0];
            let nid = node_id.0[1];
            let node_var = &node_vars[&(node.eclass, nid)];
            let constraint = format!("WARM_START_{}_{}: {} = 0\n", cid, nid, node_var);
            lp.push_str(&constraint);
        }
    }

    // ============================================
    // 4. 写入 Bounds 部分
    // ============================================
    lp.push_str("\nBounds\n");
    // 为每个层级变量设置下界 0，上界为 eclass 数（可根据需要调整）
    let upper_bound = egraph.classes().len();
    for (cid, level_var) in &level_vars {
        let bound_line = format!("0 <= {} <= {}\n", level_var, upper_bound);
        lp.push_str(&bound_line);
    }

    // ============================================
    // 5. 写入 Binaries 部分
    // ============================================
    lp.push_str("\nBinaries\n");
    // 列出所有二进制变量：类激活变量、候选节点变量、以及 opposite 变量
    for (_cid, active_var) in &class_active_vars {
        lp.push_str(&format!("{}\n", active_var));
    }
    for ((_cid, _idx), node_var) in &node_vars {
        lp.push_str(&format!("{}\n", node_var));
    }
    for ((_cid, _idx), opp_var) in &opposite_vars {
        lp.push_str(&format!("{}\n", opp_var));
    }

    lp.push_str("\nEnd\n");

    // ============================================
    // 6. 写入到文件
    // ============================================
    let mut file = File::create(file_path)
        .expect("无法创建 ILP 文件");
    file.write_all(lp.as_bytes())
        .expect("写入 ILP 文件失败");

    println!("ILP 文件已生成：{}", file_path);
}
