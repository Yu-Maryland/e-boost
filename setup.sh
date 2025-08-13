#!/bin/bash

# setup.sh - Environment setup script for Extraction Tool
# This script helps configure the environment variables needed for the solvers

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}===============================================${NC}"
echo -e "${BLUE}      Extraction Tool Environment Setup      ${NC}"
echo -e "${BLUE}===============================================${NC}"
echo

# Function to check if a directory exists
check_directory() {
    local dir="$1"
    local name="$2"
    
    if [ -d "$dir" ]; then
        echo -e "${GREEN}✓${NC} Found $name at: $dir"
        return 0
    else
        echo -e "${RED}✗${NC} $name not found at: $dir"
        return 1
    fi
}

# Function to check if a file exists
check_file() {
    local file="$1"
    local name="$2"
    
    if [ -f "$file" ]; then
        echo -e "${GREEN}✓${NC} Found $name at: $file"
        return 0
    else
        echo -e "${RED}✗${NC} $name not found at: $file"
        return 1
    fi
}

# Function to detect installation paths
detect_paths() {
    echo -e "${YELLOW}Detecting solver installations...${NC}"
    echo
    
    # Detect Gurobi
    echo -e "${BLUE}Checking for Gurobi...${NC}"
    GUROBI_DETECTED=""
    for gurobi_path in /opt/gurobi*/linux64 ~/gurobi*/linux64 /usr/local/gurobi*/linux64; do
        if [ -d "$gurobi_path" ]; then
            GUROBI_DETECTED="$gurobi_path"
            check_directory "$gurobi_path" "Gurobi"
            break
        fi
    done
    if [ -z "$GUROBI_DETECTED" ]; then
        echo -e "${RED}✗${NC} Gurobi not found in common locations"
    fi
    echo
    
    # Detect CPLEX
    echo -e "${BLUE}Checking for CPLEX...${NC}"
    CPLEX_DETECTED=""
    CONCERT_DETECTED=""
    for cplex_base in /opt/ibm/ILOG/CPLEX_Studio* ~/CPLEX_Studio* /usr/local/CPLEX_Studio*; do
        if [ -d "$cplex_base" ]; then
            cplex_path="$cplex_base/cplex"
            concert_path="$cplex_base/concert"
            if [ -d "$cplex_path" ] && [ -d "$concert_path" ]; then
                CPLEX_DETECTED="$cplex_path"
                CONCERT_DETECTED="$concert_path"
                check_directory "$cplex_path" "CPLEX"
                check_directory "$concert_path" "Concert"
                break
            fi
        fi
    done
    if [ -z "$CPLEX_DETECTED" ]; then
        echo -e "${RED}✗${NC} CPLEX not found in common locations"
    fi
    echo
    
    # Detect OR-Tools
    echo -e "${BLUE}Checking for OR-Tools...${NC}"
    ORTOOLS_DETECTED=""
    for ortools_path in /opt/or-tools* ~/or-tools* /usr/local/or-tools*; do
        if [ -d "$ortools_path" ]; then
            ORTOOLS_DETECTED="$ortools_path"
            check_directory "$ortools_path" "OR-Tools"
            break
        fi
    done
    if [ -z "$ORTOOLS_DETECTED" ]; then
        echo -e "${RED}✗${NC} OR-Tools not found in common locations"
    fi
    echo
}

# Function to generate environment setup
generate_env_setup() {
    echo -e "${YELLOW}Generating environment setup...${NC}"
    echo
    
    cat > env_setup.sh << 'EOF'
#!/bin/bash
# Environment setup for Extraction Tool
# Source this file: source env_setup.sh

EOF
    
    if [ -n "$GUROBI_DETECTED" ]; then
        cat >> env_setup.sh << EOF
# Gurobi configuration
export GUROBI_HOME="$GUROBI_DETECTED"
export PATH="\${PATH}:\${GUROBI_HOME}/bin"
export LD_LIBRARY_PATH="\${LD_LIBRARY_PATH}:\${GUROBI_HOME}/lib"

EOF
    else
        cat >> env_setup.sh << 'EOF'
# Gurobi configuration (PLEASE UPDATE PATHS)
# export GUROBI_HOME="/path/to/gurobi1201/linux64"
# export PATH="${PATH}:${GUROBI_HOME}/bin"
# export LD_LIBRARY_PATH="${LD_LIBRARY_PATH}:${GUROBI_HOME}/lib"

EOF
    fi
    
    if [ -n "$CPLEX_DETECTED" ] && [ -n "$CONCERT_DETECTED" ]; then
        cat >> env_setup.sh << EOF
# CPLEX configuration
export CPLEX_HOME="$CPLEX_DETECTED"
export CONCERT_HOME="$CONCERT_DETECTED"

EOF
    else
        cat >> env_setup.sh << 'EOF'
# CPLEX configuration (PLEASE UPDATE PATHS)
# export CPLEX_HOME="/path/to/cplex"
# export CONCERT_HOME="/path/to/concert"

EOF
    fi
    
    if [ -n "$ORTOOLS_DETECTED" ]; then
        cat >> env_setup.sh << EOF
# OR-Tools configuration
export ORTOOLS_HOME="$ORTOOLS_DETECTED"
export LD_LIBRARY_PATH="\${LD_LIBRARY_PATH}:\${ORTOOLS_HOME}/lib"

EOF
    else
        cat >> env_setup.sh << 'EOF'
# OR-Tools configuration (PLEASE UPDATE PATHS)
# export ORTOOLS_HOME="/path/to/or-tools"
# export LD_LIBRARY_PATH="${LD_LIBRARY_PATH}:${ORTOOLS_HOME}/lib"

EOF
    fi
    
    cat >> env_setup.sh << 'EOF'
echo "Extraction Tool environment configured"
echo "GUROBI_HOME: ${GUROBI_HOME}"
echo "CPLEX_HOME: ${CPLEX_HOME}"
echo "CONCERT_HOME: ${CONCERT_HOME}"
echo "ORTOOLS_HOME: ${ORTOOLS_HOME}"
EOF
    
    chmod +x env_setup.sh
    echo -e "${GREEN}✓${NC} Generated env_setup.sh"
}

# Function to check Rust installation
check_rust() {
    echo -e "${BLUE}Checking Rust installation...${NC}"
    if command -v rustc &> /dev/null; then
        rust_version=$(rustc --version)
        echo -e "${GREEN}✓${NC} Rust found: $rust_version"
    else
        echo -e "${RED}✗${NC} Rust not found"
        echo -e "${YELLOW}Install Rust with:${NC}"
        echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo "source ~/.cargo/env"
    fi
    echo
}

# Function to test build
test_build() {
    echo -e "${YELLOW}Testing build process...${NC}"
    echo
    
    if [ -f "env_setup.sh" ]; then
        echo -e "${BLUE}To test the build, run:${NC}"
        echo "source env_setup.sh"
        echo "make help"
        echo "make all"
    else
        echo -e "${RED}env_setup.sh not found. Please run the setup first.${NC}"
    fi
}

# Main execution
main() {
    detect_paths
    check_rust
    generate_env_setup
    
    echo -e "${GREEN}===============================================${NC}"
    echo -e "${GREEN}              Setup Complete!                 ${NC}"
    echo -e "${GREEN}===============================================${NC}"
    echo
    echo -e "${YELLOW}Next steps:${NC}"
    echo -e "1. Review and edit ${BLUE}env_setup.sh${NC} if needed"
    echo -e "2. Source the environment: ${BLUE}source env_setup.sh${NC}"
    echo -e "3. Build the project: ${BLUE}make all${NC}"
    echo -e "4. See README.md for usage instructions"
    echo
    
    test_build
}

# Run main function
main "$@"
