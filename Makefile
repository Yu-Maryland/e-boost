# Makefile for Extraction Tool
# Builds all solver backends and E-syn2 components

.PHONY: all solvers gurobi cplex cpsat e-syn2 clean help

# Default target
all: solvers e-syn2

# Build all solvers
solvers: gurobi cplex cpsat

# Build Gurobi solver
gurobi:
	@echo "Building Gurobi solver..."
	@if [ -z "$(GUROBI_HOME)" ]; then \
		echo "Error: GUROBI_HOME environment variable not set"; \
		echo "Please set GUROBI_HOME to your Gurobi installation directory"; \
		exit 1; \
	fi
	cd gurobi && g++ -std=c++17 main.cpp -o gurobi_solver \
		-I$(GUROBI_HOME)/include \
		-L$(GUROBI_HOME)/lib \
		-lgurobi_c++ -lgurobi120
	@echo "Gurobi solver built successfully"

# Build CPLEX solver
cplex:
	@echo "Building CPLEX solver..."
	@if [ -z "$(CPLEX_HOME)" ] || [ -z "$(CONCERT_HOME)" ]; then \
		echo "Error: CPLEX_HOME and/or CONCERT_HOME environment variables not set"; \
		echo "Please set both CPLEX_HOME and CONCERT_HOME to your CPLEX installation directories"; \
		exit 1; \
	fi
	cd cplex && g++ -std=c++17 main.cpp -o cplex_solver \
		-I$(CONCERT_HOME)/include \
		-I$(CPLEX_HOME)/include \
		-L$(CPLEX_HOME)/lib/x86-64_linux/static_pic \
		-L$(CONCERT_HOME)/lib/x86-64_linux/static_pic \
		-lilocplex -lcplex -lconcert -lpthread -ldl
	@echo "CPLEX solver built successfully"

# Build CP-SAT solver
cpsat:
	@echo "Building CP-SAT solver..."
	@if [ -z "$(ORTOOLS_HOME)" ]; then \
		echo "Error: ORTOOLS_HOME environment variable not set"; \
		echo "Please set ORTOOLS_HOME to your OR-Tools installation directory"; \
		exit 1; \
	fi
	cd cpsat && g++ -std=c++17 main.cpp -I./include \
		-L$(ORTOOLS_HOME)/lib \
		-Wl,-rpath=$(ORTOOLS_HOME)/lib \
		-lortools -o cpsat -O3
	@echo "CP-SAT solver built successfully"

# Build E-syn2 components
e-syn2:
	@echo "Building E-syn2 components..."
	cd E-syn2 && $(MAKE)
	@echo "E-syn2 components built successfully"

# Clean all build artifacts
clean:
	@echo "Cleaning build artifacts..."
	-rm -f gurobi/gurobi_solver
	-rm -f cplex/cplex_solver
	-rm -f cpsat/cpsat
	cd E-syn2 && $(MAKE) clean 2>/dev/null || true
	@echo "Clean completed"

# Show help
help:
	@echo "Extraction Tool Build System"
	@echo ""
	@echo "Targets:"
	@echo "  all      - Build all components (default)"
	@echo "  solvers  - Build all solver backends"
	@echo "  gurobi   - Build Gurobi solver only"
	@echo "  cplex    - Build CPLEX solver only"
	@echo "  cpsat    - Build CP-SAT solver only"
	@echo "  e-syn2   - Build E-syn2 components only"
	@echo "  clean    - Remove all build artifacts"
	@echo "  help     - Show this help message"
	@echo ""
	@echo "Environment Variables Required:"
	@echo "  GUROBI_HOME   - Path to Gurobi installation"
	@echo "  CPLEX_HOME    - Path to CPLEX installation"
	@echo "  CONCERT_HOME  - Path to Concert (CPLEX) installation"
	@echo "  ORTOOLS_HOME  - Path to OR-Tools installation"
	@echo ""
	@echo "Example:"
	@echo "  export GUROBI_HOME=/opt/gurobi1201/linux64"
	@echo "  export CPLEX_HOME=/opt/cplex"
	@echo "  export CONCERT_HOME=/opt/concert"
	@echo "  export ORTOOLS_HOME=/opt/or-tools"
	@echo "  make all"
