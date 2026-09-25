#!/bin/sh
# Assemble Disktree.app from the release binary.
#
#   packaging/bundle.sh                 # target/release/Disktree.app
#   packaging/bundle.sh path/to/binary  # a binary built somewhere else
#
# Honours CARGO_TARGET_DIR. The bundle is ad-hoc signed: Apple Silicon refuses
# to run unsigned code at all, and there is no Developer ID here, so this is
# what a local build gets. A downloaded copy still carries the quarantine
# xattr; the README says how to clear it.
#
# The signature carries an explicit designated requirement, the bundle
# identifier alone. Without one, an ad-hoc signature's requirement is the
# code hash of that exact build, and a privacy grant (Full Disk Access) is
# stored against the requirement: the next `make install` would then be a
# different app to macOS, still ticked in System Settings but refused. With
# the identifier as the requirement every build of Disktree is the same
# app to the privacy protection, and the grant survives a rebuild.
set -eu

here=$(cd "$(dirname "$0")/.." && pwd)
target=${CARGO_TARGET_DIR:-"$here/target"}
release="$target/release"
binary=${1:-"$release/disktree"}
app=${BUNDLE_DIR:-"$release"}/Disktree.app

if [ ! -x "$binary" ]; then
    echo "bundle.sh: no release binary at $binary (run make build)" >&2
    exit 1
fi

version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$here/Cargo.toml" | head -1)

# Start clean so a renamed or removed file never lingers in the bundle.
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"

cp "$binary" "$app/Contents/MacOS/disktree"
chmod 755 "$app/Contents/MacOS/disktree"
sed -e "s|@VERSION@|$version|g" "$here/packaging/Info.plist.in" \
    > "$app/Contents/Info.plist"
cp "$here/assets/disktree.icns" "$app/Contents/Resources/disktree.icns"
# Type and creator; the creator is unregistered, hence the four ?s.
printf 'APPL????' > "$app/Contents/PkgInfo"

plutil -lint "$app/Contents/Info.plist" >/dev/null
# --force replaces the linker's ad-hoc signature on the binary, --deep signs
# the bundle around it. Identity "-" is ad-hoc; -r sets the designated
# requirement explained above.
codesign --force --deep -s - \
    -r='designated => identifier "dev.disktree.Disktree"' "$app"
codesign --verify --deep --strict "$app"

echo "bundled Disktree.app $version at $app"
