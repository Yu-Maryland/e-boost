import os
import json

def process_json(input_file, a):
    # Read input file
    with open(input_file, 'r') as f:
        data = json.load(f)

    choices = data['choices']
    values = list(choices.values())

    # Read graph_internal_serd.json file
    input_dir = os.path.join(os.getcwd(), 'data', 'my_data')
    print(input_dir)
    files = os.listdir(input_dir)
    json_files = [file for file in files if file.endswith('.json')]
    graph_file = os.path.join(input_dir, json_files[0])
    with open(graph_file, 'r') as f:
         graph_data = json.load(f)

    # Build new result dictionary, only keeping keys that exist in values list
    new_nodes = {key: value for key, value in graph_data['nodes'].items() if key in values}

    # Build final result dictionary
    result = {'nodes': new_nodes}
   
    # Get the basename of input file
    file_name = os.path.basename(input_file)

    # Build output file path
    if(a==1):
      output_file = os.path.join('out_process_result1', file_name)
    else:
      output_file = os.path.join('out_process_dag_result1', file_name)
        
    # Output result
    with open(output_file, 'w') as f:
        json.dump(result, f, indent=2)

    #print(f'Processing completed, result saved to file: {output_file}')

def process_json1(input_file,a):
    # Read input file
    with open(input_file, 'r') as f:
        data = json.load(f)

    # Process decimal points and decimal parts in keys
    new_nodes = {}
    for key, value in data['nodes'].items():
        new_key = key.split('.')[0]  # Remove decimal points and decimal parts
        value['children'] = [child.split('.')[0] for child in value['children']]  # Process numbers in "children"
        new_nodes[new_key] = value

    # Build final result dictionary
    result = {'nodes': new_nodes}

    # Get the basename of input file
    file_name = os.path.basename(input_file)

    # Build output file path
    if(a==1):
       output_dir = 'out_process_result1'
    else :
       output_dir = 'out_process_dag_result1'
    os.makedirs(output_dir, exist_ok=True)
    output_file = os.path.join(output_dir, file_name)

    # Output result
    with open(output_file, 'w') as f:
        json.dump(result, f, indent=2)

    #print(f'Processing completed, result saved to file: {output_file}')

# Iterate through all JSON files in the directory
output_dir = 'out_process_result1'
os.makedirs(output_dir, exist_ok=True)
input_dir = os.path.join(os.getcwd(), 'out_json', 'my_data')
#input_dir = '/data/cchen/extraction-gym-new/extraction-gym/out_json/'
files = [file for file in os.listdir(input_dir)]

for file in files:
    input_file = os.path.join(input_dir, file)

    if os.path.isfile(input_file):
        #print(f'Processing file: {input_file}')
        process_json(input_file,1)
    else:
        print(f'File does not exist: {input_file}')

output_dir = 'out_process_dag_result1'
os.makedirs(output_dir, exist_ok=True)
#input_dir = '/data/cchen/extraction-gym-new/extraction-gym/out_dag_json/'
input_dir = os.path.join(os.getcwd(), 'out_dag_json', 'my_data')
files = [file for file in os.listdir(input_dir)]

for file in files:
    input_file = os.path.join(input_dir, file)

    if os.path.isfile(input_file):
       # print(f'Processing file: {input_file}')
        process_json(input_file,0)
    else:
        print(f'File does not exist: {input_file}')






# Iterate through all files in the directory
input_dir = os.path.join(os.getcwd(), 'out_process_result1')
#input_dir = '/data/cchen/extraction-gym-new/extraction-gym/out_process_result/'
files = [file for file in os.listdir(input_dir)]

for file in files:
    input_file = os.path.join(input_dir, file)

    if os.path.isfile(input_file):
      #  print(f'Processing file: {input_file}')
        process_json1(input_file,1)
    else:
        print(f'File does not exist: {input_file}')

input_dir = os.path.join(os.getcwd(), 'out_process_dag_result1') 
#input_dir = '/data/cchen/extraction-gym-new/extraction-gym/out_process_dag_result/'
files = [file for file in os.listdir(input_dir)]

for file in files:
    input_file = os.path.join(input_dir, file)

    if os.path.isfile(input_file):
     #   print(f'Processing file: {input_file}')
        process_json1(input_file,0)
    else:
        print(f'File does not exist: {input_file}')


input_dir = os.path.join(os.getcwd(), 'out_process_dag_result1')
# input_dir = '/data/cchen/extraction-gym-new/extraction-gym/out_process_dag_result/'

files = os.listdir(input_dir)

for file in files:
    input_file = os.path.join(input_dir, file)

    if os.path.isfile(input_file):
        filename, extension = os.path.splitext(input_file)
        
        if extension != '.json':
            new_file = input_file + '.json'
            os.rename(input_file, new_file)



input_dir = os.path.join(os.getcwd(), 'data', 'my_data')
print(input_dir)
files = os.listdir(input_dir)
json_files = [file for file in files if file.endswith('.json')]
graph_file = os.path.join(input_dir, json_files[0])
with open(graph_file, 'r') as f:
    source_data = json.load(f)
    root_eclasses = source_data.get("root_eclasses", [])


output_dir = os.path.join(os.getcwd(), 'out_process_dag_result1')

for filename in os.listdir(output_dir):
    if filename.endswith(".json"):
        target_file_path = os.path.join(output_dir, filename)

        # Read target file content
        with open(target_file_path, "r") as target_file:
            target_data = json.load(target_file)

        # Add key-value pair to data
        target_data["root_eclasses"] = root_eclasses

        # Write updated data to target file
        with open(target_file_path, "w") as target_file:
            json.dump(target_data, target_file, indent=4)



