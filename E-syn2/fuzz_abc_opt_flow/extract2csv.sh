#!/bin/bash

# Define the log file name
log_file="abc_fuzz.log" # replace with your actual log file name

# Extract the lines containing 'WireLoad' and 'ps' and then format them into CSV
grep 'WireLoad' "$log_file" | sed -E 's/.*Gates = +([0-9]+).+Cap = +([0-9.]+) ff.+Area = +([0-9.]+).+Delay =([0-9.]+) ps.*/\1,\2,\3,\4/' | awk 'BEGIN{FS=","; OFS=","; print "Index,Gates,Cap,Area,Delay"} {print NR-1,$1,$2,$3,$4}' > output.csv

# Display the CSV file
cat Qor_fuzz_result.csv