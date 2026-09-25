# disktree — agent guide

A GPUI treemap explorer for disk usage on macOS, shipped as `Disktree.app`.
Read `README.md` for the product; this file is the working contract.

## What this is

Find what is eating a volume, mark paths for removal, review the list, and
remove it — with the volume's free space live on screen the whole time. Two
phases: explore (treemap, breadcrumbs, the selection and status lines) and
review (the marked list, the removal mode, the confirmation). Marking is
never destructive.

The macOS port of the original Linux/Omarchy disktree (imported at 0.9.1).
The theme is a local module with AppKit-style light and dark palettes that
follow the window appearance; there is no gpui-omarchy.

## Commands

```sh
make help                       # this list
make build                      # release build
make bundle                     # Disktree.app in target/release
make open                       # build, bundle and open Disktree.app
make run                        # build and run the bare binary on $HOME
make install                    # Disktree.app into /Applications
make install-cli                # symlink disktree into /usr/local/bin
make uninstall                  # remove what install put there
make lint                       # rustfmt --check, then clippy --all-targets -D warnings
make test                       # core and window-harness tests
make ci                         # lint, then test
make fmt                        # format in place
make icon                       # re-render assets/disktree.icns from the SVG
make dist                       # release zip and .sha256 in dist/
make release                    # gh release create from dist/ for the tag
make clean                      # cargo clean
```

`make install` is the supported way to put this on a machine:
`packaging/bundle.sh` wraps the release binary in `Disktree.app` with
`packaging/Info.plist.in` rendered (the crate version, the privacy usage
strings, the `public.folder` document type with alternate rank so the app
is offered for folders but never made their default), ad-hoc signs it
(Apple silicon runs nothing unsigned and there is no Developer ID), and
`ditto` copies it to `/Applications`, or `~/Applications` when that is not
writable. `assets/disktree.icns` is committed, so an install needs no SVG
toolchain; `make icon` regenerates it from `assets/disktree.svg` with `sips`
and `iconutil` after the SVG changes. `make run` is the bare binary, so
panics and stdout land in the terminal; `make open` is the bundle, the way a
user launches it.

Releases are built on this Mac, for Apple silicon only: `make dist` bundles
the release build and writes `dist/Disktree-<version>-aarch64-apple-darwin.zip`
with its `.sha256` (`packaging/dist.sh`); `make release` publishes those two
files with `gh release create v<version>` and
`packaging/release-notes.md`, and refuses unless the tag `v<version>` from
`Cargo.toml` exists and points at `HEAD`.

`cargo xtask lint` is the gate, and it runs here: there is no CI service
behind it. It must be green before anything is called done, and it must not
fix anything: a red run reports the diff and fails, so what it says is what
would have been shipped.

## House rules

* **Strict lints from omatrack's style.** `clippy::all` and `clippy::pedantic`
  are errors, `clippy::nursery` warns, and `-D warnings` promotes the rest. Every
  deliberate exception is listed with its reason in the workspace `Cargo.toml`
  or as an `#[allow(..., reason = "...")]` on the item — never silently.
  `unsafe_code` is denied; the four FFI calls the port needs (`fsid`
  packing, `statfs`, `getfsstat`, the dataless-file policy) each carry an
  allow with the reason.
* **80 columns**, 4-space indent, by `rustfmt.toml`. Only stable rustfmt
  options, because the gate runs on stable; unstable options would be ignored
  with a warning instead of applying.
* **Comments say why.** The code says what. Any non-obvious number, ordering or
  boundary deserves the reason next to it — the `sys/mount.h` and
  `sys/stat.h` bits mirrored in `volumes.rs` and `removal.rs` are the model.
* **Tests live beside the promise they make.** `disktree-core` tests size
  accounting, layout, the mount rules and deletion guards against real
  temporary trees and a fixture of a real machine's mount table;
  `disktree-app/src/tests.rs` drives the real window harness — draw a frame,
  press keys — so a screen that panics while painting fails a test.
* **Never delete anything outside a marked path.** See `removal.rs`; the guards
  are load-bearing and are covered by tests.
* **Describe macOS in its own words.** Trash, Finder, Full Disk Access,
  System Settings, `⌘`: the user-facing strings and the docs name what the
  user sees on this platform, not a Linux equivalent.

## Invariants

1. **Sizes come from `st_blocks * 512` unless apparent size was asked for.**
   That is the number that comes back when a file is deleted. On APFS that
   means compressed, sparse and dataless files count what they spend, and a
   clone counts its full size because nothing else is observable.
2. **`own_bytes`/`own_files` are derived, never tracked.** `tree::aggregate`
   computes the totals from the children. Hardlink de-duplication rewrites a
   leaf's weight and re-aggregates; anything that patches `bytes` directly will
   be overwritten.
3. **A directory is only built when its own scan *and* every subdirectory task
   has finished.** That is the `+1` sentinel in `PendingDir::pending`. Building
   early silently drops whole subtrees — it has happened once.
4. **Only paths under the scanned root may be removed**, and mount points, the
   root, the home directory and symlink targets are refused. On top of that
   `removal.rs` refuses by the kernel's own flags first (`SF_RESTRICTED` is
   SIP, `SF_NOUNLINK` is a firmlink root), then read-only volumes, the
   system trees, `/private/etc`, `~/Library` and its managed subdirectories,
   and a volume's Trash. Every guard compares paths in one lexical form —
   `/System/Volumes/Data` stripped, `/etc`, `/var`, `/tmp` expanded into
   `/private` — so a path cannot dodge a rule by arriving under its other
   name.
5. **Marks are keyed by absolute path**, not tree position, so they survive a
   re-scan; `Marks::refresh` re-reads their sizes and drops what is gone.
6. **The treemap is painted, not composed of elements.** Thousands of
   rectangles belong in one canvas callback; labels are shaped there too so they
   clip to their own tile.
7. **Tile crumbs are absolute.** `treemap::layout` takes the drawn node's
   crumbs and every tile extends them, so a tile resolves from the scanned
   root at any depth. Relative crumbs look right at `~` and silently point
   at other directories after descending — including for marks. Any code that
   turns a path into crumbs walks from the scanned root, too.
8. **The view transform is the only thing zoom changes.** Layout runs in
   base-space pixels and is cached; `screen = (base - origin) * scale`.
9. **The status bar never claims a saving it cannot measure.** Projections come
   from marked bytes; the final number comes from `statfs` before and after
   (`statfs`, not `statvfs`: on macOS the latter keeps 32-bit counts and
   saturates on a volume of a few terabytes). Purgeable space is shown on
   its own line and never added to a projection: the OS may give it back,
   disktree cannot.
10. **Volume identity is `f_fsid` from the mount table, never `st_dev`.** The
    boot disk is an APFS volume group whose sealed system volume and data
    volume report the *same* `st_dev`, so the classic "stay on the root's
    device" rule walks the data volume twice — through the firmlinks and
    again under `/System/Volumes/Data`. `f_fsid` tells them apart; the group
    is walked once from `/`, `/System/Volumes` is skipped, and
    `volume_root_for($HOME)` is `/`. Both spellings of a firmlinked mount
    point are excluded, since the walk may reach either.
11. **Excluded mount points are checked by path before any stat**, so an
    automount trigger (`autofs`, `MNT_AUTOMOUNTED`) is never fired: reading
    it would *create* the mount, and a scan must not mount a NAS to measure
    it. The exclusion list is computed from the mount table up front and
    consulted on the path alone.
12. **A package bundle is removed as a unit.** An `.app`, a `.photoslibrary`
    or any other bundle `classify::is_package_name` recognises is one thing
    to the Finder and to the app that owns it; a path inside one is refused
    with "mark the package itself". The scanned root is exempt (scanning
    inside a package is the user asking to see its pieces) except for a
    Photos library, whose database and originals must never be split.
13. **The scan never materialises dataless (iCloud) files.** `scan.rs` sets
    the process I/O policy to leave dataless placeholders alone before the
    walk starts, so measuring `Mobile Documents` or a File Provider folder
    downloads nothing; a placeholder is `st_blocks` zero, which is exactly
    what it costs. Any new code that reads file contents during a scan must
    keep that policy in force.

## Where changes belong

| change | where |
| --- | --- |
| measurement, filtering, parallelism, the dataless policy | `crates/disktree-core/src/scan.rs` |
| the mount table, the boot volume group, what a scan skips | `crates/disktree-core/src/volumes.rs` |
| what a node is, or a derived total | `crates/disktree-core/src/tree.rs` |
| tile geometry, nesting, the merged tail | `crates/disktree-core/src/treemap.rs` |
| a directory name → category or reclaim, package names | `crates/disktree-core/src/classify.rs` |
| anything that deletes, or refuses to; the Finder Trash | `crates/disktree-core/src/removal.rs` |
| free space, purgeable space and projections | `crates/disktree-core/src/space.rs` |
| a key, a menu action, a screen transition, a mark | `crates/disktree-app/src/state.rs` |
| spacing, type and size | `crates/disktree-app/src/ui.rs` — tokens only, no `px` in layout |
| the light and dark palettes, appearance, radii, fonts | `crates/disktree-app/src/theme.rs` |
| buttons, choice groups, dialogs, tooltips, keycaps | `crates/disktree-app/src/controls.rs` |
| the mosaic's painting or labels | `crates/disktree-app/src/treemap_view.rs` |
| layout of a screen | `crates/disktree-app/src/views.rs` |
| category colours derived from the theme | `crates/disktree-app/src/palette.rs` |
| the bundle, `Info.plist`, the icon | `packaging/` — `bundle.sh`, `Info.plist.in`, `icon.sh` |

## Verification expectations

* Size accounting, hardlinks, symlinks, hidden entries, depth limits, the
  removal guards and squarified layout are covered by `disktree-core` tests
  against real temporary trees.
* The mount rules (`volumes.rs`) are pure functions over a `Mount` table and
  are tested against a fixture of a real machine: the whole-disk scan skips
  `/System/Volumes` and every other mount, an external disk stays inside
  its own fsid group, automounts are excluded by path.
* The removal guards are tested on the real system layout: SIP flags,
  firmlinked spellings, `/usr/local` allowed under `/usr` refused,
  `~/Library` refused while `~/Library/Caches/x` is not, packages as units.
* Moving to the Trash is tested for real: a uniquely named temporary file
  goes through `NSFileManager`, must be gone from where it was, and the
  path it landed on is checked; the test then tidies it out of the Trash,
  best effort, since listing `~/.Trash` needs Full Disk Access.
* `scan::tests::whole_disk_smoke` walks the whole boot disk and is
  `#[ignore]`d; run it with `cargo test -p disktree-core -- --ignored` after
  touching `volumes.rs` or the exclusion path in `scan.rs`.
* The screens are covered by window-harness tests that draw frames and press
  keys, including one that marks a directory, confirms the removal and checks
  the files are gone while unmarked neighbours are untouched, one that
  draws the window under both palettes, and one that checks every
  `WindowAppearance` (vibrant or not) picks the matching palette.
* Rendering was verified by those tests and by running the app against a real
  home directory in light and dark; it has not been eyeballed at every
  interface zoom.
