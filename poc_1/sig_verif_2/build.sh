#!/bin/bash

# stricter mode
# set -e
set -euo pipefail

# allow alias (spel, wallet)
shopt -s expand_aliases
source ~/.bash_aliases


CARGO_TARGET_DIR=${HOME}/native_target cargo risczero build --manifest-path methods/guest/Cargo.toml
echo "Generating idl..."
spel generate-idl methods/guest/src/bin/sig_verif_2.rs > sig_verif_2-idl.json
echo "Deploying program..."
wallet deploy-program /home/ubuntu/native_target/riscv32im-risc0-zkvm-elf/docker/sig_verif_2.bin

