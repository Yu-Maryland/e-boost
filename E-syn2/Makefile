.PHONY: all compile clean

all: compile

compile:
	cd e-rewriter/ && CARGO_BUILD_JOBS=$(shell nproc) cargo build --release
	cd graph2eqn/ && CARGO_BUILD_JOBS=$(shell nproc) cargo build --release
	cd e-rewriter/src/egraph-serialize/ && CARGO_BUILD_JOBS=$(shell nproc) cargo build --release
	cd process_json/ && CARGO_BUILD_JOBS=$(shell nproc) cargo build --release
	cd extraction-gym/ && CARGO_BUILD_JOBS=$(shell nproc) cargo build --release

clean:
	cd e-rewriter/ && cargo clean
	cd graph2eqn/ && cargo clean
	cd e-rewriter/src/egraph-serialize/ && cargo clean
	cd process_json/ && cargo clean
	cd extraction-gym/ && cargo clean