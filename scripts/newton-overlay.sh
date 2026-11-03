#!/bin/sh
# Start the puck: the headless daemon plus the layer-shell overlay that draws it.
set -e
here=$(cd "$(dirname "$0")/.." && pwd)
"$here/build/src/newton" --daemon &
exec quickshell -c newton
