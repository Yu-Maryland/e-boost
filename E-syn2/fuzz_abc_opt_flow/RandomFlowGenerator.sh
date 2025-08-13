#!/bin/bash

# Commands array
cmds=("rw" "rwz" "rf" "rfz" "balance" "resub" "resub -z" "resyn2" "compress2" "dch ; strash" "compress2rs")

# Function to generate a random sequence
generate_sequence() {
    local len=$1  # Length of the sequence
    local seq=""  # Initialize empty sequence
    for (( i=0; i<$len; i++ )); do
        # Get a random command from cmds
        local cmd=${cmds[$RANDOM % ${#cmds[@]}]}
        # Append command to the sequence
        seq+="$cmd; "
    done
    # add one more strash
    seq+="strash; "
    # Print the generated sequence
    echo "${seq%??}"  # remove the last "; "
}

# Check for user input
if [ $# -eq 0 ]; then
    echo "Usage: $0 <number_of_flows>"
    exit 1
fi

# Number of flows to generate
num_flows=$1

# Sequence lengths to consider
#lengths=(10 15 20 25)

# Random length minimum and maximum (inclusive)
min_length=10
max_length=25

# Output file
output_file="GeneratedFlows.txt"

# Make sure the output file is empty
> "$output_file"

# Generate the specified number of flows
for (( n=0; n<$num_flows; n++ )); do
    # Randomly select a length
    #len=${lengths[$RANDOM % ${#lengths[@]}]}
    len=$((RANDOM % (max_length + 1 - min_length) + min_length))
    # Generate a sequence of the selected length
    seq=$(generate_sequence $len)
    # Write the sequence to the output file
    echo "$seq" >> "$output_file"
done

echo "Generated $num_flows flows into $output_file"