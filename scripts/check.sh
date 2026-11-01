#!/usr/bin/env bash
# Everything CI runs, in the same order. Run this before a push.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
build="${BUILD_DIR:-$root/build}"

cmake -S "$root" -B "$build" -G Ninja -DCMAKE_BUILD_TYPE=Debug
cmake --build "$build" --parallel

# Includes qml_smoke, which loads the UI and walks all five states headless.
ctest --test-dir "$build" --output-on-failure

for crate in core memory google crash tools/models; do
    cd "$root/$crate"
    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo test
done

# Packaging and privacy guards: no signature, no publish; no foreign host, no ship.
bash "$root/installer/sign-check-test.sh"
bash "$root/scripts/no-phone-home-test.sh"
if [ -x "$build/src/newton" ]; then
    bash "$root/scripts/no-phone-home.sh" "$build/src/newton"
fi

echo "all green"
