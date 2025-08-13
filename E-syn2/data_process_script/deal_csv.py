import csv
import sys

input_filename1 = sys.argv[1]
output_filename = sys.argv[2]
existing_table = {}

# Read the first input CSV file
with open(input_filename1, 'r') as csvfile:
    reader = csv.reader(csvfile)
    for row in reader:
        op = row[0]
        data = row[1:]
        if op in existing_table:
            existing_table[op].extend(data)
        else:
            existing_table[op] = data


# Convert dictionary to list
merged_table = [[op, *data] for op, data in existing_table.items()]
merged_table[-1].insert(1, '0')
merged_table[-1].insert(6, '0')
merged_table[-1].insert(7, '0')
merged_table[-1].insert(8, '0')
merged_table[-1].insert(9, '0')
# Write to new CSV file
with open(output_filename, 'w', newline='') as csvfile:
    writer = csv.writer(csvfile)
    writer.writerows(merged_table)    