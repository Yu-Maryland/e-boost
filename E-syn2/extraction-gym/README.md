# Extraction Gym

A suite of benchmarks to test e-graph extraction algorithms.

Add your algorithm in `src/extract` and then add a line in `src/main.rs`. 
To run, type `make`.

If you need to enable more features, do 

```
make FEATURES=my-feature,my-other-feature
```

## Data

Please add data! It's just a JSON! See the `data/` directory for examples.

Make sure all root eclass ids are canonical!

Go check out the [egraph-serialize](https://github.com/egraphs-good/egraph-serialize) repo to see how to make the format!


#execution


cd E-Syn2/extraction-gym-new/extraction-gym/
make


cd E-Syn2/E-Brush-new/e-rewriter
cargo run /E-Brush-new/test_data_beta_runner/sexpr_for_egg_1.txt /E-Brush-new/test_data_beta_runner/sexpr_for_egg.txt
