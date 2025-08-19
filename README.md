# E-boost: Boosted E-Graph Extraction with Adaptive Heuristics and Exact Solving (ICCAD'25)

[![arXiv](https://img.shields.io/badge/arXiv-2508.13020-b31b1b.svg)](https://arxiv.org/abs/2508.13020)

E-graph extraction is a challenging NP-hard optimization problem that serves as the primary bottleneck in e-graph based applications. Traditional methods face a critical trade-off between speed and optimality.

E-boost bridges this gap through three key innovations: (1) **parallelized heuristic extraction** for efficient multi-threaded performance, (2) **adaptive search space pruning** to reduce solution space while preserving quality, and (3) **initialized exact solving** with warm-start ILP capabilities for faster convergence to optimal solutions, as described in our [ICCAD'25 paper](https://arxiv.org/abs/2508.13020).

---

## 📄 Paper

If you use E-boost in your research, please cite our paper:

```bibtex
@article{song2025eboost,
  title={E-boost: Boosted E-Graph Extraction with Adaptive Heuristics and Exact Solving},
  author={Song, Zhan and others},
  journal={arXiv preprint arXiv:2508.13020},
  year={2025}
}
```

---

## 🚀 Quick Start

### Overview

E-boost consists of several key components:

1. **Parallelized Heuristic Extraction**: Multi-threaded DAG cost computation with optimized data structures
2. **Adaptive Search Space Pruning**: Parameterized threshold mechanism for candidate selection
3. **Initialized Exact Solving**: ILP formulation with warm-start capabilities
4. **Solver Backends**: Support for Gurobi, CPLEX, and CP-SAT solvers
5. **Benchmark Suite**: Comprehensive test datasets for evaluation

---

## 📋 Prerequisites

### System Requirements
- **Operating System**: Linux (tested on Linux x86_64)
- **Compiler**: GCC with C++17 support
- **Build Tool**: Make
- **Language Runtime**: Rust (for E-graph components)

### Required Software
- **Rust**: Latest stable version with Cargo
- **At least one solver**:
  - Gurobi Optimizer (commercial/academic license)
  - IBM CPLEX (commercial/academic license)
  - Google OR-Tools (free, includes CP-SAT)

---

## ⚙️ Installation & Environment Setup

### 1. Install Required Rust Environment

Make sure you have [Rust](https://www.rust-lang.org/tools/install) installed (recommended: stable toolchain):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installation, ensure `cargo` and `rustc` are in your PATH:

```bash
rustc --version
cargo --version
```

---

### 2. Build E-boost

Once Rust is installed, build E-boost:

```bash
cargo build --release
```

---

### 3. Solver Setup

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

---

### 4. Build the Project

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

---

## 🧪 Benchmarks

The `benchmark/` directory contains test datasets organized by source:

- **BoolE/**: BoolE benchmarks
  - `mul32.json`, `mul32_map.json`: 32-bit multiplier
  - `mul48.json`, `mul48_map.json`: 48-bit multiplier

- **E-morphic/**: E-morphic benchmarks
  - `adder.json`: Adder circuit
  - `log2.json`: Logarithm computation
  - `sin.json`: Sine function approximation

- **E-syn/**: E-Syn benchmarks
  - `c2670.json`: ISCAS benchmark circuit
  - `qdiv.json`: Quotient division circuit

- **SmootheE/**: SmootheE benchmarks
  - `direct_recexpr_root_18.json`: Direct recursive expression
  - `fir_8_tap_7iteration_egraph.json`: FIR filter
  - `large_mul2048.json`: Large multiplier
  - `nasneta.json`: Neural architecture search
  - `vector_2d_conv_2x2_2x2_root_36.json`: 2D convolution

---

## 🔧 Usage

### Running Benchmarks

E-boost provides flexible command-line options for different extraction scenarios. The basic command structure is:

```bash
cargo run -- --bound <threshold> --solver <solver> --timeout <seconds> --extractor <algorithm> --pre <mode> <benchmark_file>
```

#### Command-Line Parameters

- **`--bound <value>`**: Threshold parameter for adaptive search space pruning (e.g., 1.25)
  - Values > 1.0 retain more candidates while increasing search space
  - Lower values (closer to 1.0) are more aggressive in pruning
  
- **`--solver <name>`**: Choose optimization solver backend
  - `gurobi`: Commercial solver (requires license)
  - `cplex`: IBM CPLEX solver (requires license) 
  - `cpsat`: Google OR-Tools CP-SAT (free)

- **`--timeout <seconds>`**: Maximum execution time in seconds

- **`--extractor <algorithm>`**: Heuristic extraction algorithm variant
  - `faster-greedy-dag-mt1`: Multi-threaded parallelized extraction (recommended)
  - `faster-greedy-dag-mt2`: Alternative multi-threaded variant
  - `faster-greedy-dag`: Single-threaded version

- **`--pre <mode>`**: Preprocessing and execution mode (0-5)
  - `0`: Solver only (skip LP generation)
  - `1`: Generate LP file only, no warm start
  - `2`: Generate LP file only, with warm start (default)
  - `3`: Full run without warm start
  - `4`: Full run with warm start (recommended for best results)
  - `5`: Heuristic extraction only

#### Usage Examples

**Basic optimization with warm start:**
```bash
cargo run -- --bound 1.25 --solver gurobi --timeout 1800 --extractor faster-greedy-dag-mt1 --pre 4 benchmark/BoolE/mul32_map.json
```

**Quick heuristic-only extraction:**
```bash
cargo run -- --bound 1.25 --solver gurobi --timeout 300 --extractor faster-greedy-dag-mt1 --pre 5 benchmark/BoolE/mul32.json
```

**Generate optimization files without solving:**
```bash
cargo run -- --bound 1.50 --solver cplex --timeout 3600 --extractor faster-greedy-dag-mt1 --pre 2 benchmark/SmootheE/fir_8_tap_7iteration_egraph.json
```

**Using free CP-SAT solver:**
```bash
cargo run -- --bound 1.25 --solver cpsat --timeout 1800 --extractor faster-greedy-dag-mt1 --pre 4 benchmark/E-syn/c2670.json
```

---

## 🔧 Real-World Applications: E-syn2 Logic Synthesis Integration

**Note**: Make sure you have built E-boost using `cargo build --release` before using the E-syn2 integration.

E-boost demonstrates its practical impact through integration with E-syn2, a complete logic synthesis optimization workflow. This integration showcases how E-boost's optimal extraction capabilities significantly improve real-world circuit optimization tasks.

### E-syn2 Scripts

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

### Integration Examples

#### Example 1: Basic Circuit Optimization

```bash
# First, ensure E-boost is built
cargo build --release

# Place your circuit in E-syn2/e-rewriter/circuit0.eqn
cd E-syn2/
cp test/adder.eqn e-rewriter/circuit0.eqn

# Run optimization with Gurobi, bound 1.25
echo -e "5\narea\nfaster-bottom-up\nnew\n1.25\ngurobi\n" | bash run_test.sh
```

#### Example 2: Batch Processing

```bash
# First, ensure E-boost is built
cargo build --release

# Process all test circuits with multiple configurations
cd E-syn2/
./process_test.sh

# Results will be in logs/ directory
ls logs/
# Output: # Output: adder/ bar/ c2670/ ... (one directory per circuit)
```

---

## 📧 Contact

For questions or feedback, please reach out to the authors listed in the [paper](https://arxiv.org/abs/2508.13020) or open an issue in this repository.

---

## 📜 License

This project is licensed under the MIT License.

---

Enjoy boosted E-graph extraction with E-boost!