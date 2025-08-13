#!/bin/bash
# Script to run run_test.sh on all EQN files in the test folder

# Colors for output
RED="\e[31m"
GREEN="\e[32m"
YELLOW="\e[1;33m"
RESET="\e[0m"

# Directory containing EQN files
EQN_DIR="test"
# Log directory base
LOG_BASE_DIR="logs"
# Path to the target circuit0.eqn file
TARGET_PATH="e-rewriter/circuit0.eqn"
# Base directory where run_test.sh is located
BASE_DIR="$(pwd)"

# Define arrays for bounds and solvers
bounds=(1 1.05 1.25 1.50)
solvers=("gurobi")

# Create logs directory if it doesn't exist
mkdir -p "$LOG_BASE_DIR"

# Get the total count of EQN files for progress reporting
total_files=$(find "$EQN_DIR" -name "*.eqn" | wc -l)
current_file=0

echo -e "${GREEN}Found $total_files EQN files to process${RESET}"
echo -e "${GREEN}Results will be saved in $LOG_BASE_DIR${RESET}"

# Create a summary log file
summary_log="$LOG_BASE_DIR/summary.log"
echo "Processing started at $(date)" > "$summary_log"
echo "Total files to process: $total_files" >> "$summary_log"
echo "Bounds: ${bounds[*]}" >> "$summary_log"
echo "Solvers: ${solvers[*]}" >> "$summary_log"
echo "------------------------------------------" >> "$summary_log"

# Function to clean up directories between runs
cleanup_directories() {
    rm -rf e-rewriter/rewritten_circuit/*
    rm -rf e-rewriter/random_graph/*
    rm -rf extraction-gym/input/*
    rm -rf extraction-gym/out_dag_json/*
    rm -rf extraction-gym/out_json/*
    rm -rf extraction-gym/output_log/*
    rm -rf process_json/input_saturacted_egraph/*
    rm -rf process_json/input_extracted_egraph/*
    rm -rf process_json/out_process_dag_result/*
    rm -rf extraction-gym/random_out_dag_json/*
    rm -rf graph2eqn/*.json
    rm -rf graph2eqn/*.eqn
    rm -rf abc/*.eqn
}

# Process each EQN file
find "$EQN_DIR" -name "*.eqn" | sort | while read eqn_file; do
    current_file=$((current_file + 1))
    base_name=$(basename "$eqn_file" .eqn)
    
    start_time=$(date +"%Y-%m-%d %H:%M:%S")
    echo -e "\n${YELLOW}[$current_file/$total_files] Processing file: $eqn_file (Started at: $start_time)${RESET}"
    
    # Log to summary
    echo "[$current_file/$total_files] Processing $eqn_file (Started at: $start_time)" >> "$summary_log"
    
    # Create log directory for this file
    log_dir="$LOG_BASE_DIR/$base_name"
    mkdir -p "$log_dir"
    
    # Ensure we're in the base directory before starting
    cd "$BASE_DIR"
    
    # Clean up from previous run
    cleanup_directories
    
    # Copy the EQN file to the target location
    cp "$eqn_file" "$TARGET_PATH" || {
        echo -e "${RED}Failed to copy $eqn_file to $TARGET_PATH. Skipping.${RESET}"
        echo "  ERROR: Failed to copy $eqn_file to $TARGET_PATH" >> "$summary_log"
        continue
    }
    
    # Save a copy of the original file in the log directory
    cp "$eqn_file" "$log_dir/original.eqn"
    
    # Run with original extractor
    echo -e "${YELLOW}Running with original extractor...${RESET}"
    cd "$BASE_DIR"
    ori_start=$(date +%s)
    (cd "$BASE_DIR" && /usr/bin/time -v bash -c "echo -e \"5\narea\nfaster-bottom-up\nori\n\" | bash run_test.sh") > "$log_dir/run_ori.log" 2> "$log_dir/time_ori.log"
    ori_end=$(date +%s)
    ori_runtime=$((ori_end - ori_start))
    
    # Capture the ABC results if available
    if [ -f "abc/opt.eqn" ]; then
        cp "abc/opt.eqn" "$log_dir/result_ori.eqn"
    fi
    
    # Ensure we're back in the base directory
    cd "$BASE_DIR"
    
    # Run with new extractor for each bound and solver combination
    for bound in "${bounds[@]}"; do
        for solver in "${solvers[@]}"; do
            echo -e "${YELLOW}Running with new extractor (bound=$bound, solver=$solver)...${RESET}"
            
            # Clean up directories for this run
            cleanup_directories
            
            # Copy the EQN file again to ensure a clean start
            cp "$eqn_file" "$TARGET_PATH"
            
            cd "$BASE_DIR"
            new_start=$(date +%s)
            
            # Run with specified bound and solver
            (cd "$BASE_DIR" && /usr/bin/time -v bash -c "echo -e \"5\narea\nfaster-bottom-up\nnew\n$bound\n$solver\n\" | bash run_test.sh") > "$log_dir/run_new_${bound}_${solver}.log" 2> "$log_dir/time_new_${bound}_${solver}.log"
            
            new_end=$(date +%s)
            new_runtime=$((new_end - new_start))
            
            # Capture the ABC results if available
            if [ -f "abc/opt.eqn" ]; then
                cp "abc/opt.eqn" "$log_dir/result_new_${bound}_${solver}.eqn"
            fi
            
            # Ensure we're back in the base directory
            cd "$BASE_DIR"
            
            echo "  NEW runtime (bound=$bound, solver=$solver): $new_runtime seconds" >> "$summary_log"
        done
    done
    
    end_time=$(date +"%Y-%m-%d %H:%M:%S")
    echo -e "${GREEN}Completed processing $eqn_file at $end_time${RESET}"
    
    # Update summary log
    echo "  Completed at: $end_time" >> "$summary_log"
    echo "  ORI runtime: $ori_runtime seconds" >> "$summary_log"
    echo "------------------------------------------" >> "$summary_log"
done

echo -e "\n${GREEN}All files processed. Results are saved in $LOG_BASE_DIR${RESET}"
echo "Processing completed at $(date)" >> "$summary_log"