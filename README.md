# FRIES (Fuzzing Rust Library Interactions via Ecosystem-Guided Target Generation)
This is the prototype tool for FRIES. We implemented FRIES based on the rustc compiler, utilizing the MIR module and librustdoc.

# How to run it

## Change the version of the rustup toolchain.
```
rustup toolchain install nightly-2022-11-30  
rustup default nightly-2022-11-30 
```
## install it
```
cd $WORKDIR
./x.py setup  ./x.py check
# This may fail because the LLVM library from six months ago may not be available.
./x.py build --stage=2 && rustup toolchain link fuzz build/x86_64-unknown-linux-gnu/stage2 
```

## Analyse target library
```
cd $TL_ROOT_DIR
cargo +fuzz doc --target-dir=tested
```

## Analyse corpus crate

```
cd $CP_DIR
cargo +fuzz doc
```

Alternatively, there are automation scripts available for running. Before using it, you need to modify the code inside. In parse_dependents.rs, locate the last line and modify it to your experiment root directory, for example:
```
const EXPERIMENT_ROOT_PATH: &'static str = "/home/.../workspace/fuzz/experiment_root/";
```
install it
```
cd RustFuzzer
cargo install --path .
```

