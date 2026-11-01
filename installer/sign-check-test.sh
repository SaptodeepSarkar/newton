#!/usr/bin/env bash
# Tests for sign-check.sh. Fake packages only; needs no signing tool.
set -uo pipefail

here=$(cd "$(dirname "$0")" && pwd)
check="$here/sign-check.sh"
tmp=$(mktemp -d) || exit 1
trap 'rm -rf "$tmp"' EXIT

failed=0
out=""
code=0

fail() { echo "FAIL: $*" >&2; failed=$((failed + 1)); }
run() { out=$("$check" "$@" 2>&1); code=$?; }
want() {
    [ "$code" -eq "$1" ] || fail "$2: exit $code, wanted $1 (output: ${out//$'\n'/ | })"
}
says() {
    grep -qi -- "$1" <<<"$out" || fail "$2: output should mention '$1' (output: ${out//$'\n'/ | })"
}
never_says_signed() {
    grep -Eqi 'valid signature|is signed|signature is valid' <<<"$out" \
        && fail "$1: claims a signature it did not verify (output: ${out//$'\n'/ | })"
    return 0
}

# --- usage errors -----------------------------------------------------------
run;                     want 2 "no arguments"
run "$tmp/x.deb";        want 2 "one argument"
run a b c;               want 2 "three arguments"
run a beos;              want 2 "unknown platform"
run "$tmp/nope.deb" linux; want 2 "missing package"

# --- linux ------------------------------------------------------------------
printf 'fake deb\n' >"$tmp/newton.deb"
run "$tmp/newton.deb" linux; want 0 "linux .deb"
says 'warning' "linux .deb"
says 'SHA-256' "linux .deb"

printf 'fake tgz\n' >"$tmp/newton.tgz"
run "$tmp/newton.tgz" LINUX; want 0 "linux .tgz, uppercase platform"

printf 'fake zip\n' >"$tmp/newton.zip"
run "$tmp/newton.zip" linux; want 1 "linux .zip is not a linux package"

# --- macos ------------------------------------------------------------------
printf 'fake dmg\n' >"$tmp/newton.dmg"
run "$tmp/newton.dmg" macos; want 1 "unsigned macos .dmg"
says 'macOS' "unsigned macos .dmg"
never_says_signed "unsigned macos .dmg"

printf 'fake tar\n' >"$tmp/newton.tar.gz"
run "$tmp/newton.tar.gz" macos; want 1 "macos package of the wrong type"
says '\.dmg or \.app' "macos wrong type"

# --- windows ----------------------------------------------------------------
printf 'MZ fake\n' >"$tmp/newton.exe"
run "$tmp/newton.exe" windows; want 1 "unsigned windows .exe"
says 'Authenticode' "unsigned windows .exe"
never_says_signed "unsigned windows .exe"

if command -v zip >/dev/null 2>&1; then
    mkdir -p "$tmp/zipin"
    printf 'MZ fake\n' >"$tmp/zipin/newton.exe"
    (cd "$tmp/zipin" && zip -q "$tmp/newton-win.zip" newton.exe)
    run "$tmp/newton-win.zip" windows; want 1 "unsigned windows .zip"
    never_says_signed "unsigned windows .zip"
fi

# --- honest when the checking tool is absent --------------------------------
if ! command -v codesign >/dev/null 2>&1; then
    run "$tmp/newton.dmg" macos; want 1 "macos without codesign"
    says 'unverified' "macos without codesign"
fi
if ! command -v osslsigncode >/dev/null 2>&1 && ! command -v pwsh >/dev/null 2>&1; then
    run "$tmp/newton.exe" windows; want 1 "windows without a verifier"
    says 'unverified' "windows without a verifier"
fi

[ "$failed" -eq 0 ] || { echo "sign-check: $failed failure(s)" >&2; exit 1; }
echo "sign-check: ok"
