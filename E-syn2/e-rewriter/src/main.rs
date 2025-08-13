use egg::*;
use egraph_serialize::EGraph as SerializedEGraph;
use rand::random;
use rayon::iter::ParallelDrainRange;
use serde::Serialize;
use serde_json::json;
use std::f32::consts::E;
use std::fs;
use std::io;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use rayon::prelude::*;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::time::Instant;
mod utils;
use crate::utils::cost::*;
use crate::utils::random_gen;
use crate::utils::runner_modified;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use utils::{extract_new::*, language::*, preprocess::*};

use log::LevelFilter;

// fn print_usage(program_name: &str) {
//     println!(
//         "Usage: {} <input_file_path> <runner_iteration_limit> <extract_pattern>",
//         program_name
//     );
// }

fn save_egraph_to_json(egraph: &EGraph<Prop, ()>, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let json_rep = serde_json::to_string_pretty(&egraph).unwrap();
    fs::write(&file_path, json_rep)?;
    Ok(())
}

fn save_serialized_egraph_to_json(serialized_egraph: &SerializedEGraph, file_path: &PathBuf, root_ids: &[usize]) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(&file_path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &serialized_egraph)?;

    let root_eclasses_value: serde_json::Value = root_ids
        .iter()
        .map(|id| serde_json::Value::String(id.to_string()))
        .collect();

    let json_string = std::fs::read_to_string(&file_path)?;
    let mut json_data: serde_json::Value = serde_json::from_str(&json_string)?;
    json_data["root_eclasses"] = root_eclasses_value;

    let file = File::create(&file_path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &json_data)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let input_path = &args[1];

    // Set up timer to measure time for eqn2egraph
    let start = Instant::now();

    // Preprocess input file
    preprocess_file_concat(&input_path)?;
    preprocess_file_order(&input_path)?;
    println!("Finished preprocessing input file");

    // Transfer eqn file into egraph format in egg
    let (root_id, input_vec_id, input_vec_symbol) = process_file(input_path);

    println!("root: {:?}", root_id);

    let duration_eqn2egraph_initial = start.elapsed();
    println!("eqn2egraph initially finished in: {:?}.", duration_eqn2egraph_initial);

    
    let mut root_ids: Vec<usize> = vec![root_id.into()];
    
    let json_file = format!("{}.json", input_path);

    // format with base name+json
    // let path = Path::new(input_path);
    // let Some(stem) = path.file_stem() else {
    //     panic!("Invalid input file path")
    // };
    // let json_file =  format!("{}.json", stem.to_string_lossy());
    // println!("json_file: {}", json_file);

    // Transfer egg::egraph symbol language's json into my defined language's json
    let modified_json_file = process_json_prop(&json_file);
    //let modified_json_file = process_json_prop_prallel(&json_file);
    let duration_eqn2egraph_handling_base_language = start.elapsed();
    let converted_json_data = fs::read_to_string(&modified_json_file).expect("Unable to read the JSON file");
    let mut input_egraph: egg::EGraph<Prop, ()> = serde_json::from_str(&converted_json_data).unwrap();
    input_egraph.rebuild();//eqn2egraph finished

    // print the time taken for eqn2egraph
    let eqn2egraph_all_duration = start.elapsed();
    println!("eqn2egraph with handling of base language finished in: {:?}.", duration_eqn2egraph_handling_base_language - duration_eqn2egraph_initial);
    println!("eqn2egraph finished in: {:?}.", eqn2egraph_all_duration);

    // Save input_egraph into json file
    let input_egraph_json_path = env::current_dir().unwrap().join("rewritten_circuit/eqn2egraph.json"); 
    save_egraph_to_json(&input_egraph, &input_egraph_json_path)?;

    // Read from json file and print info
    // let json_contents = fs::read_to_string(&input_egraph_json_path).expect("Failed to read JSON file");
    // let mut converted_egg: egg::EGraph<Prop, ()> = serde_json::from_str(&json_contents).unwrap();
    // converted_egg.rebuild();

    let converted_egg = input_egraph.clone();

    println!("total");
    println!("input node: {}", converted_egg.total_size());
    println!("input class: {}", converted_egg.number_of_classes());

    // Transfer egg::egraph to serialized_egraph and save it into json file
    let serialized_input_egraph = egg_to_serialized_egraph(&converted_egg);
    let serialized_input_egraph_json_path = env::current_dir().unwrap().join("rewritten_circuit/egraph2egraph_serd.json"); // egraph to serialized_egraph finished
    save_serialized_egraph_to_json(&serialized_input_egraph, &serialized_input_egraph_json_path, &root_ids)?;


    // Rewrite time!
    {
        let runner_iteration_limit = env::args()
            .nth(2)
            .unwrap_or("10".to_string())
            .parse()
            .unwrap_or(20);
        let egraph_node_limit = 200000000;
        let start = Instant::now();
        let mut runner = Runner::default()
            .with_explanations_enabled()
            .with_egraph(converted_egg.clone())
            .with_time_limit(std::time::Duration::from_secs(10))
            .with_iter_limit(runner_iteration_limit)
            .with_node_limit(egraph_node_limit);

        runner.roots = root_ids.iter().cloned().map(Id::from).collect();
        let runner_result = runner.run(&make_rules());

        let duration = start.elapsed();
        println!(
            "Runner stopped: {:?}. Time taken for runner: {:?}, Classes: {}, Nodes: {}, Size: {} \n\n",
            runner_result.stop_reason,
            duration,
            runner_result.egraph.number_of_classes(),
            runner_result.egraph.total_number_of_nodes(),
            runner_result.egraph.total_size()
        );
        println!("root{:?}", runner_result.roots);
        runner_result.print_report();
        let root = runner_result.roots[0];

        // Save output egraph from runner (input for extraction gym)
        let output_egraph_json_path = env::current_dir().unwrap().join("rewritten_circuit/rewritten_egraph_internal.json");
        save_egraph_to_json(&runner_result.egraph, &output_egraph_json_path)?;

        println!("egraph after runner");
        println!("egraph node: {}", runner_result.egraph.total_size());
        println!("egraph class: {}", runner_result.egraph.number_of_classes());

        // Save serialized output egraph to json with root nodes
        let serialized_output_egraph = egg_to_serialized_egraph(&runner_result.egraph);
        let serialized_output_egraph_json_path = env::current_dir().unwrap().join("rewritten_circuit/rewritten_egraph_internal_serd.json");
        save_serialized_egraph_to_json(&serialized_output_egraph, &serialized_output_egraph_json_path, &root_ids)?;

        println!("------------------assign cost of enode-----------------");
        let json_string = serde_json::to_string(&serialized_output_egraph).unwrap();
        let cost_string = process_json_prop_cost(&json_string);

        let output_egraph_cost_json_path = env::current_dir().unwrap().join("rewritten_circuit/rewritten_egraph_with_weight_cost_serd.json");
        let mut json_data: serde_json::Value = serde_json::from_str(&cost_string)?;
        json_data["root_eclasses"] = serde_json::Value::Array(root_ids.iter().map(|id| serde_json::Value::String(id.to_string())).collect());
        let file = File::create(&output_egraph_cost_json_path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &json_data)?;

        println!("done");
    }

    Ok(())
}