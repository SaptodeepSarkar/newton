#!/usr/bin/env bash
# Publication gate. Decides whether a built package may be published.
# It checks for a signature. It never creates one, and it never reports a
# signature it could not verify.
#
# usage: sign-check.sh <package> <linux|macos|windows>
# exit:  0 publishable, 1 not publishable, 2 usage error
set -uo pipefail

usage() {
    echo "usage: sign-check.sh <package> <linux|macos|windows>" >&2
    exit 2
}

[ $# -eq 2 ] || usage
pkg=$1
platform=$(printf '%s' "$2" | tr '[:upper:]' '[:lower:]')
[ -e "$pkg" ] || { echo "sign-check: no such package: $pkg" >&2; exit 2; }

missing=""
warning=""
miss() { missing="${missing}$1"$'\n'; }

case "$platform" in
macos|mac|darwin)
    case "$pkg" in
        *.dmg|*.app) ;;
        *) miss "macOS: package is not a .dmg or .app, so it cannot carry a Developer ID signature" ;;
    esac
    if ! command -v codesign >/dev/null 2>&1; then
        miss "macOS: Developer ID signature unverified (codesign not installed)"
    else
        codesign --verify --deep --strict "$pkg" >/dev/null 2>&1 \
            || miss "macOS: no valid code signature (codesign --verify failed)"
        codesign -dv --verbose=4 "$pkg" 2>&1 | grep -q "Authority=Developer ID Application" \
            || miss "macOS: signature is not a Developer ID Application certificate"
    fi
    ;;
windows|win|win32)
    target=$pkg
    tmp=""
    case "$pkg" in
        *.zip)
            if ! command -v unzip >/dev/null 2>&1; then
                target=""
                miss "Windows: cannot inspect the archive for a signed executable (unzip not installed)"
            else
                tmp=$(mktemp -d)
                if unzip -qq -o "$pkg" -d "$tmp" >/dev/null 2>&1; then
                    target=$(find "$tmp" -type f -iname '*.exe' -print -quit)
                else
                    target=""
                fi
                [ -n "$target" ] || miss "Windows: archive contains no .exe that can be Authenticode-signed"
            fi
            ;;
        *.exe|*.msi) ;;
        *) miss "Windows: package is not a .exe, .msi or .zip" ;;
    esac
    if [ -n "$target" ]; then
        if command -v osslsigncode >/dev/null 2>&1; then
            osslsigncode verify -in "$target" >/dev/null 2>&1 \
                || miss "Windows: no Authenticode signature on $(basename "$target")"
        elif command -v pwsh >/dev/null 2>&1; then
            status=$(pwsh -NoProfile -Command "(Get-AuthenticodeSignature '$target').Status" 2>/dev/null)
            [ "$status" = "Valid" ] \
                || miss "Windows: no valid Authenticode signature on $(basename "$target") (status: ${status:-unknown})"
        else
            miss "Windows: Authenticode signature unverified (osslsigncode/pwsh not installed)"
        fi
    fi
    [ -n "$tmp" ] && rm -rf "$tmp"
    ;;
linux)
    case "$pkg" in
        *.deb|*.tgz|*.tar.gz)
            warning="Linux: no repository metadata, so apt/dnf cannot check origin; publish a SHA-256 next to it"
            ;;
        *) miss "Linux: package is not a .deb or .tgz" ;;
    esac
    ;;
*)
    usage
    ;;
esac

if [ -n "$missing" ]; then
    printf '%s' "$missing" >&2
    echo "sign-check: NOT publishable: $pkg" >&2
    exit 1
fi

[ -n "$warning" ] && echo "sign-check: warning: $warning" >&2
echo "sign-check: publishable: $pkg"
exit 0
