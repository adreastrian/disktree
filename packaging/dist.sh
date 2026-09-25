#!/bin/sh
# Zip Disktree.app for a GitHub release, with its checksum beside it.
#
#   packaging/dist.sh    # dist/Disktree-<version>-aarch64-apple-darwin.zip
#                        # dist/Disktree-<version>-aarch64-apple-darwin.zip.sha256
#
# Runs after packaging/bundle.sh (`make dist` does both) and honours
# CARGO_TARGET_DIR the same way. Releases are built on an Apple silicon Mac
# only, so the target in the name is fixed and the script refuses to run
# anywhere else rather than label an Intel build wrongly.
set -eu

here=$(cd "$(dirname "$0")/.." && pwd)
target=${CARGO_TARGET_DIR:-"$here/target"}
app="$target/release/Disktree.app"
dist="$here/dist"
triple=aarch64-apple-darwin

if [ "$(uname -m)" != "arm64" ]; then
    echo "dist.sh: releases are built on Apple silicon only" >&2
    exit 1
fi
if [ ! -d "$app" ]; then
    echo "dist.sh: no bundle at $app (run make bundle)" >&2
    exit 1
fi

version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$here/Cargo.toml" | head -1)
name="Disktree-$version-$triple"

mkdir -p "$dist"
rm -f "$dist/$name.zip" "$dist/$name.zip.sha256"
# ditto keeps the signature and resource forks that zip(1) would drop;
# --keepParent puts Disktree.app at the top of the archive, so `unzip`
# yields the bundle and not its contents.
ditto -c -k --keepParent "$app" "$dist/$name.zip"
# Written from inside dist/ so the checksum file names the zip without a
# path and `shasum -a 256 -c` works from wherever the two were downloaded to.
(cd "$dist" && shasum -a 256 "$name.zip" > "$name.zip.sha256")

echo "dist: $dist/$name.zip"
echo "      $dist/$name.zip.sha256"
