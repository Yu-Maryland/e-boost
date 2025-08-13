FLAGS =
FLAGS += --features=$(FEATURES)

EXTRACTORS = $(shell cargo run -q $(FLAGS) -- --extractor=print)

PROGRAM = target/debug/extraction-gym

SRC = $(shell find . -name '*.rs') Cargo.toml Cargo.lock
DATA = $(shell find data -name '*.json')

TARGETS =

.PHONY: all
all: nits bench

define run-extraction
TARGETS += $(1:data/%=output/%)-$(2).json
$(1:data/%=output/%)-$(2).json: $(1)
	@mkdir -p $$(dir $$@)
	$(PROGRAM) $$< --extractor=$(2) --out=$$@
endef

$(foreach ext,$(EXTRACTORS),\
	$(foreach data,$(DATA),\
        $(eval $(call run-extraction,$(data),$(ext)))\
    )\
)

.PHONY: bench
bench: $(TARGETS)

$(PROGRAM): $(SRC)
	cargo build $(FLAGS)

.PHONY: nits
nits:
	rustup component add rustfmt clippy
	cargo clean --doc