// cargo build --release
// cargo run -- --release --bound 1.25 --solver gurobi --timeout 1800 --extractor faster-greedy-dag-mt1 --pre 4 benchmark/BoolE/mul32_map.json



mod extractor;
mod ilp_gen;
use egg::egraph;
use rustc_hash::{FxHashMap, FxHashSet};
use extraction_gym::ExtractionResult;
use indexmap::{IndexMap,IndexSet};
use egraph_serialize::EGraph as SerializedEGraph;
use egraph_serialize::ClassId;
use egraph_serialize::NodeId;
use egraph_serialize::Node;
use egraph_serialize::Data;
use egraph_serialize::Cost;
use anyhow::Context;
use core::panic;
use std::default;
use std::fmt::format;
use std::result;
use std::time::Instant;
use std::path::PathBuf;
use std::fs;
use std::env;
use serde::{Serialize, Deserialize};
use serde_json::{to_string_pretty, from_str,Value};
use std::collections::{HashMap, BTreeMap,HashSet};
use linked_hash_map::{LinkedHashMap};
use std::collections::VecDeque;
use ordered_float::NotNan;
use std::process::Command;
use std::fs::File;
use std::io::Read;
use std::error::Error;
use wait_timeout::ChildExt;



fn remove_redundant_nodes(data: &mut Data, cost_func: &str) {
    let mut eclass_hashes: HashMap<u32, HashSet<NodeId>> = HashMap::new();


    let mut eclass_collect = HashMap::new();

    for (node_id, node) in data.nodes.iter() {
        let eclass = node.eclass.clone();
        if !eclass_collect.contains_key(&eclass) {
            eclass_collect.insert(eclass.clone(), Vec::<NodeId>::new());
        }
        eclass_collect.get_mut(&eclass).unwrap().push(node_id.clone());
    }

    for (eclass, node_ids) in eclass_collect.iter() {
        let mut grouped: HashMap<Vec<(ClassId, usize)>, Vec<NodeId>> = HashMap::new();
        for node_id in node_ids.iter() {
            let node = data.nodes.get(node_id).unwrap();
            let mut children_hashes: Vec<ClassId> = node.children.iter().cloned().collect();
            let mut freq_map: BTreeMap<&ClassId, usize> = BTreeMap::new();
            for elem in &children_hashes {
                *freq_map.entry(elem).or_insert(0) += 1;
            }
            // 2. Convert freq_map to ordered (String, usize) vector
            //    Using BTreeMap here ensures keys are sorted in dictionary order.
            let freq_vec: Vec<(ClassId, usize)> = freq_map
            .into_iter()
            .map(|(elem, count)| (elem.clone(), count))
            .collect();
            // 3. Insert key into corresponding group based on freq_vec
            grouped.entry(freq_vec).or_insert_with(Vec::new).push(node_id.clone());
        }

        for (children, node_ids) in grouped.iter() {
            if node_ids.len() > 1 {
                for (idx,item) in node_ids.iter().enumerate() {
                    if let Some(val) = data.nodes.get(item) {
                        if idx == 0{
                            continue;
                        }
                        else{
                            data.nodes.remove(item);
                        }
                    }
                    else{
                        panic!("node not found");
                    }
                }
            }
        }
    }
}


fn egraph_partition(data: &mut Data,factor: f32, paritioned_data: &mut Vec<Data>) -> usize {
    let nodes = &data.nodes;
    let mut mutable_nodes = nodes.clone();
    let mut parents = HashMap::new();
    for (key, node) in mutable_nodes.iter() {
        for child in node.children.iter() {
            // let child_eclass = mutable_nodes.get(child).unwrap().eclass.clone();
            if !parents.contains_key(child) {
                parents.insert(child.clone(), Vec::<NodeId>::new());
            }
            parents.get_mut(&child).unwrap().push(key.clone());
        }
    }



    let start = Instant::now();

    let mut root = Vec::<ClassId>::new();
    for (key, _) in mutable_nodes.iter() {
        let key_eclass = mutable_nodes.get(key).unwrap().eclass.clone();
        if !parents.contains_key(&key_eclass) {
            root.push(key_eclass);
        }
    }

    // println!("Root: {:?}", root);
    // println!("Root: {:?}", data.root_eclasses);
    
    let grownth_duration = start.elapsed();
    println!("remove_redundant_nodes runtime-{:?}", grownth_duration);
    
    if root.len() > 1 {
        let pseudo_root = Node {
            op: "pseudo_root".to_string(),
            id: NodeId::from((u32::MAX, 0)),
            children: root,
            eclass: ClassId::from(u32::MAX),
            cost: NotNan::new(0.0).unwrap(),
        };
        mutable_nodes.insert(NodeId::from((u32::MAX, 0)), pseudo_root);
        root = vec![ClassId::from(u32::MAX)];
    }


    let mut eclass_collect = HashMap::new();

    for (node_id, node) in mutable_nodes.iter() {
        let eclass = node.eclass.clone();
        if !eclass_collect.contains_key(&eclass) {
            eclass_collect.insert(eclass.clone(), Vec::<NodeId>::new());
        }
        eclass_collect.get_mut(&eclass).unwrap().push(node_id.clone());
    }

    let partition_num = ((1.0 / factor).round() as usize); 
    assert!(mutable_nodes.len() > partition_num);
    let num = (mutable_nodes.len() as f32 / partition_num as f32);
    // println!("num: {:?}", num);

    
    let mut visited = HashSet::new();
    let mut queue: VecDeque<ClassId> = VecDeque::new();
    let mut subgraphs = Vec::new();
    let mut current_subgraph = IndexSet::new();
    let mut current_count = 0;

    queue.push_back(root[0].clone());



    while let Some(class_id) = queue.remove(0) {
        // let class_id = mutable_nodes.get(&_class_id).unwrap().eclass.clone();

        // if visited.contains(&class_id) {
        //     continue;
        // }
        // visited.insert(class_id.clone());
        current_subgraph.insert(class_id.clone());

        if current_count as f32 >= num {
            subgraphs.push(current_subgraph.clone());
            current_subgraph.clear();
            current_count = 0;
            if subgraphs.len() == partition_num {
                break;
            }
        }

        if let Some(class_nodes) = eclass_collect.get(&class_id) {
            for (idx,class_node) in class_nodes.iter().enumerate() {
                current_count += 1;
                for child in mutable_nodes.get(class_node).unwrap().children.iter() {
                    if !visited.contains(child) {
                        queue.push_back(child.clone());
                        visited.insert(child);
                    }
                }
            }
        }
        else{
            panic!("class_id not found:{:?}", class_id);
        }
    }

    if !current_subgraph.is_empty() {
        subgraphs.push(current_subgraph);
    }




    // for (idx, subgraph) in subgraphs.iter().enumerate() {
    //     println!("subgraph{}: {:?}", idx, subgraph);
    // }

    let total_length: usize = subgraphs.iter().map(|subgraph| subgraph.len()).sum();
    let union_subgraphs: HashSet<_> = subgraphs.iter().flat_map(|subgraph| subgraph.iter()).collect();
    let eclass_keys: HashSet<_> = eclass_collect.keys().collect();
    assert_eq!(union_subgraphs, eclass_keys);

    let mut subgraph_maps: Vec<IndexMap<NodeId, Node>> = Vec::new();
    for entry in fs::read_dir("test").expect("Unable to read directory") {
        let entry = entry.expect("Unable to get entry");
        let path = entry.path();
        if path.is_file() && path.file_name().unwrap().to_str().unwrap().starts_with("subgraph_") {
        fs::remove_file(path).expect("Unable to delete file");
        }
    }
    


    // let mut _roots = Vec::<HashSet::<String>>::new();
    // _roots.push(root.iter().cloned().collect());
    for (idx,subgraph) in subgraphs.iter().enumerate() {
        let mut subgraph_map: IndexMap<NodeId, Node> = IndexMap::new();
        for class_id in subgraph.iter() {
            if let Some(node_ids) = eclass_collect.get(class_id) {
                for node_id in node_ids.iter() {
                    if let Some(node) = mutable_nodes.get(node_id) {
                        subgraph_map.insert(node_id.clone(), node.clone());
                    }
                }
            }
        }
        
        // let mut roots = HashSet::<String>::new();

        // 1) Collect needed info in a read-only pass
        let mut to_remove: HashMap<NodeId, Vec<ClassId>> = HashMap::new();
        let mut subgraph_parents: HashMap<ClassId, Vec<NodeId>> = HashMap::new();

        for (key, node) in subgraph_map.iter() {
            for child in node.children.iter() {
                // If child not in map, record for removal
                if !subgraph_map.contains_key(&NodeId::from((child.return_value(), 0))) {
                    // roots.insert(nodes.get(child).unwrap().eclass.clone());
                    to_remove.entry(key.clone())
                        .or_default()
                        .push(child.clone());
                } else {
                    // child is valid, record parents
                    // let child_eclass = subgraph_map.get(child).unwrap().eclass.clone();
                    subgraph_parents.entry(child.clone())
                        .or_default()
                        .push(key.clone());
                }
            }
        }
        // _roots.push(roots.clone());

        // 2) Mutate each node in a second pass, removing invalid children
        for (key, node) in subgraph_map.iter_mut() {
            if let Some(children_to_remove) = to_remove.get(key) {
                node.children.retain(|c| !children_to_remove.contains(c));
            }
        }

        let mut subgraph_root = IndexSet::<ClassId>::new();
        for (key, _) in subgraph_map.iter() {
            let key_eclass = subgraph_map.get(key).unwrap().eclass.clone();
            if !subgraph_parents.contains_key(&key_eclass) {
                // subgraph_root.insert(key_eclass.clone()+".0");
                subgraph_root.insert(key_eclass);
            }
        }

        if subgraph_root.len() > 1 {
            let pseudo_root = Node {
                op: format!("pseudo_root_{:?}", idx),
                id: NodeId::from((u32::MAX, 0)),
                children: subgraph_root.iter().cloned().collect(),
                eclass: ClassId::from(u32::MAX),
                cost: NotNan::new(0.0).unwrap(),
            };
            subgraph_map.insert(NodeId::from((u32::MAX, 0)), pseudo_root);
            subgraph_root = IndexSet::from([ClassId::from(u32::MAX)]);
        }

        assert_eq!(subgraph_root.len(), 1);

        let new_data = Data {
            nodes: subgraph_map.clone(),
            root_eclasses: subgraph_root.iter().cloned().collect(),
        };

        subgraph_maps.push(subgraph_map.clone());


        // let new_file_content = serde_json::to_string_pretty(&new_data).expect("Unable to serialize JSON");
        // fs::write(format!("test/subgraph_{}.json", idx), new_file_content).expect("Unable to write file");
        new_data.to_json_file(format!("test/subgraph_{}.json", idx));
        paritioned_data.push(new_data);
    }


    partition_num
}


fn collect_results(cost: HashMap<NodeId,Cost>, bound:f32, zero_node: &mut Vec<NodeId>) {
    // assert!(bound >= 1.0);
    let mut collects: HashMap<u32, Vec<(NodeId, NotNan<f64>)>> = HashMap::new();
    
    // Collect nodes by category
    for (node_id, cost) in cost.iter() {
        let classid = node_id.0[0];
        match collects.get_mut(&classid) {
            Some(v) => {
                v.push((node_id.clone(), cost.clone()));
            }
            None => {
                collects.insert(classid, vec![(node_id.clone(), cost.clone())]);
            }
        }
    }

    // Process each category
    for (_, costs) in collects.iter() {
        let mut sorted_costs = costs.clone();
        // Sort by cost from smallest to largest
        sorted_costs.sort_by(|a, b| a.1.cmp(&b.1));
        
        // Get minimum value and calculate threshold
        if let Some((_, min_cost)) = sorted_costs.first() {
            // Calculate threshold: minimum value * bound
            let threshold = *min_cost * NotNan::new(bound as f64).expect("bound is not NaN");
            
            // Add nodes greater than threshold to zero_node
            for &(ref node_id, cost) in &sorted_costs {
                if cost > threshold {
                    zero_node.push(node_id.clone());
                }
            }
        }
    }
}

// fn ilp_solver_gurobi(egraph: &SerializedEGraph, warm_start: Option<Vec<NodeId>>) -> Result<ExtractionResult, Box<dyn std::error::Error>> {
//     ilp_gen::generate_ilp_file(egraph, &egraph.root_eclasses, "lp/total.lp", warm_start);

//     // 2. 调用 gurobi_cl 命令行求解，导出解文件 result.sol
//     //    这里用到 Gurobi 的命令行参数: "ResultFile=result.sol total.lp"
//     //    也可以先把 "total.lp" 放前面，都可以。
//     let status = Command::new("gurobi_cl")
//         .args([
//             "InputFile=lp/total_gurobi.mst",
//             "ResultFile=lp/result.sol",  // 告诉 Gurobi 把解写到 result.sol
//             "lp/total.lp"
//         ])
//         .status()?;

//     if !status.success() {
//         eprintln!("gurobi_cl did not exit successfully.");
//         // 此处返回一个自定义错误也可以
//         return Err("gurobi_cl failed".into());
//     }

//     // 3. 读取刚才生成的 result.sol 文件
//     let sol_contents = fs::read_to_string("lp/result.sol")?;

//     let mut solution:ExtractionResult = ExtractionResult::new(IndexMap::new());
//     for line in sol_contents.lines() {
//         let line = line.trim();
//         // 跳过空行 或 注释行
//         if line.is_empty() || line.starts_with('#') {
//             continue;
//         }

//         // 按空格分割得到 [变量名, 变量值]
//         let parts: Vec<_> = line.split_whitespace().collect();
//         if parts.len() == 2 {
//             let var_name = parts[0];
//             if var_name.starts_with("N_") {
//                 let cid = var_name[2..].split('_').next().unwrap().parse::<u32>().unwrap();
//                 let nid = var_name[2..].split('_').nth(1).unwrap().parse::<u32>().unwrap();
//                 let var_value_str = parts[1];
//                 let val = var_value_str.parse::<i32>()?;
//                 if val == 1 {
//                     if !solution.choices.contains_key(&ClassId::from(cid)) {
//                         solution.choose(ClassId::from(cid), NodeId::from((cid, nid)));
//                     }
//                     else{
//                         panic!("classid already exists");
//                     }
//                 }
//             }
//         }
//     }

//     // 5. 返回解析后的解
//     Ok(solution)
// }

fn write_json_result<T: serde::Serialize>(filename: &str, data: &T) {
    let json_result = to_string_pretty(data).unwrap();
    //let _ = fs::create_dir_all("out_json");
    let __ = fs::write(filename, json_result);
}

pub fn parse_cplex_solution(file_path: &str) -> Result<HashMap<String, f64>, Box<dyn Error>> {
    // 读取文件内容
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    
    // 查找变量部分
    let variables_start = contents.find("<variables>")
        .ok_or("Could not find <variables> tag")?;
    let variables_end = contents.find("</variables>")
        .ok_or("Could not find </variables> tag")?;
    
    // 提取变量部分
    let variables_section = &contents[variables_start..variables_end + 12]; // +12 for "</variables>"
    
    // 使用正则表达式提取N_开头的变量
    let mut variables = HashMap::new();
    let lines: Vec<&str> = variables_section.lines().collect();
    
    for line in lines {
        if line.contains("<variable") && line.contains("name=\"N_") {
            // 提取名称
            let name_start = line.find("name=\"")
                .map(|pos| pos + 6)
                .ok_or("Could not find name attribute")?;
            let name_end = line[name_start..].find("\"")
                .map(|pos| name_start + pos)
                .ok_or("Could not find end of name attribute")?;
            let name = &line[name_start..name_end];
            
            // 提取值
            let value_start = line.find("value=\"")
                .map(|pos| pos + 7)
                .ok_or("Could not find value attribute")?;
            let value_end = line[value_start..].find("\"")
                .map(|pos| value_start + pos)
                .ok_or("Could not find end of value attribute")?;
            let value_str = &line[value_start..value_end];
            let value = value_str.parse::<f64>()?;
            
            // 只保存N_开头的变量
            if name.starts_with("N_") {
                variables.insert(name.to_string(), value);
            }
        }
    }
    
    Ok(variables)
}


// fn ilp_solver_cplex(egraph: &SerializedEGraph, warm_start: Option<Vec<NodeId>>) -> Result<ExtractionResult, Box<dyn std::error::Error>> {
//     ilp_gen::generate_ilp_file(egraph, &egraph.root_eclasses, "lp/total.lp", warm_start);

//     let status = Command::new("cplex")
//     .args([
//         "-c",
//         "set mip display 4",
//         "read lp/total.lp",  // 告诉 Gurobi 把解写到 result.sol
//         "read lp/total_cplex.mst",
//         "mip start",
//         "optimize",
//         "write lp/cplex_result.sol",
//         "y"
//     ])
//     .status()?;

//     if !status.success() {
//         eprintln!("cplex did not exit successfully.");
//         // 此处返回一个自定义错误也可以
//         return Err("cplex failed".into());
//     }

//     let sol_contents = parse_cplex_solution("lp/cplex_result.sol")?;

//     let mut solution:ExtractionResult = ExtractionResult::new(IndexMap::new());
//     for (var_name, var_value) in sol_contents.iter() {
//         let cid = var_name[2..].split('_').next().unwrap().parse::<u32>().unwrap();
//         let nid = var_name[2..].split('_').nth(1).unwrap().parse::<u32>().unwrap();
//         if *var_value == 1.0 {
//             if !solution.choices.contains_key(&ClassId::from(cid)) {
//                 solution.choose(ClassId::from(cid), NodeId::from((cid, nid)));
//             }
//             else{
//                 panic!("classid already exists");
//             }
//         }
//     }

//     Ok(solution)
// }

#[derive(Default, Clone,Serialize)]
pub struct ExtractionResultttt {
    pub choices: IndexMap<ClassId, NodeId>,
}

impl ExtractionResultttt {

    pub fn new_empty() -> Self {
        Self {
            choices: IndexMap::<ClassId, NodeId>::default(),
        }
    }
}

fn gen_gurobi_mst(activated: &FxHashSet<NodeId>, results: &ExtractionResult, filename: &str) {
    let mut str = String::new();
    for (cid,nid) in results.choices.iter() {
        if activated.contains(nid) {
            str.push_str(&format!("N_{}_{} 1\n", cid.0, nid.0[1]));
        }
        else{
            str.push_str(&format!("A_{} 0\n", cid.0));
        }
    }
    fs::write(filename, str).expect("Unable to write file");
}

// fn gen_cplex_mst(activated: &FxHashSet<NodeId>, results: &ExtractionResult, filename: &str) {
//     let mut str = String::new();
//     let start_str = "<?xml version = \"1.0\" ?>
// <CPLEXSolutions>
//  <CPLEXSolution>
//   <header
//    objectiveValue=\"0\"
//    />
//   <variables>\n".to_string();
//     let end_str = "  </variables>
//  </CPLEXSolution>
// </CPLEXSolutions>".to_string();
//     str.push_str(&start_str);
//     // for nid in activated.iter() {
//     //     str.push_str(&format!("   <variable name=\"N_{}_{}\" value=\"1\"/>\n", nid.0[0], nid.0[1]));
//     // }
//     for (cid,nid) in results.choices.iter() {
//         if activated.contains(nid) {
//             str.push_str(&format!("   <variable name=\"N_{}_{}\" value=\"1\"/>\n", nid.0[0], nid.0[1]));
//         }
//         else{
//             str.push_str(&format!("   <variable name=\"A_{}\" value=\"0\"/>\n", cid.0));
//         }
//     }
//     str.push_str(&end_str);
//     fs::write(filename, str).expect("Unable to write file");
// }

fn main() {

    // Get command-line arguments
    let args: Vec<String> = env::args().collect();

    // Initialize variables
    let mut filename = String::new();
    let mut extractor = String::from("faster-greedy-dag-mt1");
    let mut bound: f32 = 1.25;
    let mut solver = String::from("gurobi"); // Default solver
    let mut timeout_secs: u64 = 1800; // Default timeout (30 minutes)
    let mut pre_flag: i32 = 2; // Flag for preprocessing only
    let mut result= ExtractionResult::new_empty();
    
    // Parse command line arguments
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--bound" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<f32>() {
                        Ok(value) => {
                            bound = value;
                        },
                        Err(_) => {
                            panic!("Error: Invalid bound value");
                        }
                    }
                    i += 2;
                } else {
                    panic!("Error: Missing value for --bound parameter");
                }
            },
            "--solver" => {
                if i + 1 < args.len() {
                    let solver_name = args[i + 1].to_lowercase();
                    if solver_name == "gurobi" || solver_name == "cplex" || solver_name == "cpsat" {
                        solver = solver_name;
                    } else {
                        panic!("Error: Unknown solver '{}'. Use 'gurobi', 'cplex' or 'cpsat'", args[i + 1]);
                    }
                    i += 2;
                } else {
                    panic!("Error: Missing value for --solver parameter");
                }
            },
            "--timeout" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<u64>() {
                        Ok(value) => {
                            if value > 0 {
                                timeout_secs = value;
                            } else {
                                panic!("Error: Timeout must be greater than 0 seconds");
                            }
                        },
                        Err(_) => {
                            panic!("Error: Invalid timeout value");
                        }
                    }
                    i += 2;
                } else {
                    panic!("Error: Missing value for --timeout parameter");
                }
            },
            "--extractor" => {
                if i + 1 < args.len() {
                    let extractor_name = args[i + 1].to_lowercase();
                    if extractor::extractors().keys().any(|key| key.to_lowercase() == extractor_name) {
                        extractor = extractor_name;
                    } else {
                        panic!("Error: Unknown solver '{}'. Select from {:?}", args[i + 1], extractor::extractors().keys());
                    }
                    i += 2;
                } else {
                    panic!("Error: Missing value for --extractor parameter");
                }
            }
            "--pre" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<i32>() {
                        Ok(value) => {
                            if value == 0 || value == 1 || value == 2 || value == 3 || value == 4 || value == 5 {
                                pre_flag = value;
                            } else {
                                panic!("Error: Pre flag must be 0, 1, or 2");
                            }
                        },
                        Err(_) => {
                            panic!("Error: Invalid pre flag value");
                        }
                    }
                    i += 2;
                } else {
                    panic!("Error: Missing value for --pre parameter");
                }
            },
            arg => {
                // Assume this is the filename
                filename = arg.to_string();
                i += 1;
            }
        }
    }

    if pre_flag == 1 || pre_flag == 3 {
        bound = -1.0;
    }

    // Check if filename is provided
    if filename.is_empty() {
        eprintln!("Error: No input file specified");
        eprintln!("Usage: {} [OPTIONS] <input.json>", args[0]);
        eprintln!("Options:");
        eprintln!("  --bound <value>      Bound value (default: 1.25)");
        eprintln!("  --solver <name>      Solver: gurobi, cplex, or cpsat (default: gurobi)");
        eprintln!("  --timeout <seconds>  Timeout in seconds (default: 1800)");
        eprintln!("  --extractor <name>   Extractor name (default: faster-greedy-dag-mt1)");
        eprintln!("  --pre <flag>         Pre-processing flag: 0-5 (default: 2)");
        eprintln!("");
        eprintln!("Example: {} --bound 1.1 --solver gurobi input.json", args[0]);
        std::process::exit(1);
    }

    let path = std::path::Path::new(&filename);

    let ext = path.extension()
        .expect("Error: 文件没有扩展名");
    if ext.to_string_lossy().to_lowercase() != "json" {
        panic!("Error: 文件类型不是 json");
    }

    let base_name = path.file_stem()
        .expect("Error: 无法提取文件名主体")
        .to_string_lossy()
        .to_string();

    let lp_file_path = format!("file/lp/{}_{}.lp", base_name, bound);
    let mst_file_path = format!("file/start/{}_{}.mst", base_name, bound);
    let zero_file_path = format!("file/ZeroNode/{}_{}_{}.mst", base_name, bound, solver);
    let redundancy_file_path = format!("file/redundancy/{}_{}.json", base_name, bound);
    let result_file = format!("file/result/{}_{}_{}.sol", base_name, bound, solver);
    let pool = format!("file/pool/{}_{}_{}", base_name, bound, solver);
    let log_file = format!("file/log/{}_{}_{}.log", base_name, bound, solver);

    if extractor != "faster-greedy-dag" && extractor != "faster-greedy-dag-mt1" && extractor != "faster-greedy-dag-mt2" {
        pre_flag = 5;
    }

    println!("Using solver: {}", solver);
    println!("Using extractor: {}", extractor);
    println!("Using bound value: {}", bound);
    println!("Using timeout: {} seconds", timeout_secs);
    println!("Pre-processing mode: {}", match pre_flag {
        0 => "Solver only (skip LP generation)",
        1 => "Generate LP file only (no solving) -- wo warm start",
        2 => "Generate LP file only (no solving) -- w warm start",
        3 => "Full run (generate LP and solve) -- wo warm start",
        4 => "Full run (generate LP and solve) -- w warm start",
        5 => "Heuristic run only",
        _ => {
            panic!("unknown pre-flag");
        }
    });
    println!("LP file path: {}", lp_file_path);
    println!("MST file path: {}", mst_file_path);
    println!("Zero Node file path: {}", zero_file_path);


    let mut zero_node = Vec::<NodeId>::new();
    let mut runtime: f64 = 0.0;
    let mut total_egraph;
    
    // Create all necessary directories
    let directories = vec![
        "file",
        "file/lp",
        "file/start", 
        "file/ZeroNode",
        "file/result",
        "file/log",
        "file/redundancy",
        &pool,
    ];
    
    for dir in directories {
        fs::create_dir_all(dir).unwrap_or_else(|err| {
            eprintln!("Warning: Could not create directory '{}': {}", dir, err);
        });
    }

    if pre_flag == 0 {
        println!("Skipping extraction phase (--pre=0 mode)");
        let empty_data = Data {
            nodes: IndexMap::new(),
            root_eclasses: Vec::new(),
        };
        total_egraph = SerializedEGraph::from_Data(&empty_data)
            .with_context(|| format!("Failed to create empty egraph"))
            .unwrap();
    }
    else {
        let file_path: PathBuf = env::current_dir().unwrap().join(&filename);
        println!("Loading file: {}", file_path.display());
 
        let mut data: Data = Data::from_json_file(&file_path)
            .with_context(|| format!("Failed to parse {filename}"))
            .unwrap();
        // remove_redundant_nodes(&mut data, "dag");
        data.to_json_file(redundancy_file_path.clone());
        let mut paritioned_data = Vec::<Data>::new();
 
        total_egraph = SerializedEGraph::from_Data(&data).with_context(|| format!("Failed to get egraph")).unwrap();


        // remove lp and mst file if exist
        if std::path::Path::new(&lp_file_path).exists() {
            fs::remove_file(&lp_file_path)
                .unwrap_or_else(|err| eprintln!("Failed to delete {}: {}", lp_file_path, err));
        }
        if std::path::Path::new(&mst_file_path).exists() {
            fs::remove_file(&mst_file_path)
                .unwrap_or_else(|err| eprintln!("Failed to delete {}: {}", mst_file_path, err));
        }
    }

    if pre_flag == 2 || pre_flag == 4 || pre_flag == 5 {
        let mut extractors: indexmap::IndexMap<&str, extractor::ExtractorDetail, _> = extractor::extractors();
        extractors.retain(|_, ed| ed.get_use_for_bench());
        let extractor_name: String = extractor.into();
        let ed = extractors
            .get(extractor_name.as_str())
            .with_context(|| format!("Unknown extractor: {extractor_name}"))
            .unwrap();
        let start = Instant::now();
        result = ed.get_extractor().extract(&total_egraph, &total_egraph.root_eclasses);
        let grownth_duration = start.elapsed();
        runtime += grownth_duration.as_secs_f64();
        result.check(&total_egraph);
        let tree = result.tree_cost(&total_egraph, &total_egraph.root_eclasses);
        let dag = result.dag_cost(&total_egraph, &total_egraph.root_eclasses);
        let depth = result.depth_cost(&total_egraph, &total_egraph.root_eclasses);
        println!("{:<18}: runtime-{} tree:{} dag:{} depth: {}", extractor_name, runtime, tree, dag, depth);
    }

    if pre_flag == 1 || pre_flag == 2 || pre_flag == 3 || pre_flag == 4 {
        // Generate MST files based on solver type - only when pre_flag == 1
        if (pre_flag == 2 || pre_flag == 4) {
            collect_results(result.cost.clone(), bound, &mut zero_node);
            println!("zero_node: {:?}", zero_node.len());
            let activated: FxHashSet<NodeId> = result.activate_nodes(&total_egraph, &total_egraph.root_eclasses);
            if solver == "gurobi" || solver == "cplex" {
                gen_gurobi_mst(&activated,&result, &mst_file_path);
                println!("MST file successfully generated at: {}", mst_file_path);
            }
            //  else if solver == "cplex" {
            //     gen_cplex_mst(&activated,&result, &mst_file_path);
            //     println!("MST file successfully generated at: {}", mst_file_path);
            // }
            else if solver == "cpsat" {
                let mut str = String::new();
                for nid in zero_node.iter() {
                    str.push_str(&format!("N_{}_{}\n", nid.0[0], nid.0[1]));
                }
                fs::write(zero_file_path.clone(), str).expect("Unable to write file");
                println!("Zero Node file successfully generated at: {}", zero_file_path);
                gen_gurobi_mst(&activated,&result, &mst_file_path);
                println!("MST file successfully generated at: {}", mst_file_path);
            }
            else {
                panic!("Error: Unknown solver: {}", solver);
            }

            println!("Generating LP file: {}", lp_file_path);
            ilp_gen::generate_ilp_file(&total_egraph, &total_egraph.root_eclasses, &lp_file_path, Some(zero_node));
        }
        else{
            ilp_gen::generate_ilp_file(&total_egraph, &total_egraph.root_eclasses, &lp_file_path, None);
        }
        println!("LP file successfully generated at: {}", lp_file_path);
    }

    if pre_flag == 0 || pre_flag == 3 || pre_flag == 4 {
        println!("Running solver: {}", solver);
        
        // Make sure the LP file exists
        if !std::path::Path::new(&lp_file_path).exists() {
            panic!("Error: LP file not found: {}", lp_file_path);
        }


        // Check if MST file exists when in solver-only mode
        if !std::path::Path::new(&mst_file_path).exists() {
            eprintln!("Warning: MST file not found: {}", mst_file_path);
            eprintln!("Continuing without warm start solution");
        }

        if solver == "cpsat" && !std::path::Path::new(&zero_file_path).exists() {
            eprintln!("Warning: Zero Node file not found: {}", zero_file_path);
            eprintln!("Continuing without warm start solution");
        }

        // Run the selected solver as a child process
        let mut runtime_solve: f64 = 0.0;
        let start_solve = Instant::now();
        let mut child = match solver.as_str() {
            "gurobi" => {
                // Using Gurobi
                let mut cmd = Command::new("gurobi/gurobi_solver");
                let mut args = vec![
                    "--lp_file".to_string(),
                    lp_file_path.clone(),
                    "--output_file".to_string(), 
                    result_file.clone(),
                    "--time_limit".to_string(),
                    timeout_secs.to_string(),
                    // "--solution_pool_dir".to_string(),
                    // pool,
                    "--log_file".to_string(),
                    log_file,
                ];
                
                // Add MST file if it exists
                if std::path::Path::new(&mst_file_path).exists() {
                    args.insert(0, "--mst_file".to_string());
                    args.insert(1, mst_file_path.clone());
                }

                
                println!("command: {}", args.join(" "));
                
                cmd.args(args)
                    .spawn()
                    .expect("Failed to start Gurobi solver")

                
            },
            "cplex" => {
                // Using CPLEX
                let mut cmd = Command::new("cplex/cplex_solver");
                let mut args = vec![
                    "--lp_file".to_string(),
                    lp_file_path.clone(),
                    "--output_file".to_string(), 
                    result_file.clone(),
                    "--time_limit".to_string(),
                    timeout_secs.to_string(),
                    // "--solution_pool_dir".to_string(),
                    // pool,
                    "--log_file".to_string(),
                    log_file,
                ];
                
                // Add MST file if it exists
                if std::path::Path::new(&mst_file_path).exists() {
                    args.insert(0, "--mst_file".to_string());
                    args.insert(1, mst_file_path.clone());
                }

                // clear;cplex/cplex_solver --lp_file file/lp/serialized_egraph_32_1.25.lp --output_file file/result/serialized_egraph_32_1.25_cplex.sol --log_file file/log/serialized_egraph_32_1.25_cplex.log --time_limit 50 --solution_pool_dir pool --mst_file file/start/serialized_egraph_32_1.25_cplex.mst

                println!("command: {}", args.join(" "));
                
                cmd.args(args)
                    .spawn()
                    .expect("Failed to start CPLEX solver")
            },
            "cpsat" => {
                let mut cmd = Command::new("cpsat/cpsat");
                let mut args = vec![
                    "--egraph_json_file".to_string(),
                    redundancy_file_path.to_string(),
                    "--output_sol_file".to_string(), 
                    result_file.clone(),
                    "--time_limit".to_string(),
                    timeout_secs.to_string(),
                    // "--solution_pool_dir".to_string(),
                    // pool,
                    "--log_file".to_string(),
                    log_file,
                ];

                if std::path::Path::new(&mst_file_path).exists() {
                    args.insert(0, "--total_gurobi_mst".to_string());
                    args.insert(1, mst_file_path.clone());
                }

                if std::path::Path::new(&zero_file_path).exists() {
                    args.insert(2, "--zero_node_mst".to_string());
                    args.insert(3, zero_file_path.clone());
                }

                
                println!("command: {}", args.join(" "));

                cmd.args(args)
                    .spawn()
                    .expect("Failed to start CPSAT solver")
            },
            _ => {
                panic!("Error: Unknown solver: {}", solver);
            }
        };


        println!("-----------------------------------------------------");
        let status = child.wait().expect("Failed to wait for solver process");
        println!("-----------------------------------------------------");

        let grownth_duration_solve = start_solve.elapsed();
        runtime_solve += grownth_duration_solve.as_secs_f64();

        if !status.success() {
            
            panic!("{} did not exit successfully.", solver);
        }

        if !std::path::Path::new(result_file.as_str()).exists() {
            panic!("Solver did not produce a solution file");
        }

        let sol_contents = fs::read_to_string(result_file).expect("Failed to read solution file");
        if sol_contents.trim().is_empty() {
            panic!("Solver produced an empty solution file");
        }
        let mut ilp_solution = ExtractionResult::new(IndexMap::new());
        
        // Parse the solution file
        for line in sol_contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.len() == 2 {
                let var_name = parts[0];
                if var_name.starts_with("N_") {
                    let cid = var_name[2..].split('_').next().unwrap().parse::<u32>().unwrap();
                    let nid = var_name[2..].split('_').nth(1).unwrap().parse::<u32>().unwrap();
                    let var_value_str = parts[1];
                    let val = var_value_str.parse::<f64>().expect(format!("Failed to parse solution value: {:?}", var_value_str).as_str()).round() as i32;
                    if val == 1 {
                        if !ilp_solution.choices.contains_key(&ClassId::from(cid)) {
                            ilp_solution.choose(ClassId::from(cid), NodeId::from((cid, nid)));
                        } else {
                            panic!("classid already exists");
                        }
                    }
                }
            }
        }



        // Skip solution checking if we used an empty e-graph
        println!("Solution found with solver: {}", solver);
    }
    

    // let mut extractors: indexmap::IndexMap<&str, extractor::ExtractorDetail, _> = extractor::extractors();
    // extractors.retain(|_, ed| ed.get_use_for_bench());
    // let extractor_name: String = extractor;
    // let ed = extractors
    //     .get(extractor_name.as_str())
    //     .with_context(|| format!("Unknown extractor: {extractor_name}"))
    //     .unwrap();
    
    // // let filename: String = "smoothe_artifact/dataset_new2/set/test.json".into(); // 7925
    // let filename: String = filename.into(); // 7925
    // let file_path: PathBuf = env::current_dir().unwrap().join(format!("{}", filename));

    // let mut data: Data = Data::from_json_file(&file_path)
    //     .with_context(|| format!("Failed to parse {filename}"))
    //     .unwrap();
    // // assert!(data.nodes.len() > 0 && (data.nodes.len() as u32) < u32::MAX);
    // remove_redundant_nodes(&mut data, "dag");
    // data.to_json_file("test/remove_redundant.json");
    // let mut paritioned_data = Vec::<Data>::new();

    // // let partition_num = egraph_partition(&mut data,0.125, &mut paritioned_data);


    
    

    // // let total_egraph = SerializedEGraph::from_json_file("test/remove_redundant.json")
    // // .with_context(|| format!("Failed to parse {filename}"))
    // // .unwrap();

    // let total_egraph = SerializedEGraph::from_Data(&data).with_context(|| format!("Failed to get egraph")).unwrap();
    // println!("total egraph1");
    // let mut runtime: f64 = 0.0;


    // let start = Instant::now();
    // let result = ed.get_extractor().extract(&total_egraph, &total_egraph.root_eclasses);
    // let grownth_duration = start.elapsed();
    // runtime += grownth_duration.as_secs_f64();
    // println!("total egraph2");
    // result.check(&total_egraph);
    // let activated: FxHashSet<NodeId> = result.activate_nodes(&total_egraph, &total_egraph.root_eclasses);
    // gen_gurobi_mst(&activated, &result, format!("lp/total_gurobi.mst").as_str());
    // // gen_cplex_mst(&activated, &result);
    // let tree = result.tree_cost(&total_egraph, &total_egraph.root_eclasses);
    // let dag = result.dag_cost(&total_egraph, &total_egraph.root_eclasses);
    // let depth = result.depth_cost(&total_egraph, &total_egraph.root_eclasses);
    // println!("{:<18}: runtime-{} tree:{} dag:{} depth:{}", extractor_name, runtime,tree,dag,depth);
    // collect_results(result.cost.clone(),1.25, &mut zero_node);
    // println!("zero_node: {:?}", zero_node.len());

    // let mut runtime_gurobi: f64 = 0.0;
    // let start_gurobi = Instant::now();
    // let Ok(ilp_solution_gurobi) = ilp_solver_gurobi(&total_egraph, Some(zero_node)) else {
    //     panic!("ilp_solver_gurobi failed");
    // };
    // let grownth_duration_gurobi = start_gurobi.elapsed();
    // runtime_gurobi += grownth_duration_gurobi.as_secs_f64();
    // ilp_solution_gurobi.check(&total_egraph);

    // // let mut runtime_cplex: f64 = 0.0;
    // // let start_cplex = Instant::now();
    // // let Ok(ilp_solution_cplex) = ilp_solver_cplex(&total_egraph, Some(zero_node)) else {
    // //     panic!("ilp_solver_cplex failed");
    // // };
    // // let grownth_duration_cplex = start_cplex.elapsed();
    // // runtime_cplex += grownth_duration_cplex.as_secs_f64();
    // // ilp_solution_cplex.check(&total_egraph);


    // let tree = ilp_solution_gurobi.tree_cost(&total_egraph, &total_egraph.root_eclasses);
    // let dag = ilp_solution_gurobi.dag_cost(&total_egraph, &total_egraph.root_eclasses);
    // let depth = ilp_solution_gurobi.depth_cost(&total_egraph, &total_egraph.root_eclasses);
    // println!("{:<18}: runtime-{} tree:{} dag:{} depth{}", "ilp_solver_gurobi", runtime_gurobi,tree,dag,depth);

    // // let tree = ilp_solution_cplex.tree_cost(&total_egraph, &total_egraph.root_eclasses);
    // // let dag = ilp_solution_cplex.dag_cost(&total_egraph, &total_egraph.root_eclasses);
    // // let depth = ilp_solution_cplex.depth_cost(&total_egraph, &total_egraph.root_eclasses);
    // // println!("{:<18}: runtime-{} tree:{} dag:{} depth{}", "ilp_solver_cplex", runtime_cplex,tree,dag,depth);

    

    // // let mut results = Vec::<extraction_gym::ExtractionResult>::new();
    // // let mut total_runtime:f64 = 0.0;
    // // let mut zero_node = Vec::<NodeId>::new();
    // // for idx in 0..partition_num {
    // //     let egraph = SerializedEGraph::from_Data(&paritioned_data[idx]).with_context(|| format!("Failed to get egraph")).unwrap();
    // //     let start = Instant::now();
    // //     println!("subgraph{}: start", idx);
    // //     let result = ed.get_extractor().extract(&egraph, &egraph.root_eclasses);
    // //     let grownth_duration = start.elapsed();
    // //     total_runtime += grownth_duration.as_secs_f64();
    // //     result.check(&egraph);
    // //     let tree = result.tree_cost(&egraph, &egraph.root_eclasses);
    // //     let dag = result.dag_cost(&egraph, &egraph.root_eclasses);
    // //     // println!("{:<18}: runtime-{:?} tree:{} dag:{}", extractor_name, grownth_duration, tree,dag);
    // //     println!("subgraph{}: runtime-{:?} tree:{} dag:{}", idx, grownth_duration, tree,dag);

    // //     let activated: FxHashSet<NodeId> = result.activate_nodes(&egraph, &egraph.root_eclasses);
    // //     gen_gurobi_mst(&activated, &result, format!("lp/gurobi_{}.mst", idx).as_str());
    // //     gen_cplex_mst(&activated, &result, format!("lp/cplex_{}.mst", idx).as_str());
    // //     let tree = result.tree_cost(&total_egraph, &total_egraph.root_eclasses);
    // //     let dag = result.dag_cost(&total_egraph, &total_egraph.root_eclasses);
    // //     let depth = result.depth_cost(&total_egraph, &total_egraph.root_eclasses);
    // //     println!("{:<18} {}: runtime-{} tree:{} dag:{} depth:{}", extractor_name, idx, runtime,tree,dag,depth);
    // //     collect_results(result.cost.clone(),1.25, &mut zero_node);
    // //     println!("zero_node: {:?}", zero_node.len());

    // //     let mut runtime_gurobi: f64 = 0.0;
    // //     let start_gurobi = Instant::now();
    // //     let Ok(ilp_solution_gurobi) = ilp_solver_gurobi(&total_egraph, None) else {
    // //         panic!("ilp_solver_gurobi failed");
    // //     };
    // //     let grownth_duration_gurobi = start_gurobi.elapsed();
    // //     runtime_gurobi += grownth_duration_gurobi.as_secs_f64();
    // //     ilp_solution_gurobi.check(&total_egraph);

    // //     // let mut runtime_cplex: f64 = 0.0;
    // //     // let start_cplex = Instant::now();
    // //     // let Ok(ilp_solution_cplex) = ilp_solver_cplex(&total_egraph, Some(zero_node)) else {
    // //     //     panic!("ilp_solver_cplex failed");
    // //     // };
    // //     // let grownth_duration_cplex = start_cplex.elapsed();
    // //     // runtime_cplex += grownth_duration_cplex.as_secs_f64();
    // //     // ilp_solution_cplex.check(&total_egraph);
    // //     results.push(ilp_solution_gurobi);
    // // }


    // // let mut total_choice: IndexMap<ClassId, NodeId> = IndexMap::new();
    // // for i in 0..results.len() {
    // //     for (cid, nid) in &results[i].choices {
    // //         if cid == &ClassId::from(u32::MAX) {
    // //             continue;
    // //         }
    // //         total_choice.insert(cid.clone(), nid.clone());
    // //     }
    // // }


    // // let total_results = extraction_gym::ExtractionResult::new(total_choice);
    // // total_results.check(&total_egraph);
    // // let tree = total_results.tree_cost(&total_egraph, &total_egraph.root_eclasses);
    // // let dag = total_results.dag_cost(&total_egraph, &total_egraph.root_eclasses);
    // // println!("{:<18}: runtime-{:?} tree:{} dag:{}", extractor_name, total_runtime, tree,dag);
}