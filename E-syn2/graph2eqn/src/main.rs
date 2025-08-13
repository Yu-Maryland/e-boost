use serde::Deserialize;
//use std::collections::HashMap;
use dashmap::DashMap;
use rustc_hash::FxHashMap;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

// Define Data Structures
#[derive(Deserialize, Debug)]
struct Node {
    op: String,
    children: Vec<String>,
    eclass: String, // Add this to handle the 'eclass' field in JSON
    cost: f64,      // Add this to handle the 'cost' field in JSON
}

#[derive(Deserialize, Debug)]
struct Graph {
    nodes: FxHashMap<String, Node>,
    root_eclasses: Vec<String>,
}

fn is_cyclic_graph(nodes: &FxHashMap<String, Node>) -> bool {
    let mut visited = FxHashMap::default();
    let mut rec_stack = FxHashMap::default();

    for node_id in nodes.keys() {
        if !visited.contains_key(node_id) {
            let cyclic_nodes = is_cyclic_util(nodes, node_id, &mut visited, &mut rec_stack);
            if !cyclic_nodes.is_empty() {
                // Find the cyclic nodes and print them.
                for node in cyclic_nodes {
                    println!("{}", node);
                }
                return true; // Found a cycle
            }
        }
    }

    false // No cycle found.
}

fn is_cyclic_util(
    nodes: &FxHashMap<String, Node>,
    node_id: &str,
    visited: &mut FxHashMap<String, bool>,
    rec_stack: &mut FxHashMap<String, bool>,
) -> Vec<String> {
    visited.insert(node_id.to_string(), true);
    rec_stack.insert(node_id.to_string(), true);

    let mut cyclic_nodes = Vec::new();

    if let Some(node) = nodes.get(node_id) {
        for child_id in &node.children {
            if !visited.get(child_id).unwrap_or(&false) {
                let mut child_cyclic_nodes = is_cyclic_util(nodes, child_id, visited, rec_stack);
                if !child_cyclic_nodes.is_empty() {
                    // Find the cyclic nodes and add them to the list of cyclic nodes.
                    cyclic_nodes.push(child_id.to_string());
                    cyclic_nodes.append(&mut child_cyclic_nodes);
                    return cyclic_nodes;
                }
            } else if *rec_stack.get(child_id).unwrap_or(&false) {
                // Find the cyclic nodes and add them to the list of cyclic nodes.
                cyclic_nodes.push(child_id.to_string());
                return cyclic_nodes;
            }
        }
    }

    rec_stack.remove(node_id);
    cyclic_nodes // return the cyclic nodes if any.
}

// Parse json
fn parse_json(json_str: &str) -> Graph {
    serde_json::from_str(json_str).expect("JSON was not well-formatted")
}

fn string_to_unique_id(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

// Convert DAG to eqn with proper hierarchical representation
fn dag_to_equations(
    nodes: &FxHashMap<String, Node>,
    node_id: &str,
    visited: &mut FxHashMap<String, String>,
    visit_count: &mut FxHashMap<String, usize>,
) -> String {
    *visit_count.entry(node_id.to_string()).or_insert(0) += 1;

    // if let Some(expr) = visited.get(node_id) {
    //     return expr.clone(); // Return the expression if already recorded.
    // }
    if visited.contains_key(node_id) {
        return format!("new_n_{}", node_id); // Return the modified key value if already recorded.
    }

    //println!("Node ID: {}", node_id);

    let node = nodes.get(node_id).unwrap();
    //  println!("Node ID: {:?}", node_id);
    let expression = match node.op.as_str() {
        "&" => {
            node.children
                .iter()
                .map(|child_id| dag_to_equations(nodes, child_id, visited, visit_count))
                .collect::<Vec<_>>()
                .join(" & ")
        }
        _ => {
            let operands: Vec<String> = node
                .children
                .iter()
                .map(|child_id| dag_to_equations(nodes, child_id, visited, visit_count))
                .collect();
    
            match operands.len() {
                0 => node.op.clone(), // No children means it's a variable or a constant
                1 => {
                    let operand = &operands[0];
                    if operand.len() > 50 {
                        let new_node_id = string_to_unique_id(operand);
                        visited.insert(new_node_id.to_string(), operand.clone());
                        format!("{}(new_n_{})", node.op, new_node_id)
                    } else {
                        format!("{}({})", node.op, operand)
                    }
                }
                2 => {
                    let lhs = &operands[0];
                    let rhs = &operands[1];
                    let (lhs_id, rhs_id) = (
                        if lhs.len() > 50 {
                            let id = string_to_unique_id(lhs);
                            visited.insert(id.to_string(), lhs.clone());
                            Some(id)
                        } else {
                            None
                        },
                        if rhs.len() > 50 {
                            let id = string_to_unique_id(rhs);
                            visited.insert(id.to_string(), rhs.clone());
                            Some(id)
                        } else {
                            None
                        },
                    );
    
                    format!(
                        "({} {} {})",
                        lhs_id.map_or_else(|| lhs.clone(), |id| format!("new_n_{}", id)),
                        node.op,
                        rhs_id.map_or_else(|| rhs.clone(), |id| format!("new_n_{}", id))
                    )
                }
                _ => unreachable!(),
            }
        }
    };
    

    //if expression.contains(" ") || expression.contains("(") {// record the expression if it is intermediate node.
    if visit_count[node_id] > 1 && (expression.contains(" ") || expression.contains("(")) {
        // Only record the expression if the node has been visited more than once.
        //if visit_count[node_id] > 1  {
        visited.insert(node_id.to_string(), expression.clone());
    }

    expression
}

// Function to format Synopsys for a single equation
fn format_synopsys_single(equation: &str) -> Vec<String> {
    equation
        .split('&')
        .map(|part| part.trim().to_string())
        .collect()
}

// Function to read the mapping from the reference file
fn read_prefix_mapping(file_path: &str) -> FxHashMap<String, String> {
    let file = File::open(file_path).expect("Unable to open file");
    let reader = BufReader::new(file);
    let mut mapping = FxHashMap::default();

    for line in reader.lines() {
        let line = line.expect("Unable to read line");
        if line.starts_with("OUTORDER = ") {
            let parts: Vec<&str> = line
                .trim_start_matches("OUTORDER = ")
                .trim_end_matches(';')
                .split_whitespace()
                .collect();
            for (index, part) in parts.iter().enumerate() {
                mapping.insert(format!("p[{}]", index), part.to_string());
            }
            break;
        }
    }

    mapping
}

// Function to write Synopsys format to a file with updated prefix mapping
fn write_to_file(
    variables: &Vec<String>,
    parts: Vec<String>,
    file_name: &str,
    f_prefix: &str,
    visited: FxHashMap<String, String>,
    prefix_mapping: &FxHashMap<String, String>,
) {
    let mut file = File::create(file_name).expect("Unable to create file");

    writeln!(file, "INORDER = {};", variables.join(" ")).expect("Unable to write to file");

    for (index, part) in parts.iter().enumerate() {
        let f_number = format!("{}[{}]", f_prefix, index);
        let mapped_prefix = prefix_mapping.get(&f_number).unwrap_or(&f_number);
        writeln!(file, "{} = {};", mapped_prefix, part).expect("Unable to write to file");
    }

    let outorder: Vec<String> = (0..parts.len())
        .map(|i| format!("{}[{}]", f_prefix, i))
        .map(|f_number| {
            prefix_mapping
                .get(&f_number)
                .unwrap_or(&f_number)
                .to_string()
        })
        .collect();

    writeln!(file, "OUTORDER = {};", outorder.join(" ")).expect("Unable to write to file");

    for (node_id, expr) in visited.iter() {
        writeln!(file, "new_n_{} = {};", node_id, expr).expect("Unable to write to file");
    }
}

fn main() {
    // Read the file path from the command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: <program> <path_to_input_json_file> <path_to_output_circuit_file> <check_cyclic>");
        std::process::exit(1);
    }
    let file_path = &args[1];

    let output_path = &args[2];

    // Open the file and read the JSON content
    let mut file = File::open(file_path).expect("Unable to open file");
    let mut json_str = String::new();
    file.read_to_string(&mut json_str)
        .expect("Unable to read file");

    // Parse the JSON string
    let graph = parse_json(&json_str);

    // read final args that check if the graph is cyclic or not
    // let check_cyclic = &args[2] if args.len() > 2 else "0";
    if args.len() == 4 {
        let check_cyclic = &args[3];
        if check_cyclic == "1" {
            println!("Checking for cyclic graph");
            // Check if the graph is cyclic
            if is_cyclic_graph(&graph.nodes) {
                println!("Error: The graph is cyclic.");
                return; // Exit the program or handle the error as needed
            }
        }
    }

    // Determine the nodes that are not children of any other nodes
    // Use the root_eclasses as root nodes
    let root_nodes = graph.root_eclasses;

    // Print the root nodes
    // println!("Root nodes: {:?}", root_nodes);

    // Format and write each Circuit to a file
    let prefix_mapping = read_prefix_mapping("../e-rewriter/circuit0.eqn");

    // Process each identified root node
    for (i, root) in root_nodes.iter().enumerate() {
        let mut visited = FxHashMap::default();
        let mut visit_count = FxHashMap::default();
        let equation = dag_to_equations(&graph.nodes, root, &mut visited, &mut visit_count);
        //println!("Equation: {}", equation);
        // if the length of visited is 0, not printing the visited nodes
        if visited.len() != 0 {
            //    println!("Visit count: {:?}", visit_count);
            //    println!("Visited nodes: {:?}", visited);
        }

        // for (node_id, expr) in visited.iter() {
        //     println!("{}: {}", node_id, expr);
        // }
        //println!("{}", equation);
        //println!("Visited nodes: {:?}", visited);
        //  println!("Circuit {}: ", i + 1);

        // Extract unique variables
        let mut variables = vec![];
        for node in graph.nodes.values() {
            if node.children.is_empty()
                && !variables.contains(&node.op)
                && node.op != "1"
                && node.op != "0"
            {
                variables.push(node.op.clone());
            }
        }

        // Format each equation
        let parts = format_synopsys_single(&equation);

        // Print Synopsys format for each Circuit
        // println!("INORDER = {};", variables.join(" "));
        // println!("OUTORDER = {};", (1..=parts.len()).map(|i| format!("F_{}", i)).collect::<Vec<String>>().join(" "));
        // for (i, part) in parts.iter().enumerate() {
        //     println!("F_{} = {};", i + 1, part);
        // }
        // println!();

        write_to_file(
            &variables,
            parts,
            //&format!("circuit{}.eqn", i),
            // use output_path instead of hardcoded circuit{}.eqn
            &format!("{}", output_path),
            "p",
            visited,
            &prefix_mapping,
        );
        //post_process_eqn(&format!("circuit{}.eqn", i));
        println!("Finished graph to equation conversion")
    }
}
