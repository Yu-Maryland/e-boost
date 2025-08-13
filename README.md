# Extraction Tool

A comprehensive optimization framework for logic synthesis using E-graphs and various solvers (Gurobi, CPLEX, CP-SAT). This tool provides optimal extraction from E-graphs and integrates with logic synthesis workflows for circuit optimization.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Installation & Environment Setup](#installation--environment-setup)
- [Quick Start](#quick-start)
- [Solver Usage](#solver-usage)
- [E-syn2 Integration](#e-syn2-integration)
- [Benchmarks](#benchmarks)
- [Directory Structure](#directory-structure)
- [Examples](#examples)

## Overview

The extraction tool consists of several components:

1. **Solver Backends**: Gurobi, CPLEX, and CP-SAT implementations for optimal E-graph extraction
2. **E-syn2**: Complete logic synthesis optimization workflow
3. **Benchmarks**: Test datasets for evaluation
4. **Supporting Libraries**: E-graph serialization and utilities

## Prerequisites

### System Requirements
- **Operating System**: Linux (tested on Linux x86_64)
- **Shell**: tcsh (default shell support)
- **Compiler**: GCC with C++17 support
- **Build Tool**: Make
- **Language Runtime**: Rust (for E-graph components)

### Required Software
- **Rust**: Latest stable version with Cargo
- **Berkeley ABC**: Logic synthesis tool
- **At least one solver**:
  - Gurobi Optimizer (commercial/academic license)
  - IBM CPLEX (commercial/academic license)
  - Google OR-Tools (free, includes CP-SAT)

## Installation & Environment Setup

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 2. Solver Setup

#### Gurobi Setup
1. Download Gurobi from [gurobi.com](https://www.gurobi.com/)
2. Extract the package:
   ```bash
   tar -xzf gurobi12.0.1_linux64.tar.gz
   ```
3. Set environment variables:
   ```bash
   export GUROBI_HOME=/path/to/gurobi1201/linux64
   export PATH="${PATH}:${GUROBI_HOME}/bin"
   export LD_LIBRARY_PATH="${LD_LIBRARY_PATH}:${GUROBI_HOME}/lib"
   ```
4. Install license file (gurobi.lic) in `$GUROBI_HOME/` or `$HOME/`

#### CPLEX Setup
1. Download IBM CPLEX Studio from [IBM website](https://www.ibm.com/products/ilog-cplex-optimization-studio)
2. Install CPLEX:
   ```bash
   chmod +x cplex_studio2211.linux_x86_64.bin
   ./cplex_studio2211.linux_x86_64.bin
   ```
3. Set environment variables:
   ```bash
   export CPLEX_HOME=/path/to/cplex
   export CONCERT_HOME=/path/to/concert
   ```

#### OR-Tools Setup (CP-SAT)
1. Download OR-Tools:
   ```bash
   wget https://github.com/google/or-tools/releases/download/v9.7/or-tools_amd64_ubuntu-22.04_cpp_v9.7.2996.tar.gz
   tar -xzf or-tools_amd64_ubuntu-22.04_cpp_v9.7.2996.tar.gz
   ```
2. Set environment variables:
   ```bash
   export ORTOOLS_HOME=/path/to/or-tools
   export LD_LIBRARY_PATH="${LD_LIBRARY_PATH}:${ORTOOLS_HOME}/lib"
   ```

### 3. Build the Project

#### Build Solvers
```bash
# Build Gurobi solver
cd gurobi/
g++ -std=c++17 main.cpp -o gurobi_solver -I$GUROBI_HOME/include -L$GUROBI_HOME/lib -lgurobi_c++ -lgurobi120

# Build CPLEX solver  
cd ../cplex/
g++ -std=c++17 main.cpp -o cplex_solver \
    -I$CONCERT_HOME/include -I$CPLEX_HOME/include \
    -L$CPLEX_HOME/lib/x86-64_linux/static_pic \
    -L$CONCERT_HOME/lib/x86-64_linux/static_pic \
    -lilocplex -lcplex -lconcert -lpthread -ldl

# Build CP-SAT solver
cd ../cpsat/
g++ -std=c++17 main.cpp -I./include \
    -L$ORTOOLS_HOME/lib -Wl,-rpath=$ORTOOLS_HOME/lib \
    -lortools -o cpsat -O3
```

#### Build E-syn2 Components
```bash
cd E-syn2/
make
```

## Quick Start

### Basic Solver Usage

1. **Prepare input files**: Ensure you have LP files (for Gurobi/CPLEX) or JSON files (for CP-SAT)

2. **Run Gurobi solver**:
   ```bash
   ./gurobi/gurobi_solver \
       --lp_file input.lp \
       --output_file result.sol \
       --log_file optimization.log \
       --time_limit 300
   ```

3. **Run CPLEX solver**:
   ```bash
   ./cplex/cplex_solver \
       --lp_file input.lp \
       --output_file result.sol \
       --log_file optimization.log \
       --time_limit 300
   ```

4. **Run CP-SAT solver**:
   ```bash
   ./cpsat/cpsat \
       --egraph_json_file input.json \
       --output_sol_file result.sol \
       --log_file optimization.log \
       --time_limit 300
   ```

## Solver Usage

### Command Line Parameters

#### Gurobi & CPLEX
- `--lp_file <file>`: Input LP format file (required)
- `--output_file <file>`: Output solution file (required)
- `--log_file <file>`: Optimization log file (required)
- `--mst_file <file>`: Warm start solution file (optional)
- `--time_limit <seconds>`: Time limit in seconds (optional)
- `--solution_pool_dir <dir>`: Directory to save all solutions (optional)

#### CP-SAT
- `--egraph_json_file <file>`: Input E-graph JSON file (required)
- `--output_sol_file <file>`: Output solution file (required)
- `--log_file <file>`: Optimization log file (required)
- `--zero_node_mst <file>`: Zero node constraints file (optional)
- `--total_gurobi_mst <file>`: Warm start file (optional)
- `--time_limit <seconds>`: Time limit in seconds (optional)
- `--solution_pool_dir <dir>`: Directory to save all solutions (optional)

### Output Formats

All solvers produce solution files in the format:
```
variable_name value
x_1_2 1.0
x_2_3 0.0
...
```

Log files contain optimization progress:
```
time_seconds: objective_value
0.15: 125.50
0.23: 120.30
...
```

## E-syn2 Integration

E-syn2 provides a complete logic synthesis optimization workflow using the extraction tool.

### Key Scripts

#### process_test.sh
Automated batch processing script that runs optimization on all EQN files in the test directory.

**Usage**:
```bash
cd E-syn2/
./process_test.sh
```

**Features**:
- Processes all `.eqn` files in the `test/` directory
- Tests multiple bounds (1, 1.05, 1.25, 1.50) and solvers
- Compares original vs. optimized results
- Generates comprehensive logs in `logs/` directory
- Provides progress tracking and runtime statistics

#### run_test.sh
Interactive script for single circuit optimization.

**Usage**:
```bash
cd E-syn2/
./run_test.sh
```

**Interactive Parameters**:
- **Iteration times**: Number of E-graph rewriting iterations (default: 30)
- **Cost function**: 'area' or 'delay' (default: 'area')
- **Extraction pattern**: 'faster-bottom-up' or 'random-based-faster-bottom-up' (default: 'faster-bottom-up')
- **Extraction mode**: 'new' (external solver) or 'ori' (original extractor) (default: 'ori')
- **Solver**: 'gurobi', 'cplex', or 'cpsat' (when using 'new' mode)
- **Bound**: Cost bound for extraction (when using 'new' mode)

### Workflow Pipeline

1. **Circuit Rewriting**: Convert EQN to E-graph and apply rewriting rules
2. **DAG Extraction**: Extract optimal DAG from saturated E-graph
3. **JSON Processing**: Process extraction results
4. **Graph to Equation**: Convert back to EQN format
5. **ABC Optimization**: Run Berkeley ABC for final optimization
6. **Equivalence Checking**: Verify logical equivalence

### Quick Examples

**Area optimization with external solver**:
```bash
echo -e "5\narea\nfaster-bottom-up\nnew\n1.25\ngurobi\n" | bash run_test.sh
```

**Delay optimization with original extractor**:
```bash
echo -e "10\ndelay\nfaster-bottom-up\nori\n" | bash run_test.sh
```

**Randomized extraction**:
```bash
echo -e "30\narea\nrandom-based-faster-bottom-up\nori\n" | bash run_test.sh
```

## Benchmarks

The `benchmark/` directory contains test datasets organized by source:

- **BoolE/**: Boolean function benchmarks
  - `mul32.json`, `mul32_map.json`: 32-bit multiplier
  - `mul48.json`, `mul48_map.json`: 48-bit multiplier

- **E-morphic/**: Morphic computing benchmarks
  - `adder.json`: Adder circuit
  - `log2.json`: Logarithm computation
  - `sin.json`: Sine function approximation

- **E-syn/**: Logic synthesis benchmarks
  - `c2670.json`: ISCAS benchmark circuit
  - `qdiv.json`: Quotient division circuit

- **SmootheE/**: Advanced benchmarks
  - `direct_recexpr_root_18.json`: Direct recursive expression
  - `fir_8_tap_7iteration_egraph.json`: FIR filter
  - `large_mul2048.json`: Large multiplier
  - `nasneta.json`: Neural architecture search
  - `vector_2d_conv_2x2_2x2_root_36.json`: 2D convolution

### Running Benchmarks

```bash
# Run specific benchmark with Gurobi
./gurobi/gurobi_solver \
    --lp_file benchmark/converted/mul32.lp \
    --output_file results/mul32_gurobi.sol \
    --log_file logs/mul32_gurobi.log \
    --time_limit 300

# Run E-syn2 on test circuits
cd E-syn2/
./process_test.sh  # Processes all test/*.eqn files
```

## Directory Structure

```
extraction-tool/
├── benchmark/              # Test datasets
│   ├── BoolE/             # Boolean function benchmarks
│   ├── E-morphic/         # Morphic computing benchmarks
│   ├── E-syn/             # Logic synthesis benchmarks
│   └── SmootheE/          # Advanced benchmarks
├── cplex/                 # CPLEX solver implementation
│   ├── main.cpp           # CPLEX solver source
│   └── cplex_solver       # Compiled binary
├── cpsat/                 # CP-SAT solver implementation
│   ├── main.cpp           # CP-SAT solver source
│   ├── cpsat              # Compiled binary
│   └── include/           # Headers for E-graph serialization
├── gurobi/                # Gurobi solver implementation
│   ├── main.cpp           # Gurobi solver source
│   └── gurobi_solver      # Compiled binary
├── E-syn2/                # Logic synthesis optimization workflow
│   ├── process_test.sh    # Batch processing script
│   ├── run_test.sh        # Interactive optimization script
│   ├── Makefile           # Build configuration
│   ├── abc/               # Berkeley ABC integration
│   ├── e-rewriter/        # E-graph rewriting
│   ├── extraction-gym/    # Extraction algorithms
│   ├── process_json/      # JSON post-processing
│   ├── graph2eqn/         # Graph to equation conversion
│   └── test/              # Test EQN files
├── egg/                   # E-graph utilities
├── egraph-serialize/      # E-graph serialization library
├── extraction_gym/        # Extraction gym library
├── file/                  # Working directories
│   ├── log/               # Log files
│   ├── lp/                # LP input files
│   ├── result/            # Solution files
│   └── start/             # Warm start files
└── src/                   # Main Rust sources
```

## Examples

### Example 1: Basic Circuit Optimization

```bash
# Place your circuit in E-syn2/e-rewriter/circuit0.eqn
cd E-syn2/
cp test/adder.eqn e-rewriter/circuit0.eqn

# Run optimization with Gurobi, bound 1.25
echo -e "5\narea\nfaster-bottom-up\nnew\n1.25\ngurobi\n" | bash run_test.sh
```

### Example 2: Batch Processing

```bash
# Process all test circuits with multiple configurations
cd E-syn2/
./process_test.sh

# Results will be in logs/ directory
ls logs/
# Output: adder/ bar/ c2670/ ... (one directory per circuit)
```

### Example 3: Compare Solvers

```bash
# Run same circuit with different solvers
echo -e "5\narea\nfaster-bottom-up\nnew\n1.25\ngurobi\n" | bash run_test.sh
echo -e "5\narea\nfaster-bottom-up\nnew\n1.25\ncplex\n" | bash run_test.sh
echo -e "5\narea\nfaster-bottom-up\nnew\n1.25\ncpsat\n" | bash run_test.sh
```

### Example 4: Custom Benchmark

```bash
# Use CP-SAT with benchmark data
./cpsat/cpsat \
    --egraph_json_file benchmark/BoolE/mul32.json \
    --output_sol_file results/mul32_cpsat.sol \
    --log_file logs/mul32_cpsat.log \
    --time_limit 600
```

## Troubleshooting

### Common Issues

1. **Solver not found**: Ensure environment variables are set and libraries are in PATH/LD_LIBRARY_PATH
2. **License errors**: Verify solver licenses are properly installed
3. **Compilation errors**: Check that all dependencies are installed and paths are correct
4. **Permission denied**: Ensure scripts have execute permissions (`chmod +x script.sh`)

### Debug Tips

- Check solver logs for detailed error messages
- Verify input file formats are correct
- Ensure sufficient disk space for solution pools
- Monitor memory usage for large instances

### Support

For issues related to:
- **Gurobi**: Check [Gurobi documentation](https://www.gurobi.com/documentation/)
- **CPLEX**: Refer to [IBM CPLEX documentation](https://www.ibm.com/docs/en/icos)
- **OR-Tools**: See [Google OR-Tools documentation](https://developers.google.com/optimization)
- **E-syn2**: Review script output and log files in `logs/` directory
