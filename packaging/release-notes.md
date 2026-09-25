`Disktree-<version>-aarch64-apple-darwin.zip` is the app for Apple silicon
Macs; the `.sha256` beside it is its checksum (`shasum -a 256 -c`). There
is no Intel build: an Intel Mac builds from source with `make install`.

The app is signed ad-hoc, not with a Developer ID, so Gatekeeper will
refuse a downloaded copy until its quarantine flag is cleared:

```sh
unzip Disktree-*.zip
xattr -dr com.apple.quarantine Disktree.app
mv Disktree.app /Applications/
```
