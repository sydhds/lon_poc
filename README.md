# no_spel — precompile A/B

The `no_spel` crate from `lon_poc/poc_1`, unchanged except for one added block
in `precompiled/methods/guest/Cargo.toml`, plus both compiled guest binaries so
the comparison can be run without any toolchain.

## Result

Same program, same single ECDSA recovery, measured by the repo's own test:

| | total | paging | **user** |
|---|---:|---:|---:|
| baseline (repo as-is) | 12,058,624 | 271,482 | **11,428,025** |
| precompiled | 1,048,576 | 47,809 | **613,838** |
| | 11.5× | | **18.6×** |

`user` is the figure that matters. `total` is padded up to a power-of-two
segment size and the precompiled run already sits on the floor (2²⁰), so its
total understates the gain.

Against `MAX_NUM_CYCLES_PUBLIC_EXECUTION` (32M): 11,428,025 per signature means
**2 signatures** per public execution; 613,838 means **~52**.

## What is in here

```
precompiled/                     the no_spel crate, ready to run
  Cargo.toml                     workspace root — unchanged
  no_spel_program/src/lib.rs     the program — unchanged
  methods/guest/Cargo.toml       *** the only modified file ***
  methods/guest/src/bin/         guest entry point — unchanged
  no_spel_e2e_tests/src/lib.rs   the measuring test — unchanged
  no_spel_e2e_tests/no_spel.bin  the accelerated guest, ready to execute
bins/
  no_spel_baseline.bin           guest built without the patch
  no_spel_precompiled.bin        guest built with the patch
wrapelf/                         wraps a raw guest ELF into a ProgramBinary
run_ab.sh                        runs both and prints the comparison
RESULTS.txt                      full numbers and raw test output
```

## The only change

```
$ diff -r baseline precompiled
diff -r baseline/methods/guest/Cargo.toml precompiled/methods/guest/Cargo.toml
16a17,26
> [patch.crates-io]
> crypto-bigint = { git = "https://github.com/risc0/RustCrypto-crypto-bigint", tag = "v0.5.5-risczero.0" }
> k256 = { git = "https://github.com/risc0/RustCrypto-elliptic-curves", tag = "k256/v0.13.4-risczero.1" }
> tiny-keccak = { git = "https://github.com/risc0/tiny-keccak", tag = "tiny-keccak/v2.0.2-risczero.0" }
```

The block already exists in `no_spel/Cargo.toml`, the outer workspace root. But
`methods/guest` declares its own `[workspace]`, and Cargo only applies `[patch]`
from the root of the graph being built — so it never reached the guest.
`crypto-bigint` has to be added alongside: the k256 fork's `bigint2` path does
not compile without it.

## How to test

### 1. One command — no toolchain needed

```sh
./run_ab.sh
```

It runs the repo's own test twice, once against each supplied binary, and
prints the comparison:

```
                          total       paging           user
  baseline           12,058,624      271,482     11,428,025
  precompiled         1,048,576       47,809        613,838
  speedup                 11.5x                       18.6x
```

The test reads `no_spel.bin` from its own directory, so swapping that file is
the whole experiment — the script just does the swapping, runs
`cargo test --release -p no_spel_e2e_tests -- --nocapture`, and reads back the
line the test prints:

```
session info cycles: <total> total - <paging> paging - <user> user
```

It restores the accelerated binary on exit, including on failure.

The first run takes a while — `risc0-zkvm` with the `prove` feature compiles
several large C++ circuit kernels. The second run reuses that build.

If `risc0-circuit-recursion`'s build script fails because it cannot reach
`risc0-artifacts.s3.us-west-2.amazonaws.com`, that download is only needed for
recursive proving, which this never does; `DOCS_RS=1 ./run_ab.sh` skips it.

Note the test hard-codes `session_limit(Some(11_750_000))`, which the baseline
only just fits under. If you raise the signature count, raise that too.

### 2. Rebuild the guest yourself, with the RISC Zero toolchain

```sh
cd precompiled/methods/guest
cargo risczero build --manifest-path Cargo.toml
cp target/riscv32im-risc0-zkvm-elf/docker/no_spel.bin ../../no_spel_e2e_tests/
cd ../.. && cargo test --release -p no_spel_e2e_tests -- --nocapture
```

Delete the `[patch.crates-io]` block from `methods/guest/Cargo.toml`, delete
`methods/guest/Cargo.lock`, rebuild, and you have the baseline again.

### 3. Rebuild the guest without the RISC Zero toolchain

This is how the supplied binaries were produced — stock Rust plus `-Z build-std`
against a Rust source checkout, which is the mechanism `risc0-build` exposes as
`RISC0_RUST_SRC`.

```sh
# a Rust source tree matching your rustc
git clone --depth 1 --branch "$(rustc --version | cut -d' ' -f2)" \
  https://github.com/rust-lang/rust.git ~/rustsrc
cd ~/rustsrc && git submodule update --init --depth 1 library/backtrace library/stdarch

cd precompiled/methods/guest
export RUSTC_BOOTSTRAP=1
export __CARGO_TESTS_ONLY_SRC_ROOT=~/rustsrc/library
export RISC0_FEATURE_bigint2=1
export CC_riscv32im_risc0_zkvm_elf=/nonexistent     # no target C compiler needed
export CFLAGS_riscv32im_risc0_zkvm_elf="-march=rv32im -nostdlib"
export CARGO_ENCODED_RUSTFLAGS=$'-C\x1fpasses=lower-atomic\x1f-C\x1flink-arg=-Ttext=0x00200800\x1f-C\x1flink-arg=--fatal-warnings\x1f-C\x1fpanic=abort\x1f--cfg\x1fgetrandom_backend="custom"'

cargo build --release --target riscv32im-risc0-zkvm-elf \
  -Z build-std=alloc,core,proc_macro,panic_abort,std \
  -Z build-std-features=compiler-builtins-mem
```

That produces a raw ELF. `ExecutorImpl::from_elf` wants a RISC Zero
`ProgramBinary` — the user ELF plus the v1compat kernel — so wrap it:

```sh
cd ../../../wrapelf && cargo run --release -- \
  ../precompiled/methods/guest/target/riscv32im-risc0-zkvm-elf/release/no_spel \
  ../precompiled/no_spel_e2e_tests/no_spel.bin
```

Do **not** set a bare `CC=...` here. Cargo builds `spel-framework-macros` as a
proc-macro for the host, which drags in `ring`, and a global `CC` override
breaks that host C build. Scope it to the target with `CC_riscv32im_...`.

For the baseline, drop the patch block and `methods/guest/Cargo.lock`, unset
`RISC0_FEATURE_bigint2`, and repeat.

## Why this crate is called `no_spel`

SPEL is the contract framework — `#[lez_program]`, `#[instruction]`,
`#[account(init, pda = ...)]`, IDL generation. `sig_verif_2` in the same repo
uses all of it. `no_spel_program` deliberately does not: its `main()` is
hand-written, the `read_nssa_inputs` call is commented out, and it reads no
input and touches no account. It is a control — signature verification with the
framework taken out of the picture.

That makes it the right thing to measure here. SPEL costs roughly 250K cycles
per public execution in fixed overhead (account reads, borsh decode/encode of
state, writing the journal) — a per-transaction constant, not a per-signature
one. Measuring inside a SPEL program would fold that constant into the
comparison for no reason.

**SPEL and the precompiles are unrelated axes.** SPEL is how you write the
contract; the precompiles are how fast its cryptography runs inside the zkVM.
Turning the accelerators on is a change to `[patch.crates-io]` and nothing else
— the program source is untouched either way, and a SPEL program benefits from
exactly the same patch.
