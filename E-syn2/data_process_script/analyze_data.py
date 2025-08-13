import csv
import re
import sys

if len(sys.argv) != 3:
    print("Usage: python script.py input_filename output_filename")
    sys.exit(1)

input_filename = sys.argv[1]
output_filename = sys.argv[2]

headers = ['WireLoad','iteration', 'Gates', 'Cap', 'Area', 'Delay', 'time','Classes', 'Nodes','cost_time','best', 'Rebuilds','and','lev']

table = []
i = 0
with open(input_filename, 'r') as f:
    lines = f.readlines()
    for line in lines:
        if 'iteration set' in line:
            row = []
            i += 1
            row.append("op" + str(i - 1))
            iteration_set_match = re.search(r'iteration set = (\d+)', line)
            if iteration_set_match:
                iteration_set = iteration_set_match.group(1)
                row.append(iteration_set)
            table.append(row)

i = 0
with open(input_filename, 'r') as f:
    lines = f.readlines()
    for line in lines:
        if 'WireLoad' in line:
            row = []
            i += 1
            row.append("op" + str(i - 1))
            gates_match = re.search(r'Gates\s*=\s*(\d+)', line)
            if gates_match:
                gates = gates_match.group(1)
                row.append(gates)
            cap_match = re.search(r'Cap\s*=\s*([\d.]+)\s+ff', line)
            if cap_match:
                cap = cap_match.group(1)
                row.append(cap)
            area_match = re.search(r'Area\s*=\s*(\d+)', line)
            if area_match:
                area = area_match.group(1)
                row.append(area)
            delay_match = re.search(r'Delay\s*=\s*([\d.]+)\s+ps', line)
            if delay_match:
                delay = delay_match.group(1)
                row.append(delay)

            table.append(row)

i = 0
with open(input_filename, 'r') as f:
    lines = f.readlines()
    for line in lines:
        if 'Time take for runner:' in line:
            row = []
            i += 1
            row.append("op" + str(i - 1))
            time_match = re.search(r'Time take for runner:\s*([\d.]+)\s*s', line)
            if time_match:
                time = time_match.group(1)
                row.append(time)
                classes_match = re.search(r'Classes:\s+(\d+)', line)
                if classes_match:
                    classes = classes_match.group(1)
                    row.append(classes)
                nodes_match = re.search(r'Nodes:\s+(\d+)', line)
                if nodes_match:
                    nodes = nodes_match.group(1)
                    row.append(nodes)
                table.append(row)

i = 0
with open(input_filename, 'r') as f:
    lines = f.readlines()
    for line in lines:
        if line.startswith('find_costs:'):
            row = []
            i += 1
            row.append("op" + str(i - 1))
            cost_match = re.search(r'find_costs:\s*(\d+\.?\d*)', line)
            if cost_match:
                time = cost_match.group(1)
                row.append(time)
            table.append(row)

i = 0
with open(input_filename, 'r') as f:
    lines = f.readlines()
    for line in lines:
        if re.match(r'^best\d+', line):
            row = []
            i += 1
            row.append("op" + str(i - 1))
            cost_match = re.search(r'best(\d+)', line)
            if cost_match:
                time = cost_match.group(1)
                row.append(time)
            table.append(row)


i = 0
with open(input_filename, 'r') as f:
    lines = f.readlines()
    for line in lines:
        if 'Rebuilds:' in line:
            row = []
            i += 1
            row.append("op" + str(i - 1))
            rebuilds_match = re.search(r'Rebuilds:\s+(\d+)', line)
            if rebuilds_match:
                rebuilds = rebuilds_match.group(1)
                row.append(rebuilds)
            table.append(row)

i = 0




with open(input_filename, 'r') as f:
    lines = f.readlines()
    for line in lines:

            and_match = re.search(r'and =\s+(\d+)', line)
            lev_match = re.search(r'lev =\s*(\d+)', line)
          
            if and_match and lev_match:
                row = []
                i += 1
                row.append("op" + str(i - 1))
                
                ands = and_match.group(1)
                row.append(ands)
                levs = lev_match.group(1)
                row.append(levs)
                table.append(row)





with open(output_filename, 'w', newline='') as csvfile:
    writer = csv.writer(csvfile)
    writer.writerow(headers)
    writer.writerows(table)