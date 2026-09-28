// Writes crates/disktree-app/src/themes.rs from the palette files: one
// static table per palette half, every colour a `0xRRGGBBAA` literal, and
// two lookups the app resolves a theme through. Run the generators first:
//   node gen-catppuccin.mjs && node gen-native.mjs && node emit-rust.mjs
// The output is passed through rustfmt, so the gate sees it as hand-written.
import fs from 'node:fs';
import vm from 'node:vm';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { parseColor } from './oklab.mjs';

const OUT = fileURLToPath(new URL('../../crates/disktree-app/src/themes.rs', import.meta.url));

// Load the palette files the way the gallery does.
const ctx = { window: {} };
for (const file of ['palettes-catppuccin.js', 'palettes-native.js']) {
  vm.runInNewContext(fs.readFileSync(new URL(file, import.meta.url), 'utf8'), ctx);
}
const byId = new Map(ctx.window.PALETTES.map((p) => [p.id, p]));

// ThemeId, in `ThemeId::ALL` order, with the palette ids each draws from.
// Glass is deferred: it stays generator data and is never emitted.
const FLAVOURS = ['frappe', 'macchiato', 'mocha'];
const THEMES = [
  { variant: 'Catppuccin', prefix: 'CATPPUCCIN', ids: FLAVOURS.map((f) => `catppuccin-${f}`) },
  { variant: 'CatppuccinPastel', prefix: 'CATPPUCCIN_PASTEL', ids: FLAVOURS.map((f) => `catppuccin-pastel-${f}`) },
  { variant: 'CatppuccinQuiet', prefix: 'CATPPUCCIN_QUIET', ids: FLAVOURS.map((f) => `catppuccin-quiet-${f}`) },
  { variant: 'Aqua', prefix: 'AQUA', ids: ['aqua'] },
  { variant: 'Graphite', prefix: 'GRAPHITE', ids: ['graphite'] },
  { variant: 'RosePine', prefix: 'ROSE_PINE', ids: ['rose-pine'] },
];
const FLAVOUR_VARIANTS = { frappe: 'Frappe', macchiato: 'Macchiato', mocha: 'Mocha' };

// The order `treemap_view::CATEGORIES` numbers the categories in.
const CATEGORIES = ['code', 'agent', 'toolchain', 'synced', 'git', 'media', 'documents', 'cache', 'other'];
const UI = [
  'background', 'surface', 'sidebar', 'inset', 'foreground', 'secondary', 'bright',
  'border', 'divider', 'fill', 'accent', 'onAccent', 'highlight', 'onHighlight',
  'danger', 'success', 'warning', 'hatch', 'hover', 'label', 'labelDim',
  'meterTrack', 'meterUsed',
];
const STRIPS = { top: 'Top', left: 'Left', dot: 'Dot', none: 'None' };

const snake = (s) => s.replace(/[A-Z]/g, (c) => '_' + c.toLowerCase());
function color(str) {
  const { r, g, b, a } = parseColor(str);
  const hex = (v) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, '0');
  return `0x${hex(r)}_${hex(g)}_${hex(b)}_${hex(a * 255)}`;
}
const list = (xs) => `[${xs.join(', ')}]`;
const rustStr = (s) => JSON.stringify(s).replace(/[^\x20-\x7e]/g, (c) => `\\u{${c.codePointAt(0).toString(16)}}`);

function half(name, mode) {
  const ui = UI.map((k) => `${snake(k)}: ${color(mode.ui[k])},`).join('\n');
  const marked = ['fill', 'outline', 'text'].map((k) => `marked_${k}: ${color(mode.ui.marked[k])},`).join('\n');
  const categories = CATEGORIES.map((k) => {
    const c = mode.categories[k];
    const text = c.text ? `Some(${color(c.text)})` : 'None';
    return `CategoryColors { accent: ${color(c.accent)}, fills: ${list(c.fills.map(color))}, text: ${text} },`;
  }).join('\n');
  const age = `Age { swatches: ${list(mode.age.swatches.map(color))}, fills: ${list(mode.age.fills.map((fs) => list(fs.map(color))))} }`;
  return `Palette {
    name: ${rustStr(name)},
    ui: Ui {
${ui}
${marked}
    },
    categories: [
${categories}
    ],
    age: ${age},
}`;
}

function shape(s) {
  if (!(s.strip in STRIPS)) throw new Error(`unknown strip ${s.strip}`);
  return `Shape { tile_radius: ${s.tileRadius}, gap_top: ${s.gapTop}, gap_deep: ${s.gapDeep}, strip: Strip::${STRIPS[s.strip]}, strip_width: ${s.stripWidth}, sheen: ${Math.round(s.sheen * 100)} }`;
}

const statics = [];
const shapeArms = [];
const paletteArms = [];
for (const theme of THEMES) {
  const first = byId.get(theme.ids[0]);
  if (!first) throw new Error(`no palette ${theme.ids[0]}`);
  statics.push(`static ${theme.prefix}_SHAPE: Shape = ${shape(first.shape)};`);
  statics.push(`static ${theme.prefix}_LIGHT: Palette = ${half(first.flavours.light, first.light)};`);
  shapeArms.push(`ThemeId::${theme.variant} => &${theme.prefix}_SHAPE,`);
  paletteArms.push(`(ThemeId::${theme.variant}, _, ThemeAppearance::Light) => &${theme.prefix}_LIGHT,`);
  if (theme.ids.length === 1) {
    statics.push(`static ${theme.prefix}_DARK: Palette = ${half(first.flavours.dark, first.dark)};`);
    paletteArms.push(`(ThemeId::${theme.variant}, _, ThemeAppearance::Dark) => &${theme.prefix}_DARK,`);
    continue;
  }
  for (const [i, id] of theme.ids.entries()) {
    const p = byId.get(id);
    if (!p) throw new Error(`no palette ${id}`);
    const flavour = FLAVOURS[i];
    const name = `${theme.prefix}_${flavour.toUpperCase()}`;
    statics.push(`static ${name}: Palette = ${half(p.flavours.dark, p.dark)};`);
    paletteArms.push(`(ThemeId::${theme.variant}, Flavour::${FLAVOUR_VARIANTS[flavour]}, ThemeAppearance::Dark) => &${name},`);
  }
}

const out = `//! The theme tables: every shipped palette, as colours and tile shape.
//!
//! GENERATED by \`design/palettes/emit-rust.mjs\` from the palette files
//! there; do not edit by hand. To regenerate, from \`design/palettes\`:
//!
//! \`\`\`sh
//! node gen-catppuccin.mjs && node gen-native.mjs && node emit-rust.mjs
//! \`\`\`
//!
//! Every colour is \`0xRRGGBBAA\`, the form [\`gpui_kit::rgba\`] takes. The
//! tables are read by [\`Theme::resolve\`](crate::theme::Theme::resolve) and
//! the palette functions in [\`crate::palette\`]; nothing here is derived at
//! run time, so what the checks in \`design/palettes\` passed is what is
//! drawn.

use gpui_kit::base::ThemeAppearance;

use crate::theme::{Flavour, ThemeId};

/// A colour as \`0xRRGGBBAA\`.
pub type Color = u32;

/// The interface tokens of one palette half.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ui {
    /// Window chrome: the panel, the bars, the review summary.
    pub background: Color,
    /// A raised sheet: menus, dialogs, the help overlay.
    pub surface: Color,
    /// The side panel.
    pub sidebar: Color,
    /// A sunken well: the treemap canvas, scrolling lists, key caps.
    pub inset: Color,
    pub foreground: Color,
    pub secondary: Color,
    pub bright: Color,
    pub border: Color,
    pub divider: Color,
    /// The resting fill of a control.
    pub fill: Color,
    pub accent: Color,
    pub on_accent: Color,
    /// The one strong colour: selection, the main action, reclaimable
    /// totals.
    pub highlight: Color,
    pub on_highlight: Color,
    pub danger: Color,
    pub success: Color,
    pub warning: Color,
    /// The diagonal hatch over reclaimable space.
    pub hatch: Color,
    /// The outline of the tile under the pointer.
    pub hover: Color,
    /// A tile's name on its fill, and the dimmer size beside it.
    pub label: Color,
    pub label_dim: Color,
    /// A tile marked for removal: its fill, its ring, its name.
    pub marked_fill: Color,
    pub marked_outline: Color,
    pub marked_text: Color,
    /// The disk meter: the whole volume, and the part in use.
    pub meter_track: Color,
    pub meter_used: Color,
}

/// One category's colours: the legend swatch and strip, a fill per depth,
/// and the ink over those fills where the palette's label colour does not
/// read on them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CategoryColors {
    pub accent: Color,
    pub fills: [Color; DEPTHS],
    pub text: Option<Color>,
}

/// The age ramp, newest first: a legend swatch per bucket and a fill per
/// bucket and depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Age {
    pub swatches: [Color; BUCKETS],
    pub fills: [[Color; DEPTHS]; BUCKETS],
}

/// Depth steps a fill distinguishes; deeper clamps to the last.
pub const DEPTHS: usize = 5;
/// Age buckets, as [\`crate::palette::AGE_BUCKETS\`] lists them.
pub const BUCKETS: usize = 5;

/// How a top-level tile carries its category colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strip {
    /// A band along the top edge.
    Top,
    /// A band along the left edge.
    Left,
    /// A dot in the top-left corner.
    Dot,
    /// Nothing: the fill alone says it.
    #[allow(
        dead_code,
        reason = "the schema allows a theme without a strip; none shipped
                  is one"
    )]
    None,
}

/// The tile geometry of a theme, in pixels at 100% zoom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shape {
    pub tile_radius: u8,
    /// The gap between top-level tiles, and between deeper ones.
    pub gap_top: u8,
    pub gap_deep: u8,
    pub strip: Strip,
    /// The band's thickness, or the dot's diameter.
    pub strip_width: u8,
    /// A highlight along a tile's top edge, in hundredths of full white;
    /// zero is none.
    pub sheen: u8,
}

/// One half of a theme: everything it draws in one appearance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// The half's own name: the Catppuccin flavour, or Light and Dark.
    pub name: &'static str,
    pub ui: Ui,
    /// In the order \`treemap_view::CATEGORIES\` numbers them.
    pub categories: [CategoryColors; 9],
    pub age: Age,
}

/// The tile geometry \`theme\` draws with, in either appearance.
pub const fn shape(theme: ThemeId) -> &'static Shape {
    match theme {
${shapeArms.join('\n')}
    }
}

/// The palette \`theme\` draws in \`appearance\`; \`flavour\` picks among the
/// dark halves of a theme that has them and is ignored otherwise.
pub const fn palette(
    theme: ThemeId,
    flavour: Flavour,
    appearance: ThemeAppearance,
) -> &'static Palette {
    match (theme, flavour, appearance) {
${paletteArms.join('\n')}
    }
}

${statics.join('\n\n')}
`;

fs.writeFileSync(OUT, out);
execFileSync('rustfmt', ['--edition', '2024', OUT]);
console.log(`wrote ${OUT}`);
