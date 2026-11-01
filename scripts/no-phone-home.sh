#!/usr/bin/env bash
# Fail if a built binary names a host that is not Google's or loopback.
# A binary that cannot name an update server cannot phone home. Static check;
# phase 29 adds the socket-level gate on top of this.
#
#   scripts/no-phone-home.sh path/to/newton
#   exit 0 clean, 1 foreign host found, 2 no binary
set -uo pipefail

bin="${1:-}"
if [[ -z "$bin" || ! -f "$bin" ]]; then
    echo "no-phone-home: no binary at ${bin:-<no path given>}" >&2
    exit 2
fi

if command -v strings >/dev/null 2>&1; then
    text="$(strings -a "$bin")"
else
    text="$(grep -ao '[[:print:]]\{4,\}' "$bin")"
fi

# Match URLs only: scheme://host or //host, optional port. A bare dotted word
# is not a request and would flag filenames like model.safetensors.
urls="$(printf '%s\n' "$text" \
    | grep -Eo '(https?:)?//[A-Za-z0-9._-]+(:[0-9]+)?' \
    | tr 'A-Z' 'a-z' | sort -u || true)"

status=0
while IFS= read -r url; do
    [[ -z "$url" ]] && continue
    host="${url#*//}"
    host="${host%%:*}"
    case "$host" in
        127.0.0.1|googleapis.com|*.googleapis.com|accounts.google.com) ;;
        *)
            echo "no-phone-home: foreign host: $url" >&2
            status=1
            ;;
    esac
done <<< "$urls"

(( status == 0 )) && echo "no-phone-home: ok"
exit "$status"
