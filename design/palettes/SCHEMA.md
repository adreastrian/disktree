# Disktree palette schema

Every palette file is a plain browser script (no modules) that pushes
palette objects onto a global array:

```js
window.PALETTES = window.PALETTES || [];
window.PALETTES.push({ ... });
```

All colours are CSS colour strings (`#rrggbb`, `#rrggbbaa` or `rgba()`),
fully resolved: the renderer does no colour maths of its own.

```js
{
  id: 'frappe',                 // kebab-case, unique
  name: 'Frappé',               // short display name
  family: 'Catppuccin',         // grouping shown on the card
  pitch: 'One or two plain sentences: the idea, and what it is good at.',
  flavours: { light: 'Latte', dark: 'Frappé' }, // label for each appearance
  shape: {
    tileRadius: 0,       // px corner radius of every tile (0..8)
    gapTop: 6,           // px between top-level tiles (each inset gapTop/2)
    gapDeep: 2,          // px between nested tiles
    strip: 'top',        // category mark on top-level tiles: 'top' | 'left' | 'dot' | 'none'
    stripWidth: 2,       // px thickness of that strip (dot diameter for 'dot')
    sheen: 0,            // 0..0.12: white top-to-bottom gradient laid over each
                         // tile at this opacity, for a lit, native feel (0 = flat)
    vibrancy: false,     // true: side panel and title bar are translucent over
                         // a blurred desktop, the way Finder's sidebar is
    windowRadius: 10,    // px corner radius of the mock window
  },
  light: MODE,
  dark: MODE,
}
```

`MODE`:

```js
{
  ui: {
    desktop: '#…',      // what is behind the window (a wallpaper-ish colour or gradient string)
    background: '#…',   // window chrome: title bar, stats row, key bar
    surface: '#…',      // side panel, dialog, tooltip, review summary
    sidebar: '#…',      // side panel fill; rgba() when shape.vibrancy
    inset: '#…',        // treemap canvas, list wells, keycaps
    foreground: '#…',   // body text
    secondary: '#…',    // captions, muted labels
    bright: '#…',       // names, headings, big figures
    border: '#…',       // card/dialog/keycap outlines
    divider: '#…',      // 1px rules between bars
    fill: '#…',         // resting control fill (buttons, segmented track)
    accent: '#…',       // selected segment, checkbox, primary button, focus
    onAccent: '#…',     // text on accent
    highlight: '#…',    // THE one strong colour: selection ring, Mark button,
                        // review button, reclaimable totals, projected free space
    onHighlight: '#…',  // text on a filled highlight
    danger: '#…',       // marked tiles, delete
    success: '#…',      // freed slice, clean git
    warning: '#…',      // unreadable marker, "not permitted" (may equal highlight)
    hatch: '#…',        // reclaimable diagonal hatch line colour (with alpha)
    hover: '#…',        // 1px hover outline on a tile
    label: '#…',        // tile name text on a category fill
    labelDim: '#…',     // tile size text / deeper labels
    marked: { fill: '#…', outline: '#…', text: '#…' },
    meterTrack: '#…',   // empty part of meters
    meterUsed: '#…',    // used part of the disk meter
  },
  categories: {
    // keys are fixed; 'other' is not in the legend
    code, agent, toolchain, synced, git, media, documents, cache, other: {
      accent: '#…',          // strip over top-level tiles and legend swatch
      fills: ['#…', '#…', '#…', '#…', '#…'], // depth 0..4 tile fills
      text: '#…',            // OPTIONAL: label colour on these fills, if ui.label won't do
    },
  },
  age: {
    // newest first: This week, This month, Six months, This year, Older
    swatches: ['#…', '#…', '#…', '#…', '#…'],
    fills: [ ['d0','d1','d2','d3','d4'], … five buckets … ],
  },
}
```

Legend labels, in order: Code, Agent scratch, Toolchains, Synced, Git,
Media, Documents, Cache.
