#!/bin/sh
# Render assets/disktree.svg into assets/disktree.icns.
#
#   packaging/icon.sh              # writes assets/disktree.icns
#   packaging/icon.sh out.icns     # or somewhere else
#
# Uses only what macOS ships: sips rasterizes the SVG at each size an iconset
# wants, and iconutil packs them. The .icns is committed, so this only needs
# running after the SVG changes; a plain `make install` never needs it.
set -eu

here=$(cd "$(dirname "$0")/.." && pwd)
svg="$here/assets/disktree.svg"
out=${1:-"$here/assets/disktree.icns"}

work=$(mktemp -d "${TMPDIR:-/tmp}/disktree-icon.XXXXXX")
trap 'rm -rf "$work"' EXIT
set="$work/disktree.iconset"
mkdir -p "$set"

# iconutil wants exactly these names; the @2x file of one size is the same
# pixels as the plain file of the next, but both must be present.
for size in 16 32 128 256 512; do
    double=$((size * 2))
    sips -s format png -z "$size" "$size" "$svg" \
        --out "$set/icon_${size}x${size}.png" >/dev/null
    sips -s format png -z "$double" "$double" "$svg" \
        --out "$set/icon_${size}x${size}@2x.png" >/dev/null
done

iconutil -c icns "$set" -o "$out"
echo "wrote $out"
