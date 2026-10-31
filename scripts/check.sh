#!/usr/bin/env bash
# Everything CI runs, in the same order. Run this before a push.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
build="${BUILD_DIR:-$root/build}"

cmake -S "$root" -B "$build" -G Ninja -DCMAKE_BUILD_TYPE=Debug
cmake --build "$build" --parallel

# Includes qml_smoke, which loads the UI and walks all five states headless.
ctest --test-dir "$build" --output-on-failure

cd "$root/core"
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test

echo "all green"
