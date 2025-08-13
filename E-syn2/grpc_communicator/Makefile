# PROTO_DIR = proto
# RUST_OUT_DIR = rust/src
# PYTHON_OUT_DIR = python

# .PHONY: all clean generate_rust generate_python run_server run_client

# all: generate_rust generate_python

# clean:
#     rm -rf $(RUST_OUT_DIR)/*.rs
#     rm -rf $(PYTHON_OUT_DIR)/*.py

# generate_rust:
#     mkdir -p $(RUST_OUT_DIR)
#     protoc --rust_out=$(RUST_OUT_DIR) --proto_path=$(PROTO_DIR) $(PROTO_DIR)/service.proto
#     cargo build

# generate_python:
#     mkdir -p $(PYTHON_OUT_DIR)
#     python -m grpc_tools.protoc -I$(PROTO_DIR) --python_out=$(PYTHON_OUT_DIR) --grpc_python_out=$(PYTHON_OUT_DIR) $(PROTO_DIR)/service.proto

# run_server:
#     python python/server.py

# run_client:
#     cargo run --manifest-path rust/Cargo.toml