#!/usr/bin/env bash
# Runs the repo's own test twice — once against the baseline guest, once
# against the accelerated one — and prints the comparison.
#
# The two guests were built from identical sources; the only difference is the
# [patch.crates-io] block in precompiled/methods/guest/Cargo.toml.
#
#   ./run_ab.sh
#
# The first run compiles risc0-zkvm with the `prove` feature, which includes
# several large C++ circuit kernels. Expect that to take a while. The second
# run reuses it.

set -euo pipefail

cd "$(dirname "$0")"

TEST_DIR="precompiled/no_spel_e2e_tests"
TEST_BIN="$TEST_DIR/no_spel.bin"
BASELINE="bins/no_spel_baseline.bin"
PRECOMPILED="bins/no_spel_precompiled.bin"

for f in "$BASELINE" "$PRECOMPILED" "$TEST_DIR/src/lib.rs"; do
    [[ -f "$f" ]] || { echo "missing: $f" >&2; exit 1; }
done

# Always leave the accelerated binary in place, even on failure.
restore() { cp -f "$PRECOMPILED" "$TEST_BIN" 2>/dev/null || true; }
trap restore EXIT

run_variant() {
    local label="$1" binary="$2"
    cp -f "$binary" "$TEST_BIN"
    echo "── $label ─────────────────────────────────────────────" >&2
    local line
    line=$(cd precompiled && cargo test --release -p no_spel_e2e_tests -- --nocapture 2>&1 \
        | tee /dev/stderr | grep -m1 "session info cycles" || true)
    if [[ -z "$line" ]]; then
        echo "no cycle line — the test did not report; see output above" >&2
        exit 1
    fi
    # session info cycles: <total> total - <paging> paging - <user> user
    echo "$line" | sed -E 's/.*cycles: ([0-9]+) total - ([0-9]+) paging - ([0-9]+) user.*/\1 \2 \3/'
}

read -r BASE_TOTAL BASE_PAGING BASE_USER < <(run_variant "baseline (no patch)" "$BASELINE")
read -r PRE_TOTAL PRE_PAGING PRE_USER < <(run_variant "precompiled (risc0 accelerators)" "$PRECOMPILED")

BUDGET=33554432

awk -v bt="$BASE_TOTAL" -v bp="$BASE_PAGING" -v bu="$BASE_USER" \
    -v pt="$PRE_TOTAL"  -v pp="$PRE_PAGING"  -v pu="$PRE_USER" \
    -v budget="$BUDGET" '
function commas(n,   s, out) {
    s = sprintf("%d", n); out = ""
    while (length(s) > 3) { out = "," substr(s, length(s)-2) out; s = substr(s, 1, length(s)-3) }
    return s out
}
BEGIN {
    printf "\n"
    printf "  no_spel — one ECDSA recovery, identical source, identical test\n"
    printf "  the only difference is [patch.crates-io] in methods/guest/Cargo.toml\n\n"
    printf "  %-14s %14s %12s %14s\n", "", "total", "paging", "user"
    printf "  %-14s %14s %12s %14s\n", "baseline",    commas(bt), commas(bp), commas(bu)
    printf "  %-14s %14s %12s %14s\n", "precompiled", commas(pt), commas(pp), commas(pu)
    printf "  %-14s %13.1fx %12s %13.1fx\n", "speedup", bt/pt, "", bu/pu
    printf "\n"
    printf "  user cycles is the figure that matters — total is padded up to a\n"
    printf "  power-of-two segment size, and the precompiled run already sits on\n"
    printf "  the floor, so its total understates the gain.\n\n"
    printf "  against MAX_NUM_CYCLES_PUBLIC_EXECUTION = %s, ignoring the fixed\n", commas(budget)
    printf "  per-transaction overhead a real program also pays:\n"
    printf "    baseline     %14s user/sig  ->  up to %d signatures per execution\n", commas(bu), int(budget/bu)
    printf "    precompiled  %14s user/sig  ->  up to %d signatures per execution\n", commas(pu), int(budget/pu)
    printf "\n"
}'
