# E-syn2

## Prequisites  

- Rust environment
- Berkeley ABC tool (replaced with the `abc/src/opt/dar/darRefact.c`)

## Usage

```bash
make
chmod +x ./run.sh && ./run.sh
```

quick start:

```bash
echo -e "5\ndelay\nfaster-bottom-up\n" | bash run.sh    
```

run with randomized extraction experiments:

```bash
echo -e "60\narea\nrandom-based-faster-bottom-up\n40\n0.8\n" | bash run.sh 
```

run with iterations experiments (without randomized extraction):

```bash
for i in $(seq 5 5 50); do
  echo -e "${i}\ndelay\nfaster-bottom-up\n" | bash run.sh > tmp_log/log_${i}_no_feature.txt
  wait
done
```

### Directory Structure

```
./
├── abc
├── converted_circuit_strash # benchmark circuits
├── data_process_script
├── deprecated
├── e-rewriter # rewriter and parser
|   ├── rewritten_circuit # rewriten circuits -> wait for extraction
│   ├── circuit0.eqn # put your circuit here
│   ├── src # includes frontend parser (eqn2egraph) and egraph-serializer
│   ├── target
├── extraction-gym # extractor
│   ├── input # 1. saturacted circuits graphs for extraction (copied from e-rewriter)
│   ├── out_dag_json # 2. raw json marked the extracted nodes - dag based extraction
│   ├── out_json # 2. raw json marked the extracted nodes - tree based extraction
│   ├── random_out_dag_json # 3. random extraction results -> raw json (random-based extraction)
│   ├── output_log # 4. log files during extraction
│   ├── src
│   └── target
├── process_json # post-processing script for extracted results
│   ├── input_saturacted_egraph # saturacted circuits
│   ├── input_extracted_egraph # extracted circuits
│   ├── out_process_dag_result # processed json (handled extracted nodes raw json)
│   ├── src
│   └── target
├── graph2eqn # convert extracted circuits to eqn format
│   ├── circuit0.eqn
│   ├── src
│   └── target
├── fuzz_abc_opt_flow # explore the optimization space of circuits
├── clean.sh
├── collect.sh
├── README.md
├── run.sh
├── test.sh
├── Makefile
└── tmp_log
    ├── abc_opt_all_{timestamp}.log # log file for abc optimization (complete log)
    ├── abc_opt_all_formatted_{timestamp}.log # log file for abc optimization (formatted log)
    └── log_{iteration_number}_no_feature.txt # log file for each iteration (no feature extraction)
```