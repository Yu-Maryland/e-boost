import json
import os

def is_int(s: str) -> bool:
    """Check if a string can be converted to an integer."""
    try:
        int(s)
        return True
    except ValueError:
        return False

def read_json_file(filename):
    """Read JSON file with UTF-8 encoding to prevent Chinese character corruption."""
    try:
        # Open JSON file with UTF-8 encoding to prevent character corruption
        with open(filename, 'r', encoding='utf-8') as f:
            # Use json.load() method to read JSON data
            data = json.load(f)
        return data
    except Exception as e:
        print("Error reading JSON file:", e)
        return None

if __name__ == "__main__":
    # Use relative paths from the extraction-tool root directory
    old_filename = "E-syn2/extraction-gym/input/rewritten_egraph_with_weight_cost_serd.json"
    new_filename = "E-syn2/extraction-gym/input/rewritten_egraph_with_weight_cost_serd2.json"
    
    # Remove existing output file if it exists
    if os.path.exists(new_filename):
        os.remove(new_filename)
    
    # Read the original JSON data
    json_data = read_json_file(old_filename)
    new_json_data = json_data
    
    # Process each node in the graph
    for i in new_json_data['nodes']:
        # Convert children from string format to integer format
        child = []
        for j in json_data['nodes'][i]['children']:
            assert "." in j
            cid = j.split(".")
            assert len(cid) == 2
            ccid = cid[0]
            cnid = cid[1]
            assert is_int(ccid) and is_int(cnid)
            cnum_cid = int(ccid)
            child.append(cnum_cid)
        
        # Update children and eclass fields
        new_json_data['nodes'][i]['children'] = child
        new_json_data['nodes'][i]['eclass'] = int(json_data['nodes'][i]['eclass'])
        
        # Parse and validate node ID format
        assert "." in i
        id_parts = i.split(".")
        assert len(id_parts) == 2
        cid = id_parts[0]
        nid = id_parts[1]
        assert is_int(cid) and is_int(nid)
        num_cid = int(cid)
        num_nid = int(nid)
        new_json_data['nodes'][i]['id'] = str(num_cid) + "." + str(num_nid)
    
    # Convert root_eclasses to integers
    for i in new_json_data['root_eclasses']:
        assert is_int(i)
        new_json_data['root_eclasses'][new_json_data['root_eclasses'].index(i)] = int(i)
    
    # Write the processed data to the new file
    with open(new_filename, "w", encoding="utf-8") as file:
        print(f"Writing processed data to: {new_filename}")
        json.dump(new_json_data, file, indent=4)