# Disktree UI, as built (from the GPUI source)

Sizes in px. Font: system (San Francisco: `-apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui`).
Spacing tokens XXS 2, XS 4, SM 8, MD 12, LG 16, XL 24, XXL 32. Type CAPTION 11,
BODY 12, TITLE 14, HEADING 18, FIGURE 26, DISPLAY 40; line-height ~1.5.
Radii: keycap 4, control 6, surface 8, dialog 10.

## Explore screen, top to bottom

**Title bar** 38 tall, `background`, 1px bottom `divider`. 78px left room for
traffic lights (12px dots, first at x=9, vertically centred; red #ff5f57,
yellow #febc2e, green #28c840). Items 16 apart:
- Logo: 2×2 grid of 8px squares, 2px gaps (top: Code, Agent scratch; bottom:
  Synced, Toolchains accents), then "disktree" 18px bold `bright`.
- Breadcrumbs, 2px apart, each padded 4×2, 12px: `Macintosh HD ▾ / Users ▾ / arif ▾ / Library ▾`.
  Inactive crumbs `secondary`; current crumb `bright` semibold on a hover fill
  (foreground 8%). "/" separators `secondary` 50%; "▾" 11px `secondary`.
- Flexible spacer.
- Segmented control 176 wide: **Size | Files | Age**. Track `fill`-ish (foreground ~9%),
  2px padding, radius 6; segment padded 12×4, radius 6; chosen one filled `accent`,
  `onAccent` text, semibold, small shadow.
- Checkbox "Hidden files", checkbox "Apparent size" (16px box, radius 4,
  border foreground 40%, checked = `accent` fill with white check).
- Depth stepper: 1px `border`, radius 6, "Depth 3" then "−" "+".

**Stats + legend row** padding 16×8, items 16 apart.
- Left 11px `secondary`: `412.3 GiB · 1.8M files · 214.6k dirs · 45 not permitted` (size in `foreground`, "45 not permitted" in `warning`).
- Right, legend, items 12 apart: swatch 10px square, 4px gap, 11px `secondary` label.
  First "Reclaimable" swatch = hatch (1px lines every 3px) over Other fill;
  then the 8 categories in accent colours. In Age mode the legend becomes
  This week, This month, Six months, This year, Older.

**Treemap** fills the rest; padding 16 left/right, 8 bottom. Canvas `inset`.

**Side panel** on the right, 368 wide, `sidebar` fill, 1px left `divider`, padding 16, sections 16 apart,
separated by 1px `divider` rules. Section eyebrows 11px UPPERCASE `secondary` 70%, letter-spaced.
- SELECTION: a 4×18 bar in the category accent + name 18px bold `bright` ("Caches"),
  path 11px `secondary` (`~/Library/Caches`). Big size: **38.2** 40px bold `bright` + "GiB" 14px `secondary`.
  5px `highlight` bar showing share on a `meterTrack`. 2×2 figures (eyebrow over 14px `bright`):
  OF SCAN 9.3% · FILES 184.2k · LAST WRITE 3 days ago · KIND Cache · regenerable.
  Two equal buttons: **Open** (outline: `fill` + `border`) and **Mark for removal**
  (filled `highlight`, semibold, `onHighlight` text).
- WORTH A LOOK, total `61.4 GiB` in `highlight` at right. Rows padded 8×4: a 2×24 category
  strip, title 12px `bright` + detail 11px `secondary`, right column 88px: size + 5px bar in category accent.
  Rows: `Caches/Homebrew` regenerable 12.4 GiB · `Developer/CoreSimulator` sandbox layers 18.9 GiB ·
  `projects/atlas/target` build output 9.7 GiB · `.claude/worktrees` 3 worktrees · oldest 41 d 6.2 GiB ·
  `node_modules` (×14) package store 4.8 GiB.
- MARKED · 2 with total at right. Rows 11px: 8px category dot, path `foreground`, size `secondary`, "×".
  `~/Library/Developer/CoreSimulator/Devices` 18.9 GiB, `~/projects/atlas/target` 9.7 GiB.
- DISK (pinned bottom): eyebrow + `/dev/disk3s1` at 60%. **84.2** 26px bold + "GiB free" 12px `secondary`;
  at right `→ 112.8 GiB free` 14px `highlight`. Meter 8px: `meterTrack`, used `meterUsed`, the slice the
  marks give back = `highlight` 25% with highlight hatch. Below 11px: `410.1 GiB used` … `494.4 GiB total`;
  `12.3 GiB purgeable: freed by macOS on demand`. Review button: 1px `highlight` border, highlight 10%
  fill, radius 6, padded 12×8: `Review 2 marked · frees 28.6 GiB…` + keycap "c".

**Key bar** 1px top `divider`, padding 16×4, hints 16 apart. Keycap (11px bold, `inset` fill, 1px `border`,
radius 4, min 24 wide, padded 8×3) + 11px `secondary` label:
`space` mark · `enter` open · `⌫` up · `c` review · `hjkl` move · `/` filter · `[ ]` depth · `t` mode · `0` reset · `r` rescan.
Right: `?` all keys, then `scan 1.8M entries · 14.2 s` at 70%.

## Treemap painting
- No borders; gaps separate tiles (see palette `shape.gapTop` / `gapDeep`). Tiles < 5px are dropped.
- Squarified layout. Depth drawn: 3 levels (depth 0,1,2), deeper directories drawn closed.
- Fill = `categories[kind].fills[depth]` (Age mode: `age.fills[bucket][depth]`).
- Category strip on depth-0 tiles (per `shape.strip`), not in Age mode.
- Open directories reserve a name band: 22px at depth 0, 16px deeper (only if tile ≥ 44 wide);
  children sit below; the band shows the tile's own fill.
- Labels: name 12px, size 11px, padding ~5 left 3 top; need tile ≥ 40 wide and 12 tall. Top-level names bold.
  Colour `label` (or category `text`), deeper labels at 88%. Size in `labelDim`, short format (`1.4GiB`, `523MiB`);
  right-aligned in a top-level band, following the name after 8px in deeper bands, stacked under the name in a closed tile.
- Reclaimable: diagonal "/" hatch, 1px lines every 6px, colour `hatch`. Not on marked tiles.
- Selected: 2px `highlight` outline on top. Hover: 1px `hover`. Marked: fill `marked.fill`, 2px `marked.outline`,
  name in `marked.text`, no hatch; children of a marked directory get the same fill.
- Unreadable: a 4px `warning` square 2px in from the top-right.

## Review screen
- Title bar: "Review" 14px bold, then `2 marked · 28.6 GiB to free` 11px `secondary`.
- Body padding 16, gap 16: list well (left) + 360px summary column (right).
- List well: `inset` fill, 1px `border`, radius 8. Rows padded 12×8, 1px `divider` between, columns 12 apart:
  folder icon 12px `secondary`; name 12px + path 11px `secondary`; a 96px glyph bar `▓▓░░░░░░` in `secondary`;
  80px right-aligned size in `bright`; an **Unmark** outline button.
- Summary column: `surface`, 1px `border`, padding 16, sections 24 apart; headings 11px semibold `secondary`.
  WHAT HAPPENS: segmented control **Move to Trash | Delete permanently**; under it
  "Recoverable from the Finder's Trash until it is emptied."
  TOTALS (label left `secondary`, value right `bright`, 11px): Marked 2, Acted on 2, Nested 0, Kept back 0, Space freed 28.6 GiB.
  VOLUME: "84.2 GiB free · 112.8 GiB after removing 28.6 GiB" over a 4px meter: track, used (meterUsed),
  staying used `danger` 55%, freed slice `success` 65%.
  Bottom right: **Back** (bare) and **Move 2 items to Trash** (filled `accent`, `onAccent` text).
- Key bar: `enter` move to Trash · `m` Trash · `p` permanent · `!` unmark all · `esc` back; right `84.2 GiB available`.

## Categories (legend order) and real folder names
- Code: `src`, `projects`, `github.com`, `DerivedData`, `App.xcodeproj`
- Agent scratch: `.claude`, `.codex`, `.cursor`, `worktrees`, `experiments`, `scratch`
- Toolchains: `.cargo`, `.rustup`, `.npm`, `homebrew`, `CoreSimulator`, `Applications`
- Synced: `Dropbox`, `Google Drive`, `Mobile Documents`, `CloudStorage`, `OneDrive`
- Git: `.git`
- Media: `Pictures`, `Movies`, `Music`, `Photos Library.photoslibrary`, `.ollama`, `Steam`
- Documents: `Documents`, `Downloads`, `Desktop`, `MobileSync`
- Cache: `Caches`, `.cache`, `node_modules`, `__pycache__`, `Logs`, `.Trash`, `tmp`
- Other (not in legend): `Library`, `Containers`, `Application Support`
Reclaim reasons (hatch): regenerable, sync history, package store, build output, reinstallable,
sandbox layers, snapshots, trash, temporary.
