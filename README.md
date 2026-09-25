# disktree

![Disktree in the dark appearance: the trail Macintosh HD / Users / Arif in the title bar beside the Size, Files and Age choice, the Hidden files and Apparent size toggles and the depth stepper; a home directory as a treemap coloured by kind of data with reclaimable space hatched; and the selection, Worth a look and the volume's free space in the side panel](assets/screenshot.png)

This is the Apple silicon port of [tobi/disktree](https://github.com/tobi/disktree),
the Linux/Omarchy original. Find what is filling a disk, mark what should
go, and remove it, with the volume's free space in view the whole time.

disktree is a treemap for macOS. It scans your home directory by default,
draws every directory as a nested mosaic sized by what it really costs on disk,
and lets you walk into it with the keyboard or the mouse. Mark as much as you
like; nothing happens until you review the list and commit, and the permanent
path always asks first.

Built with [GPUI](https://gpui-kit.com/), drawn in the system font with the
colours macOS draws its own windows in, so it sits beside the Finder and
follows the system's light or dark appearance as it changes.

## Install

Download `Disktree-<version>-aarch64-apple-darwin.zip` from the
[latest release](https://github.com/adreastrian/disktree/releases/latest);
it is built for Apple silicon only, and the `.sha256` beside it is its
checksum (`shasum -a 256 -c`). The app is signed ad-hoc, not with a
Developer ID, so Gatekeeper refuses a downloaded copy until its quarantine
flag is cleared:

```sh
unzip Disktree-*.zip
xattr -dr com.apple.quarantine Disktree.app
mv Disktree.app /Applications/
```

Or build it:

```sh
git clone https://github.com/adreastrian/disktree
cd disktree
make install
```

`make install` builds a release binary, wraps it in an ad-hoc signed
`Disktree.app` and copies the bundle to `/Applications` (or
`~/Applications` when that is not writable, so no root is needed). The bundle is
what gives disktree its name and icon, a place in Spotlight and Launchpad,
a stable identity for the privacy prompts, and an entry in the Finder's
**Open With** for a folder (it adds a handler; it never becomes the default).

- `make open` builds, bundles and opens the app;
- `make install-cli` symlinks `disktree` into `/usr/local/bin` (or
  `CLI_DIR=$HOME/bin` for another directory on your `PATH`), so the command
  and the app are always the same build;
- `make uninstall` removes exactly what was installed.

You need macOS 12 or newer on Apple silicon; Rust 1.97 or newer, from
[rustup](https://rustup.rs); and the Xcode Command Line Tools
(`xcode-select --install`) for the linker and the system frameworks. An
Intel Mac gets no release build: nothing in the crates or the packaging is
tied to the architecture, so `make install` should work there too, but it
is not tested.

### Full Disk Access

macOS refuses to let an app read `~/Library/Mail`, `~/Library/Messages`,
`~/Library/Safari`, most other apps' `Containers`, `~/.Trash` and a few more
until the app has been granted **Full Disk Access**. Without it those
directories are counted as *not permitted* in the totals under the title
bar rather than measured, and a home directory can look tens of gigabytes
smaller than it is. disktree notices and shows a notice with a button
that opens **System Settings ▸ Privacy & Security ▸ Full Disk Access**;
add Disktree there, then press `⌘ R` to scan again. The Desktop, Documents
and Downloads folders ask once, the first time they are read, like any
other app.

If Disktree is already ticked there and the notice still shows, turn the
switch off and on again (or remove Disktree from the list and add it back).
macOS stores the grant against the app's code signature, and an ad-hoc
signed build used to be a different app to it after every `make install`.
The bundle now signs with a stable requirement so a grant survives a
rebuild, but a grant made for an older build has to be made once more.

A few places refuse every process, Full Disk Access or not: the system's
*data vaults*, such as `/private/var/protected` and the daemons' caches
under `/Library/Caches/com.apple.*`. They are counted as *unreadable*
rather than *not permitted*, so the notice goes away once the grant is
in place.

## Use

Open Disktree from Launchpad or Spotlight, choose a folder with `⌘ O`, or
pick **Open With ▸ Disktree** on a folder in the Finder. From a terminal,
after `make install-cli`:

```sh
disktree            # scan the home directory
disktree --disk     # the whole disk it lives on
disktree ~/src      # or any directory
disktree --help     # apparent size, follow links, skip hidden, cross
                    # filesystems, depth, rank by file count
```

### The screen

- **Title bar:** the window's own, beside the traffic lights: the logo, the
  trail from the top of the volume, then what is measured: **Size**,
  **Files** or **Age**, **Hidden files**, **Apparent size**, and the
  **Depth** stepper. In the tree a crumb goes there, and its ▾ lists its
  siblings, largest first with their share and size, to jump sideways
  (arrows and Enter work too); the scanned root has no siblings in the
  tree, so no ▾. Above the scanned root a crumb is dimmer, and clicking it
  widens the scan to there (see below). The first crumb's ▾ lists the
  Mac's volumes, each with its free space: choosing one scans it from its
  top. That is the way to `/Volumes/Space` or a mounted share, since a
  scan never crosses into another volume and the trail never climbs above
  the one it is on.
- **Under it:** the scan totals, with what could not be read counted
  apart; the filter when one is typed; and the legend.
- **Mosaic:** colour is the *kind* of data (code, agent scratch,
  toolchains, synced files, git, media, documents, caches) at one muted
  level, lighter with depth. A diagonal hatch is space that can be had back
  (caches, sync history, package stores, build output), independent of
  colour. Top-level directories carry a strip of their colour and a name
  band; deeper open directories a slim label row. In **Age** mode colour is
  the last write instead, from this week to older.
- **Panel:** the selection (its size set large, share of the scan, files,
  last write, and for a checkout what git says: changes, stashes, unpushed
  commits); *Worth a look*, the largest things that could plausibly go;
  what is marked; and the disk, free now and after the marks, with the way
  to the review screen. Drag its left edge to resize it; double-click the
  edge to reset; `p` hides it.
- **Bottom line:** the keys that matter here, and how long the scan took.

One colour is kept apart: amber marks the selection, the main action, and
what can be had back.

The kinds come from directory names and a few shapes (a bare git repository,
`target` beside a `Cargo.toml`, `Devices` only under `CoreSimulator`). On a
Mac the interesting names live under `~/Library` (`Caches`, `Logs`,
`Containers`, `Application Support`, `Mobile Documents`, `CloudStorage`,
`MobileSync`) and under Xcode's `DerivedData`, `Archives`,
`DeviceSupport` and `CoreSimulator`; Homebrew, CocoaPods, SwiftPM and
Docker-style tools are known too. See `crates/disktree-core/src/classify.rs`.

### Marking

Space, X, Enter and the arrows act on the tile under the mouse if the mouse
moved last, and on the keyboard selection after you use an arrow or Tab.

A marked tile takes the danger colour, and so does everything inside it:
removing a directory takes its contents with it. Marking a directory absorbs
any marks already inside it, and something inside a marked directory cannot be
marked or kept on its own; its panel offers to unmark the directory instead.
Marking is reversible, press it again, and the saving is never counted twice.

A package (an `.app`, a `.photoslibrary`, any bundle the Finder shows as one
thing) is one thing here too: a file inside it cannot be marked on its own,
and the panel says to mark the package itself.

### Zooming and going in

Scroll to magnify toward the pointer. The wheel magnifies until the directory
under the pointer fills the view, and the next notch goes into it, in one
continuous motion, with the directory's contents growing into place. Scroll the
other way to come back out. Enter goes into the selected directory at any
depth, and Backspace goes up one level (Escape first clears the filter,
then the selection, then goes up). `=` and `-` magnify and shrink without
going in; `0` resets.

### Removing

`c` (or **Review…**) opens the list of everything marked. Unmark anything
there, then choose:

- **Move to Trash** is the default. It goes through the Finder's own
  mechanism (`NSFileManager`), so each item lands in the Trash of the volume
  it is on: `~/.Trash` for the boot disk, the disk's own `.Trashes` for an
  external one, with no prompt and no shell. Recoverable until the Trash is
  emptied, so it commits directly. A volume without a Trash (some network
  shares) says so; use permanent removal there.
- **Delete permanently** has `rm -rf` semantics. It always asks first, in a dialog
  that names what goes and how much comes back.

When it finishes, disktree scans again so the numbers on screen match the disk,
and shows how much free space was actually gained.

## Keys

Disktree has a normal macOS menu bar, so every `⌘` shortcut below is also a
menu item.

| key | does |
| --- | --- |
| `⌘ O` | open a folder to scan |
| `⌘ R` | scan again from the same root |
| `⌘ F` | filter by name (same as `/`) |
| `⌘ =` `⌘ -` `⌘ 0` | interface zoom |
| `⌘ ⇧ D` | the whole disk (same as `g`) |
| `⌘ /` | every key (same as `?`) |
| `⌘ W` | close the window |
| `⌘ Q` | quit |
| `space` / `x` | mark or unmark the tile you point at |
| `⌘`-click, middle-click | mark without moving the selection |
| `enter` | open that directory, at any depth |
| `⌫` / `u` | go up one directory |
| `esc` | clear the filter, then the selection, then go up one directory |
| `←` `↑` `↓` `→` / `h` `j` `k` `l` | move between tiles at this level |
| `tab` / `⇧tab` | next or previous largest sibling |
| wheel | zoom toward a directory, then go into it |
| trackpad | two fingers pan; `⌘` two fingers zoom |
| `shift`-scroll | pan the magnified view |
| `[` `]` | draw fewer or more levels at once |
| `-` `=` `0` | shrink, magnify, reset the view |
| `/` / `s` | filter by name: only matches keep their colour; `enter` shows only them, `esc` clears |
| `c` | review the marked list |
| `t` | rank by size, by file count, or by age |
| `d` | disk usage or apparent size |
| `i` | include or skip hidden entries |
| `r` | scan again |
| `g` | the whole disk |
| `p` | show or hide the side panel |
| `?` | every key |
| `q` | quit |

On the review screen: `m` trash, `p` permanent, `!` unmark all, `enter`
commits, `esc` goes back.

## What it measures

- **Disk usage** by default: `st_blocks × 512`, the number `du` reports and the
  space that actually comes back when a file is deleted. Apparent size (what
  `ls -l` shows) is one toggle away.
- **What APFS actually spends.** A compressed file counts its compressed
  blocks, a sparse file only the blocks it has, and an evicted iCloud file
  zero. The one thing the kernel does not tell anyone is a clone (`cp -c`,
  the Finder's Duplicate): each copy reports its full size while they share
  the blocks, so deleting one copy frees nothing.
- **Purgeable space** (local Time Machine snapshots, evicted iCloud
  content, caches the system will drop on demand) is shown on its own line
  in the disk section, because the Finder counts it as free and `df` does
  not. It is never added to a projection: only bytes disktree can delete
  are promised.
- **Hardlinks once.** Two names for one inode cost one file.
- **Hidden entries included**, because `~/Library` is usually the biggest
  thing in a home directory. Symlinks are not followed.
- **iCloud is never downloaded.** The scan tells the kernel not to fetch
  dataless files while it reads directories, so measuring `Mobile Documents`
  costs nothing and moves nothing.

The scan follows [dust](https://github.com/bootandy/dust)'s approach: one rayon
scope per root, a completion counter per directory so no directory is built
before its last subdirectory lands, and one bottom-up pass that aggregates sizes
and removes duplicate hardlinks.

## The whole disk

Click the first crumb (or any directory above the scanned root) in the
trail, press `g`, or run `disktree --disk`. `g` scans the disk the scanned
root lives on from its top, `--disk` the one your home directory lives on,
which is `/` on any Mac. Another volume is a separate scan: the first crumb's ▾
lists them.

Widening is memoized: the tree already measured is handed to the wider walk
and reused where it is reached, so going from `~` to `/` reads only what is
outside `~` (seconds, not a full rescan). The current view stays on screen
until the wider tree lands, which then opens with the directory you came
from selected. Going back down is just navigation.

A scan stays on one volume, and on a Mac the boot disk is not one
filesystem: it is an APFS *volume group*, a sealed system volume at `/` and
a data volume at `/System/Volumes/Data`, stitched together by firmlinks so
that `/Users`, `/Applications` and `/Library` resolve into the data volume.
Both halves report the same device number, so disktree tells volumes apart
by their `statfs` identity instead, walks the group once through the
firmlinks, and skips `/System/Volumes`: the data volume would otherwise be
counted twice, and its siblings (`Preboot`, `VM`, `Update`) are never user
data. Other volumes of the same container, external disks, network shares,
snapshots and automount points are left out; automounts are recognised by
path before anything is read, so a NAS is never mounted just to be
measured. `-X` crosses into everything that can hold user data.

Some system directories cannot be read; they are counted as unreadable in
the totals under the title bar rather than guessed at, and the ones refused
by the privacy protection rather than by permissions are counted apart (see
Full Disk Access above).

## What it refuses to do

The removal rules live in `crates/disktree-core/src/removal.rs`, and each one is
tested:

- only paths under the scanned root can be removed;
- the filesystem root, the scanned root and your home directory are refused;
- anything System Integrity Protection marks (`SF_RESTRICTED`) is refused
  by its flag, whatever it is called, and so are the firmlink roots
  (`/Users`, `/Applications`, `/Library`, …) that the kernel itself will not
  unlink;
- a read-only volume, including the sealed system volume, is refused;
- system trees (`/System`, `/usr` outside Homebrew's `/usr/local`, `/bin`,
  `/private/etc`, `/private/var/db`, `/private/var/vm`, `/Library/Apple`,
  …) are refused even where permissions would allow it: macOS manages them,
  and Storage settings or the app that owns them are the tools;
- `/private`, `/Library`, `/Volumes`, `/opt`, `~/Library` and the
  directories macOS manages inside it (`Containers`, `Group Containers`,
  `Mobile Documents`, `CloudStorage`, `Keychains`, `Preferences`) and
  `~/.Trash` are refused as a whole, while what is *inside* them (a cache,
  a container's data, a synced folder) stays the user's to remove;
- a volume's Trash is refused: empty it from the Finder;
- a mount point is refused, since removing it would reach into another
  volume;
- a path inside a package bundle is refused, so an `.app` or a Photos
  library only ever goes as a unit;
- `/etc/hosts` and `/private/etc/hosts`, `/Users/x` and
  `/System/Volumes/Data/Users/x`, are one path to every rule above;
- a symlink is unlinked, never followed;
- nothing is passed through a shell, so a file called `-rf` is just a file.

## Develop

```sh
make run      # release build, scanning $HOME, with stdout in this terminal
make open     # the same build as Disktree.app, the way a user launches it
make lint     # rustfmt --check, then clippy with every warning an error
make test     # scanner, layout and removal tests, plus window-harness tests
make ci       # lint, then test
make icon     # re-render assets/disktree.icns after editing the SVG
make dist     # the release zip and its .sha256, in dist/
make release  # publish dist/ as the GitHub release for the current tag
```

The lint gate is strict on purpose: `clippy::all` and `clippy::pedantic` are
errors, and every exception is written down with its reason in `Cargo.toml`.
The window-harness tests draw real frames and press real keys. One marks a
directory, confirms the deletion and checks that the files are gone while
their neighbours are not, and both appearances are drawn. One
test moves a real file to the Finder's Trash and looks for it there. `make
ci` is the whole gate; there is no service running it elsewhere.

A release is made on an Apple silicon Mac: `make dist` builds the release
binary, bundles it and writes `dist/Disktree-<version>-aarch64-apple-darwin.zip`
with a `.sha256` beside it, `<version>` being `version` in `Cargo.toml`.
Tag that commit `v<version>`, push the tag, and publish the two files with
`gh release create v<version> dist/* --title "disktree <version>"
--notes-file packaging/release-notes.md`, or `make release`, which runs
that and refuses first unless the tag exists and points at `HEAD`.

| path | what lives there |
| --- | --- |
| `crates/disktree-core` | scanning, the tree, the squarified layout, the mount table, free space and removal; no UI |
| `crates/disktree-core/src/volumes.rs` | the mount table, the boot volume group, what a scan skips |
| `crates/disktree-core/src/classify.rs` | what a directory is, and whether its space can be had back |
| `crates/disktree-app/src/state.rs` | every action the interface can take, and the key map |
| `crates/disktree-app/src/views.rs` | the screens |
| `crates/disktree-app/src/treemap_view.rs` | painting the mosaic and its labels |
| `crates/disktree-app/src/theme.rs` | the light and dark palettes, and following the system appearance |
| `crates/disktree-app/src/controls.rs` | buttons, dialogs, tooltips and key caps |
| `crates/disktree-app/src/ui.rs` | the spacing, type and size scale, in `rem` |
| `crates/disktree-app/src/tests.rs` | end-to-end tests through a real window |
| `packaging/`, `assets/`, `Makefile` | the bundle, `Info.plist`, the icon, install and the release zip |

The interface follows the
[GPUI Kit design guides](https://gpui-kit.com/versions/main/docs/design-guides/):
every size is on one `rem` scale so interface zoom keeps its proportions,
primary is reserved for what Enter does, and the only question the app asks is
the one it cannot take back.

## License

MIT. This is the macOS port of [disktree](https://github.com/tobi/disktree)
by Tobi Lütke, imported from that project's 0.9.1; the treemap, the scanner
and the marking model are its work.
