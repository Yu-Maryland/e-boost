use ::serde::{Deserialize, Serialize};
use egg::*;
use regex::Regex;
use serde::__private::fmt::Display;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::fs::{OpenOptions};
use std::io::prelude::*;
use std::io;
use std::io::{BufWriter, Write};
use rayon::prelude::*;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Node {
    op: String,
    children: Vec<u32>,
}

//sub function for process_json_prop
pub fn process_data(data: &mut serde_json::Value) {
    if let Some(entries) = data.as_array_mut() {
        for entry in entries.iter_mut() {
            if let Some(entry_obj) = entry.as_object_mut() {
                process_entry(entry_obj);
            }
        }
    } else if let Some(obj) = data.as_object_mut() {
        process_entry(obj);
        for (_, value) in obj.iter_mut() {
            process_data(value);
        }
    }
}

//sub function for process_json_prop
pub fn process_entry(entry_obj: &mut serde_json::Map<String, serde_json::Value>) {
    // handle "op" and "children" keys,values pair
    if let Some(op_value) = entry_obj.remove("op") {
        if let Some(children_value) = entry_obj.remove("children") {
            entry_obj.insert(op_value.as_str().unwrap().to_owned(), children_value);
        }
    }

    for (key, value) in entry_obj.clone().into_iter() {
        if let serde_json::Value::Array(ref arr) = value {
            if arr.is_empty() {
                entry_obj.remove(&key);
                entry_obj.insert("Symbol".to_owned(), serde_json::Value::String(key.clone()));
            }
        }
        if key == "Not" {
            if let serde_json::Value::Array(ref arr1) = value {
                if arr1.len() == 1 {
                    let new_value = arr1[0].clone();
                    entry_obj.insert(key, new_value);
                }
            }
        }
    }
}

//transfer egg's symbol language version json format into your defined language version json format
//The reason is that u must use egg's symbol language 's trait add to add node into graph ,which the self-defined language by define_language! macro definition dosesn't support add node
//
pub fn process_json_prop(json_file: &str) -> String {
    let json_str = fs::read_to_string(json_file).expect("Failed to read JSON file");
    let mut data: Value = serde_json::from_str(&json_str).unwrap();

    // handle "memo"
    if let Some(classes) = data
        .get_mut("classes")
        .and_then(|classes| classes.as_object_mut())
    {
        for class in classes.values_mut() {
            if let Some(nodes) = class
                .get_mut("nodes")
                .and_then(|nodes| nodes.as_array_mut())
            {
                for node in nodes.iter_mut() {
                    //    println!("Processed node: {:?}", node);
                    process_data(node);
                }
            }
            if let Some(parents) = class
                .get_mut("parents")
                .and_then(|parents| parents.as_array_mut())
            {
                for parent in parents.iter_mut() {
                    //    println!("Processed parent: {:?}", parent);
                    process_data(parent);
                }
            }
        }
    }

    if let Some(memo) = data.get_mut("memo").and_then(|memo| memo.as_array_mut()) {
        for entry in memo.iter_mut() {
            process_data(entry);
        }
    }

    // converted the modified data into a json string
    let modified_json_str = serde_json::to_string_pretty(&data).unwrap();
    // make the modified json file name
    let json_file_path = PathBuf::from(json_file);
    let modified_json_file = json_file_path.with_file_name(format!(
        "modified_{}",
        json_file_path.file_name().unwrap().to_str().unwrap()
    ));
    // write the modified json file
    fs::write(&modified_json_file, modified_json_str).expect("Failed to write modified JSON file");
    modified_json_file.to_str().unwrap().to_owned()
}

// egraph_serialize::EGraph used in extraction gym
// you need to transfer egg's EGraph to  Egraph_serialize's EGraph for extraction gym input
pub fn egg_to_serialized_egraph<L, A>(egraph: &egg::EGraph<L, A>) -> egraph_serialize::EGraph
where
    L: Language + Display,
    A: Analysis<L>,
{
    use egraph_serialize::*;
    let mut out = EGraph::default();
    for class in egraph.classes() {
        for (i, node) in class.nodes.iter().enumerate() {
            out.add_node(
                format!("{}.{}", class.id, i),
                Node {
                    op: node.to_string(),
                    children: node
                        .children()
                        .iter()
                        .map(|id| NodeId::from(format!("{}.0", id)))
                        .collect(),
                    eclass: ClassId::from(format!("{}", class.id)),
                    cost: Cost::new(1.0).unwrap(),
                },
            )
        }
    }
    out
}

pub fn process_json_prop_cost(json_str: &str) -> String {
    let mut data: Value = serde_json::from_str(&json_str).unwrap();

    if let Some(nodes) = data.get_mut("nodes").and_then(|nodes| nodes.as_object_mut()) {
        for node in nodes.values_mut() {
            let op = node["op"].as_str().unwrap();
            let cost = node["cost"].as_f64().unwrap();

            let new_cost = match op {
                "+" => 1.0,
                "!" => 0.0,
                "*" => 1.0,
                _ => 0.0,
            };

            node["cost"] = serde_json::to_value(new_cost).unwrap();
        }
    }

    serde_json::to_string_pretty(&data).unwrap()
}

pub fn process_file(file_name: &str) -> (egg::Id, Vec<Id>, i32) {
    let file = File::open(file_name).expect("Unable to open the eqn file");
    let reader = BufReader::new(file);
    let mut egraph: egg::EGraph<SymbolLang, ()> = EGraph::default();
    let mut vars = HashMap::new();
    let mut out = HashMap::new();
    let mut count_out = 0;
    let mut id2concat = Vec::new();
    let mut input_id: Vec<Id> = Vec::new();
    let mut one_out_sig = 0;
    fn string_to_unique_id(s: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }

    let id0 = egraph.add(SymbolLang::leaf("0"));
    vars.insert("0".to_string(), id0);
    let id1 = egraph.add(SymbolLang::leaf("1"));
    vars.insert("1".to_string(), id1);
    for line in reader.lines() {
        let line = line.expect("Unable to read line");
        let line = line.trim().trim_end_matches(';');
        //print!("line:  {}\n",line);
        if line.starts_with('#') || line.is_empty() {
            continue;
        } else if line.starts_with("INORDER") {
            let inputs = line.trim_start_matches("INORDER = ").split_whitespace();
            for input in inputs {
                let id = egraph.add(SymbolLang::leaf(input)); // Replace "NOT" with each input string
                vars.insert(input.to_string(), id);
                input_id.push(id);
            }
        } else if line.starts_with("OUTORDER") {
            let output = line.trim_start_matches("OUTORDER = ").trim();
            for output in output.split_whitespace() {
                let id_u64 = string_to_unique_id(output);
                out.insert(output.to_string(), id_u64);
            }
        } else {
            let parts: Vec<&str> = line.split('=').map(str::trim).collect();
            let left = parts[0];
            let right = parts[1];
            // print!("right {}\n",right);
            let id = if right.contains('+') {
                let operands: Vec<&str> = right.split('+').map(str::trim).collect();
                let lhs = if operands[0].starts_with('!') {
                    let var = &operands[0][1..];
                    let id = vars[var];
                    egraph.add(SymbolLang::new("Not", vec![id]))
                } else {
                    vars[operands[0]]
                };
                let rhs = if operands[1].starts_with('!') {
                    let var = &operands[1][1..];
                    let id = vars[var];
                    egraph.add(SymbolLang::new("Not", vec![id]))
                } else {
                    vars[operands[1]]
                };
                egraph.add(SymbolLang::new("Or", vec![lhs, rhs]))
            } else if right.contains('*') {
                let operands: Vec<&str> = right.split('*').map(str::trim).collect();
                let lhs = if operands[0].starts_with('!') {
                    let var = &operands[0][1..];
                    let id = vars[var];
                    egraph.add(SymbolLang::new("Not", vec![id]))
                } else {
                    vars[operands[0]]
                };
                let rhs = if operands[1].starts_with('!') {
                    let var = &operands[1][1..];
                    let id = vars[var];
                    egraph.add(SymbolLang::new("Not", vec![id]))
                } else {
                    vars[operands[1]]
                };

                egraph.add(SymbolLang::new("And", vec![lhs, rhs]))
            } else if right.starts_with('!') {
                let var = &right[1..];
                let id = vars[var];
                egraph.add(SymbolLang::new("Not", vec![id]))
            } else {
                vars[right]
            };

            if out.contains_key(left) {
                id2concat.push(id);
                count_out += 1;
            }
            vars.insert(left.to_string(), id);
        }
    }

    let mut concat = Vec::new();

    for i in 0..count_out - 1 {
        if i == 0 {
            let id = egraph.add(SymbolLang::new(
                "Concat",
                vec![id2concat[i as usize], id2concat[(i + 1) as usize]],
            ));
            concat.push(id);
        } else {
            let id = egraph.add(SymbolLang::new(
                "Concat",
                vec![concat[(i - 1) as usize], id2concat[(i + 1) as usize]],
            ));
            concat.push(id);
        }
    }
    let last_element: Id = if let Some(element) = concat.pop() {
        element
    } else {
        one_out_sig = 1;
        let mut id: Id = id2concat.pop().unwrap().into();
        vars.insert(out.keys().next().unwrap().to_string(), id.into());
        id
    };
    egraph.rebuild();
    let json_str = serde_json::to_string_pretty(&egraph).unwrap();

    let output_dir = Path::new(file_name).parent().unwrap_or(Path::new(""));
    let output_file = format!(
        "{}.json",
        PathBuf::from(file_name)
            .file_name()
            .unwrap()
            .to_string_lossy()
    );
    let output_path = output_dir.join(output_file);
    fs::write(output_path, json_str).expect("Failed to write JSON file");
    (last_element, input_id, one_out_sig)
}



pub fn preprocess_file_concat(file_name: &str) -> Result<(), io::Error> {
    // Open the file for reading
    let file = File::open(file_name)?;
    let reader = BufReader::new(file);

    // Prepare a string to hold the new contents of the file
    let mut new_contents = String::new();

    // Flags to detect INORDER and OUTORDER sections
    let mut in_inorder_section = false;
    let mut in_outorder_section = false;

    for line in reader.lines() {
        let line = line?;

        // Check if we're entering the INORDER section
        if line.trim().starts_with("INORDER") {
            in_inorder_section = true;
            if line.trim().ends_with(";") {
                // Skip this function if INORDER is one line and ends with ;
                in_inorder_section = false;
                new_contents.push_str(&line);
                new_contents.push('\n');
                continue;
            } else {
                new_contents.push_str(&line.trim());
                new_contents.push(' ');
            }
        // Check if we're entering the OUTORDER section
        } else if line.trim().starts_with("OUTORDER") {
            in_outorder_section = true;
            if line.trim().ends_with(";") {
                // Skip this function if OUTORDER is one line and ends with ;
                in_outorder_section = false;
                new_contents.push_str(&line);
                new_contents.push('\n');
                continue;
            } else {
                new_contents.push_str(&line.trim());
                new_contents.push(' ');
            }
        } else if in_inorder_section || in_outorder_section {
            // Continue appending lines that are part of INORDER or OUTORDER sections
            new_contents.push_str(&line.trim());
            if line.trim().ends_with(";") {
                // End of section, reset flags
                in_inorder_section = false;
                in_outorder_section = false;
                new_contents.push('\n');
            } else {
                new_contents.push(' ');
            }
        } else {
            // Append lines that are not part of INORDER or OUTORDER sections
            new_contents.push_str(&line);
            new_contents.push('\n');
        }
    }

    // Open the same file for writing
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(file_name)?;

    // Write the new contents to the file
    file.write_all(new_contents.as_bytes())?;

    Ok(())
}

pub fn preprocess_file_order(file_name: &str) -> Result<(), io::Error> {
    // Open the file for reading
    let file = File::open(file_name)?;
    let reader = BufReader::new(file);

    // Search for the first line starting with "new_"
    let mut variables = Vec::new();
    let mut found_new = false;

    for line in reader.lines() {
        let line = line?;
        if found_new && !line.starts_with("new_") {
            let variable = line.split('=').next().unwrap().trim().to_string();
            variables.push(variable);
        } else if line.starts_with("new_") {
            found_new = true;
        }
    }

    // Generate new OUTORDER line
    let outorder = variables.join(" ");
    let new_outorder = format!("OUTORDER = {};", outorder.trim_end());

    // Create a temporary file
    let temp_file_name = format!("{}_temp", file_name);
    let temp_file = File::create(&temp_file_name)?;
    let mut writer = BufWriter::new(temp_file);

    // Write modified content to the temporary file
    let input_file = File::open(file_name)?;
    let reader = BufReader::new(input_file);

    for line in reader.lines() {
        let line = line?;
        if line.starts_with("OUTORDER") {
            writeln!(writer, "{}", new_outorder)?;
        } else {
            writeln!(writer, "{}", line)?;
        }
    }

    // Flush and close the writer
    writer.flush()?;
    drop(writer);

    // Replace the original file with the temporary file
    fs::rename(&temp_file_name, file_name)?;

    Ok(())
}

// -----------------------------Unused Functions----------------------------------

pub fn process_json_prop_prallel(json_file: &str) -> String {
    let json_str = fs::read_to_string(json_file).expect("Failed to read JSON file");
    let mut data: Value = serde_json::from_str(&json_str).unwrap();

    // Process "memo"
    if let Some(classes) = data.get_mut("classes").and_then(Value::as_object_mut) {
        classes.iter_mut().for_each(|(_, class)| {
            if let Some(nodes) = class.get_mut("nodes").and_then(Value::as_array_mut) {
                nodes.par_iter_mut().for_each(process_data);
            }
            if let Some(parents) = class.get_mut("parents").and_then(Value::as_array_mut) {
                parents.par_iter_mut().for_each(process_data);
            }
        });
    }

    if let Some(memo) = data.get_mut("memo").and_then(Value::as_array_mut) {
        memo.par_iter_mut().for_each(process_data);
    }

    // converted the modified data into a json string
    let modified_json_str = serde_json::to_string(&data).unwrap();
    let json_file_path = PathBuf::from(json_file);
    let modified_json_file = json_file_path.with_file_name(format!(
        "modified_{}",
        json_file_path.file_name().unwrap().to_str().unwrap()
    ));
    fs::write(&modified_json_file, modified_json_str).expect("Failed to write modified JSON file");
    modified_json_file.to_str().unwrap().to_owned()
}