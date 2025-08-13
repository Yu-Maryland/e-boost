import json

def transform_solution_to_json(sol_file, original_json_file, output_file):
    """
    Convert Gurobi solution file to extracted egraph JSON format.
    
    Args:
        sol_file: Path to the .sol file from Gurobi
        original_json_file: Path to the original rewritten egraph JSON file (processed by rewrite_json.py)
        output_file: Path to output the extracted egraph JSON file
    """
    # Read the original saturated egraph to get node information
    with open(original_json_file, 'r') as f:
        original_data = json.load(f)
    
    # Read the solution file and extract selected nodes
    selected_nodes = {}
    
    with open(sol_file, 'r') as f:
        for line in f:
            line = line.strip()
            if line.startswith('#') or not line:
                continue
            
            parts = line.split()
            if len(parts) == 2:
                var_name, value = parts[0], int(parts[1])
                if value == 1 and var_name.startswith('N_'):
                    # Parse variable name: N_eclass_node -> eclass, node
                    parts = var_name.split('_')
                    if len(parts) >= 3:
                        eclass_id = parts[1]
                        node_id = parts[2]
                        node_key = f"{eclass_id}.{node_id}"
                        
                        # Check if this node exists in the original data
                        if node_key in original_data['nodes']:
                            selected_nodes[eclass_id] = node_key
    
    # Create the output JSON with choices mapping
    output_data = {
        "choices": selected_nodes
    }
    
    # Write the output file
    with open(output_file, 'w') as f:
        json.dump(output_data, f, indent=4)
    
    print(f"Converted {len(selected_nodes)} choices from solution file to {output_file}")


# Specify the input and output file names
sol_file = 'file/result/rewritten_egraph_with_weight_cost_serd2_1_gurobi.sol'
original_json_file = 'E-syn2/extraction-gym/input/rewritten_egraph_with_weight_cost_serd.json'  # The original saturated egraph (created by rewrite_json.py)
output_file = 'E-syn2/extraction-gym/input/rewritten_egraph_with_weight_cost_serd2.json'        # The extracted egraph with choices

# Call the transformation function
transform_solution_to_json(sol_file, original_json_file, output_file)