use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error as StdError;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::Error;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

// Struct representing the graph data
// Contains a HashMap of nodes with string keys and Node values
#[derive(Debug, Deserialize, Serialize)]
struct GraphData {
    nodes: HashMap<String, Node>,
}

// Struct representing a node in the graph
// Contains fields for the operation (op), children nodes, equivalence class (eclass), and cost
#[derive(Debug, Deserialize, Serialize)]
struct Node {
    op: String,
    children: Vec<String>,
    eclass: String,
    cost: f32,
}

// Function to process a JSON file with choices
// Input:
//   - input_file: Path to the input JSON file
//   - output_file: Path to the output file
//   - a: An additional parameter (not used in this function)
// Output:
//   - Result<(), Box<dyn StdError>>: Returns Ok(()) if successful, or an error if encountered
fn process_json_with_choices(
    input_file_extracted_result: &str,
    input_file_saturated_graph: &Path,
    output_file: &str,
    a: u32,
) -> Result<(), Box<dyn StdError>> {
    println!("Processing file with choices...");
    // Read the JSON content from the input file
    let json_content = fs::read_to_string(input_file_extracted_result)?;
    let data: Value = serde_json::from_str(&json_content)?;

    // Extract the choices from the JSON data
    let choices: HashMap<String, String> = serde_json::from_value(data["choices"].clone())?;
    let values: HashSet<&str> = choices.values().map(|v| v.as_str()).collect();

    // Read the saturated graph data from the input file
    let graph_content = fs::read_to_string(input_file_saturated_graph)?;
    let graph_data: GraphData = serde_json::from_str(&graph_content)?;

    // Filter the nodes based on the choices
    let new_nodes: HashMap<String, Node> = graph_data
        .nodes
        .into_iter()
        .filter(|(key, _)| values.contains(key.as_str()))
        .collect();

    // Create the result JSON object
    let result = serde_json::json!({
        "nodes": new_nodes,
    });

    let output_content = serde_json::to_string_pretty(&result)?;

    // Write the result to the output file
    fs::write(output_file, output_content)?;

    Ok(())
}

// Function to process a JSON file and simplify the keys
// Input:
//   - input_file: Path to the input JSON file
//   - output_file: Path to the output file
//   - a: An additional parameter (not used in this function)
// Output:
//   - Result<(), Box<dyn StdError>>: Returns Ok(()) if successful, or an error if encountered
fn process_json_simplify_keys(
    input_file: &str,
    output_file: &str,
    a: u32,
) -> Result<(), Box<dyn StdError>> {
    println!("Processing file to simplify keys...");
    // Read the JSON content from the input file
    let json_content = fs::read_to_string(input_file)?;
    let data: GraphData = serde_json::from_str(&json_content)?;

    // Create a new HashMap to store the simplified nodes
    let mut new_nodes: HashMap<String, Node> = HashMap::new();

    // Iterate over the nodes and simplify the keys
    for (key, mut value) in data.nodes {
        let new_key = key.split('.').next().unwrap().to_string();
        value.children = value
            .children
            .iter()
            .map(|child| child.split('.').next().unwrap().to_string())
            .collect();
        new_nodes.insert(new_key, value);
    }

    // Create the result GraphData with the simplified nodes
    let result = GraphData { nodes: new_nodes };

    let output_content = serde_json::to_string_pretty(&result)?;
    let mut output_file = fs::File::create(output_file)?;

    // Write the result to the output file
    output_file.write_all(output_content.as_bytes())?;

    Ok(())
}

// Function to update root eclasses in the output file
// Input:
//   - graph_file: Path to the graph file containing the root eclasses
//   - output_file: Path to the output file to update
// Output:
//   - None
fn update_root_eclasses(graph_file: &Path, output_file: &Path) {
    // Read the graph data from the graph file
    let mut source_data = String::new();

    File::open(graph_file)
        .unwrap()
        .read_to_string(&mut source_data)
        .unwrap();
    let source_data: Value = serde_json::from_str(&source_data).unwrap();

    // Extract the root eclasses from the graph data
    let root_eclasses = if let Some(root_eclasses) = source_data["root_eclasses"].as_array() {
        root_eclasses.to_owned()
    } else {
        vec![]
    };

    // Read the target file data
    let mut target_data = String::new();
    File::open(&output_file)
        .unwrap()
        .read_to_string(&mut target_data)
        .unwrap();
    let mut target_data: Value = serde_json::from_str(&target_data).unwrap();

    // Update the root eclasses in the target data
    target_data["root_eclasses"] = json!(root_eclasses);

    // Write the updated data back to the target file
    File::create(&output_file)
        .unwrap()
        .write_all(
            serde_json::to_string_pretty(&target_data)
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
}

#[derive(Parser, Debug)]
#[command(name = "process_json")]
#[command(version)]
#[command(about = "Process extracted graphs based on user input", long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        value_name = "FILE",
        help = "Sets the input saturated graph file",
        required = true,
    )]
    saturated_graph_file: String,
    #[arg(
        short,
        long,
        value_name = "FILE",
        help = "Sets the saturated graph extraction result file path",
        required = true,
    )]
    extraction_result_file: String,
    #[arg(
        short,
        long,
        value_name = "FILE",
        help = "Sets the output file path",
        required = true,
    )]
    output_file: String,
    #[arg(
        short,
        long,
        help = "Extracts the graph based on DAG or tree-based extraction",
    )]
    graph_extract_type_extract_dag: bool,
}

fn main() {
    let args = Args::parse();

    let input_saturacted_graph_file = Path::new(&args.saturated_graph_file);
    let input_extraction_result_file = &args.extraction_result_file;
    let output_file = &args.output_file;
    let dag_based = args.graph_extract_type_extract_dag;

    // print whether it is DAG-based or not
    println!("DAG-based: {}", dag_based);

    if dag_based {
        println!("Processing DAG-based extracted graph...");
        // Process file with choices
        if let Err(err) = process_json_with_choices(input_extraction_result_file, &input_saturacted_graph_file, output_file, 0) {
            eprintln!(
                "Error processing file with choices {}: {}",
                input_extraction_result_file, err
            );
        }

        // Process file to simplify keys
        if let Err(err) = process_json_simplify_keys(output_file, output_file, 0) {
            eprintln!(
                "Error processing file to simplify keys {}: {}",
                output_file, err
            );
        }

        // Update root eclasses in the output file for dag_based
        update_root_eclasses(&input_saturacted_graph_file, &Path::new(output_file));
    } else {
        // assert fail
        println!("Processing tree-based extracted graph...");
        assert!(false, "Tree-based extraction is not supported yet");
    }
}