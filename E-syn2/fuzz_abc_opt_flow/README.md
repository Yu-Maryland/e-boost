# Fuzzing ABC with Random Flow

## Generate Random Flow

```
chmod +x./RaomdomFlowGenerator.sh
./RaomdomFlowGenerator.sh <# of flows>
```

A file called `GeneratedFlows.txt` will be generated, which contains the random flow. Each line in the file represents a flow.

## Fuzzing ABC with Random Flow

```
python FuzzWithRandomFlow.py > abc_fuzz.log
```

This script will fuzz the ABC with the random flow generated before. 

## Comparing the QoR of ABC with Random Flow 

Check `stats.txt` for the statistics of the QoR result. The meanings of the columns are:

- `Circuit name`: The name of the circuit.
- `i/o` : The number of input and output ports (2 columns).
- `Gates`: The number of gates in the circuit.
- `Area`: The area of the circuit. (in um^2)
- `Delay`: The delay of the circuit. (in ps)

## Analyzing the Fuzzing Result (WIP)

```
chmod +x extract2csv.sh
./extract2csv.sh
```

The extracted data (QoR) will be stored in `Qor_fuzz_result.csv`. Now you can check the QoR result by applying different flows in ABC and compare the results.