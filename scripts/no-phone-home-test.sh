#!/usr/bin/env bash
# Proves the phone-home check still catches a foreign host and still allows
# Google and loopback. Any file with printable text stands in for a binary.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
check="$here/no-phone-home.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fail() { echo "no-phone-home-test: FAIL: $*" >&2; exit 1; }

bad="$tmp/bad"
good="$tmp/good"
printf 'NEWTON\000\001https://example.com/updates\000' > "$bad"
printf 'NEWTON\000\001https://oauth2.googleapis.com/token\000http://127.0.0.1:51234/callback\000' > "$good"

set +e
out="$("$check" "$bad" 2>&1)"
code=$?
set -e
[[ $code -eq 1 ]] || fail "foreign host gave exit $code, wanted 1"
grep -q 'example.com' <<< "$out" || fail "offending string not printed"

"$check" "$good" >/dev/null 2>&1 || fail "Google and loopback hosts were rejected"

echo "no-phone-home: ok"
