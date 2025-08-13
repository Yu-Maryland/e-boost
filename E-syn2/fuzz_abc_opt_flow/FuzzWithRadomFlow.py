import os
import glob
import argparse
from concurrent.futures import ProcessPoolExecutor, as_completed

# Function to execute a single ABC flow
def execute_flow(flow, eqn_file):
    print(f"Fuzzing {eqn_file}")
    # Prepare the command with the given flow and eqn file
    command = "../abc/abc -c \"source -s ../abc/abc.rc; read_eqn {}; strash; {} ;read_lib ../abc/asap7_clean.lib ; map ; topo; upsize; dnsize; stime -d\"".format(eqn_file, flow)
    # Execute the command
    os.system(command)
    
    return "Completed: {} with {}".format(flow, eqn_file)

# Function to sample flows from a given file
def sampling_flows(list_flows, num_samples):
    import random
    random.shuffle(list_flows)
    return list_flows[:num_samples]

def main():
    # Create an argument parser
    parser = argparse.ArgumentParser(description='ABC Flow Fuzzer')
    parser.add_argument('flows_file', help='Path to the GeneratedFlows.txt file')
    parser.add_argument('--sample', '-s', action='store_true', help='Enable sampling of flows')
    parser.add_argument('--num_samples', '-n', type=int, help='Number of flows to sample')
    parser.add_argument('--circuit', '-c', choices=['original', 'optimized', 'random'], help='Circuit to fuzz')
    
    # Parse the command-line arguments
    args = parser.parse_args()
    
    # Load the flows from the specified file
    with open(args.flows_file, 'r') as file:
        flows = [line.strip() for line in file]
    
    # Determine the eqn files based on the --circuit argument
    if args.circuit == 'original':
        eqn_files = ["../abc/ori.eqn"]
    elif args.circuit == 'optimized':
        eqn_files = ["../abc/opt.eqn"]
    elif args.circuit == 'random':
        eqn_files = glob.glob("../abc/opt_*.eqn")
    else:
        parser.error('Invalid circuit choice')
    
    # Create a dictionary to store the sampled flows for each eqn_file
    sampled_flows_dict = {}
    
    # Sample flows for each eqn_file if the --sample flag is set
    if args.sample:
        if args.num_samples is None:
            parser.error('--num_samples is required when --sample is set')
        for eqn_file in eqn_files:
            sampled_flows_dict[eqn_file] = sampling_flows(flows, args.num_samples)
    else:
        for eqn_file in eqn_files:
            sampled_flows_dict[eqn_file] = flows
    
    # Number of parallel processes (up to the number of cores/cpus you have)
    max_workers = 64
    
    # Create a process pool and execute the flows in parallel
    with ProcessPoolExecutor(max_workers=max_workers) as executor:
        # Submit all the flows for execution with each eqn file
        futures = []
        for eqn_file, sampled_flows in sampled_flows_dict.items():
            for flow in sampled_flows:
                futures.append(executor.submit(execute_flow, flow, eqn_file))
        
        # Process the results as they complete
        for future in as_completed(futures):
            try:
                result = future.result()
                print(result)  # Or handle the result in another way
            except Exception as exc:
                print('Generated an exception: %s' % exc)

if __name__ == '__main__':
    main()