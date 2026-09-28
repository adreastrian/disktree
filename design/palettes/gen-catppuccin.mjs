// Generates palettes-catppuccin.js: the Catppuccin palettes for Disktree.
// Light is always Latte; dark comes in the three official dark flavours
// (Frappé, Macchiato, Mocha), each derived the same way from its own
// anchors. Three styles ship (Catppuccin, Pastel, Quiet); Glass is kept as
// data for the gallery but is deferred and never emitted to Rust. All
// blending is in OKLab; the output is literal hex so the renderer does no
// colour maths.
//
//   node gen-catppuccin.mjs          # writes palettes-catppuccin.js, prints checks
//   node gen-catppuccin.mjs --check  # checks only
import { writeFileSync } from 'node:fs';

// ---------------------------------------------------------------- colour maths
const clamp01 = (x) => Math.min(1, Math.max(0, x));
const hexToRgb = (h) => {
  const s = h.replace('#', '');
  const n = parseInt(s.slice(0, 6), 16);
  const a = s.length === 8 ? parseInt(s.slice(6, 8), 16) / 255 : 1;
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255, a];
};
const rgbToHex = ([r, g, b]) =>
  '#' + [r, g, b].map((v) => Math.round(clamp01(v / 255) * 255).toString(16).padStart(2, '0')).join('');
const srgbToLin = (c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const linToSrgb = (c) => (c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055);

function parse(css) {
  if (css.startsWith('#')) return hexToRgb(css);
  const m = css.match(/rgba?\(([^)]+)\)/);
  const p = m[1].split(',').map((s) => parseFloat(s));
  return [p[0], p[1], p[2], p.length > 3 ? p[3] : 1];
}
function rgbToOklab([r, g, b]) {
  const lr = srgbToLin(r / 255), lg = srgbToLin(g / 255), lb = srgbToLin(b / 255);
  const l = Math.cbrt(0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb);
  const m = Math.cbrt(0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb);
  const s = Math.cbrt(0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb);
  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  ];
}
function oklabToRgb([L, a, b]) {
  const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  const lr = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
  const lg = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
  const lb = -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s;
  return [linToSrgb(clamp01(lr)) * 255, linToSrgb(clamp01(lg)) * 255, linToSrgb(clamp01(lb)) * 255];
}
const oklab = (css) => rgbToOklab(parse(css));
// mix a toward b by t, in OKLab
function mix(a, b, t) {
  const A = oklab(a), B = oklab(b);
  return rgbToHex(oklabToRgb(A.map((v, i) => v + (B[i] - v) * t)));
}
const deltaE = (a, b) => {
  const A = oklab(a), B = oklab(b);
  return 100 * Math.hypot(A[0] - B[0], A[1] - B[1], A[2] - B[2]);
};
const chroma = (c) => { const [, a, b] = oklab(c); return 100 * Math.hypot(a, b); };
const lum = (css) => {
  const [r, g, b] = parse(css);
  return 0.2126 * srgbToLin(r / 255) + 0.7152 * srgbToLin(g / 255) + 0.0722 * srgbToLin(b / 255);
};
// composite an rgba() colour over an opaque backdrop (sRGB, as browsers do)
function over(fg, bg) {
  const [r, g, b, a] = parse(fg), [R, G, B] = parse(bg);
  return rgbToHex([r * a + R * (1 - a), g * a + G * (1 - a), b * a + B * (1 - a)]);
}
const contrast = (fg, bg) => {
  const f = fg.startsWith('rgba') ? over(fg, bg) : fg;
  const l1 = lum(f), l2 = lum(bg);
  return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
};
// average colour of a CSS gradient string (its stops), for vibrancy checks
function gradientAverage(css) {
  const stops = css.match(/#[0-9a-f]{6}/gi) || [css];
  const labs = stops.map(oklab);
  const avg = [0, 1, 2].map((i) => labs.reduce((s, l) => s + l[i], 0) / labs.length);
  return rgbToHex(oklabToRgb(avg));
}
const rgba = (hex, a) => { const [r, g, b] = hexToRgb(hex); return `rgba(${r},${g},${b},${a})`; };

// Five depth fills walking from `from` toward `to`, starting at t0 and stepping
// so that neighbours differ by >= minStep dE; backs t0 off until every fill
// keeps `text` at >= minContrast. `lift` optionally post-mixes toward a colour.
function ramp(from, to, text, { t0 = 0.25, minStep = 2.3, minContrast = 4.55, lift = null } = {}) {
  for (let start = t0; start >= 0; start -= 0.005) {
    const fills = [];
    let t = start;
    let ok = true;
    for (let d = 0; d < 5; d++) {
      let c = lift ? mix(mix(from, to, t), lift[0], lift[1]) : mix(from, to, t);
      if (d > 0) {
        // step forward until the dE from the previous fill is enough
        let tt = t;
        while (deltaE(fills[d - 1], c) < minStep && tt < 1) {
          tt += 0.005;
          c = lift ? mix(mix(from, to, tt), lift[0], lift[1]) : mix(from, to, tt);
        }
        t = tt;
      }
      if (contrast(text, c) < minContrast) { ok = false; break; }
      fills.push(c);
    }
    if (ok) return fills;
  }
  throw new Error(`ramp: no solution from ${from} to ${to} for text ${text}`);
}
// darken/lighten a colour until `text` reads on it at >= 4.5
function fitContrast(c, text, dir, target = 4.6) {
  let out = c;
  const toward = dir === 'darker' ? '#000000' : '#ffffff';
  for (let t = 0; t <= 1 && contrast(text, out) < target; t += 0.01) out = mix(c, toward, t);
  return out;
}

// ---------------------------------------------------------------- Catppuccin
const F = { // Frappé
  rosewater: '#f2d5cf', flamingo: '#eebebe', pink: '#f4b8e4', mauve: '#ca9ee6', red: '#e78284',
  maroon: '#ea999c', peach: '#ef9f76', yellow: '#e5c890', green: '#a6d189', teal: '#81c8be',
  sky: '#99d1db', sapphire: '#85c1dc', blue: '#8caaee', lavender: '#babbf1', text: '#c6d0f5',
  subtext1: '#b5bfe2', subtext0: '#a5adce', overlay2: '#949cbb', overlay1: '#838ba7',
  overlay0: '#737994', surface2: '#626880', surface1: '#51576d', surface0: '#414559',
  base: '#303446', mantle: '#292c3c', crust: '#232634',
};
const M = { // Macchiato
  rosewater: '#f4dbd6', flamingo: '#f0c6c6', pink: '#f5bde6', mauve: '#c6a0f6', red: '#ed8796',
  maroon: '#ee99a0', peach: '#f5a97f', yellow: '#eed49f', green: '#a6da95', teal: '#8bd5ca',
  sky: '#91d7e3', sapphire: '#7dc4e4', blue: '#8aadf4', lavender: '#b7bdf8', text: '#cad3f5',
  subtext1: '#b8c0e0', subtext0: '#a5adcb', overlay2: '#939ab7', overlay1: '#8087a2',
  overlay0: '#6e738d', surface2: '#5b6078', surface1: '#494d64', surface0: '#363a4f',
  base: '#24273a', mantle: '#1e2030', crust: '#181926',
};
const Mo = { // Mocha
  rosewater: '#f5e0dc', flamingo: '#f2cdcd', pink: '#f5c2e7', mauve: '#cba6f7', red: '#f38ba8',
  maroon: '#eba0ac', peach: '#fab387', yellow: '#f9e2af', green: '#a6e3a1', teal: '#94e2d5',
  sky: '#89dceb', sapphire: '#74c7ec', blue: '#89b4fa', lavender: '#b4befe', text: '#cdd6f4',
  subtext1: '#bac2de', subtext0: '#a6adc8', overlay2: '#9399b2', overlay1: '#7f849c',
  overlay0: '#6c7086', surface2: '#585b70', surface1: '#45475a', surface0: '#313244',
  base: '#1e1e2e', mantle: '#181825', crust: '#11111b',
};
// The dark flavours, keyed the way the app's Flavour enum spells them.
const DARK = [
  { key: 'frappe', name: 'Frappé', P: F, desktop: 'linear-gradient(135deg,#2a2d3f,#3a3550)' },
  { key: 'macchiato', name: 'Macchiato', P: M, desktop: 'linear-gradient(135deg,#20233a,#2f2a4a)' },
  { key: 'mocha', name: 'Mocha', P: Mo, desktop: 'linear-gradient(135deg,#191a2c,#28233f)' },
];
const L = { // Latte
  rosewater: '#dc8a78', flamingo: '#dd7878', pink: '#ea76cb', mauve: '#8839ef', red: '#d20f39',
  maroon: '#e64553', peach: '#fe640b', yellow: '#df8e1d', green: '#40a02b', teal: '#179299',
  sky: '#04a5e5', sapphire: '#209fb5', blue: '#1e66f5', lavender: '#7287fd', text: '#4c4f69',
  subtext1: '#5c5f77', subtext0: '#6c6f85', overlay2: '#7c7f93', overlay1: '#8c8fa1',
  overlay0: '#9ca0b0', surface2: '#acb0be', surface1: '#bcc0cc', surface0: '#ccd0da',
  base: '#eff1f5', mantle: '#e6e9ef', crust: '#dce0e8',
};
const CATS = ['code', 'agent', 'toolchain', 'synced', 'git', 'media', 'documents', 'cache', 'other'];
const LEGEND = CATS.slice(0, 8);

// Shared chrome for a dark flavour `P`; variants override what they change.
function darkUi(P, desktop, o = {}) {
  return {
    desktop,
    background: P.mantle,
    surface: P.base,
    sidebar: P.base,
    inset: P.crust,
    foreground: P.text,
    secondary: P.subtext0,
    bright: mix(P.text, '#ffffff', 0.5),
    border: P.surface1,
    divider: rgba(P.text, 0.12),
    fill: rgba(P.text, 0.06),
    accent: P.mauve,
    onAccent: P.crust,
    highlight: mix(P.peach, P.red, 0.2),
    onHighlight: P.crust,
    danger: P.red,
    success: P.green,
    warning: P.yellow,
    hatch: rgba(P.text, 0.14),
    hover: rgba(P.text, 0.6),
    label: P.text,
    labelDim: P.subtext0,
    marked: { fill: mix(P.crust, P.red, 0.32), outline: P.red, text: P.red },
    meterTrack: rgba(P.text, 0.1),
    meterUsed: rgba(P.text, 0.3),
    ...o,
  };
}
function latteUi(o = {}) {
  return {
    desktop: 'linear-gradient(135deg,#d9dcec,#e9dbe6)',
    background: L.mantle,
    surface: L.base,
    sidebar: L.base,
    inset: L.crust,
    foreground: L.text,
    secondary: L.subtext1, // subtext0 is 4.1:1 on mantle; subtext1 clears 4.5
    bright: mix(L.text, '#000000', 0.3),
    border: L.surface1,
    divider: rgba(L.text, 0.14),
    fill: rgba(L.text, 0.05),
    accent: L.mauve,
    onAccent: '#ffffff',
    highlight: L.peach,
    onHighlight: '#11111b', // Latte peach is mid-luminance: white is 3.1:1, near-black ink clears 4.5
    danger: L.red,
    success: L.green,
    warning: L.yellow,
    hatch: rgba(L.text, 0.16),
    hover: rgba(L.text, 0.6),
    label: L.text,
    labelDim: L.subtext0,
    marked: { fill: mix(L.base, L.red, 0.2), outline: L.red, text: L.red },
    meterTrack: rgba(L.text, 0.1),
    meterUsed: rgba(L.text, 0.3),
    ...o,
  };
}

// Build categories for a mode from an accent map and a fills(accent, key) fn.
function categories(accents, fillsOf, textOf = () => undefined) {
  const out = {};
  for (const k of CATS) {
    const c = { accent: accents[k], fills: fillsOf(accents[k], k) };
    const t = textOf(k);
    if (t) c.text = t;
    out[k] = c;
  }
  return out;
}
// Age: newest bucket carries `hue`; older buckets drain toward `neutral`.
function age(hue, neutral, fillsOf) {
  const swatches = [0, 0.3, 0.55, 0.78, 1].map((t) => mix(hue, neutral, t));
  return { swatches, fills: swatches.map((s) => fillsOf(s, 'age')) };
}

// ------------------------------------------------------------------- variants
const palettes = [];

// One legend mapping for every variant and flavour. The pastels sit close
// together in OKLab (blue/lavender 8, mauve/lavender 7, flamingo/pink 6,
// peach/maroon 6, yellow/peach 9.7 in Frappé; the other flavours are the
// same hues a step brighter), so the only assignment that keeps every
// legend pair >= 8 and the highlight >= 10 from all of them is: peach as the
// one strong colour, Agent scratch on rosewater, Documents on a
// lavender-tinted overlay and Cache on a muted yellow. Media keeps mauve,
// Git keeps pink (well clear of the red that marks a tile for deletion).
//
// Macchiato and Mocha are brighter and closer together: rosewater sits
// 6-7 from yellow and teal 7-8 from green. `apart` walks a colour toward a
// second one in twentieths until it is >= 8 from every other legend accent
// and >= 10 from the highlight; Frappé needs no step, so its accents are
// the anchors unchanged. Rosewater goes toward white (still the pale warm
// one), teal toward sapphire (still the cool one, a touch bluer).
function apart(c, toward, others, highlight) {
  for (let t = 0; t <= 0.7; t += 0.05) {
    const m = mix(c, toward, t);
    if (others.every((o) => deltaE(m, o) >= 8) && deltaE(m, highlight) >= 10) return m;
  }
  throw new Error(`apart: ${c} toward ${toward} never clears the legend`);
}
const darkAccents = (P) => {
  const highlight = mix(P.peach, P.red, 0.2); // the dark highlight (darkUi)
  const fixed = {
    code: P.blue, toolchain: P.green, git: P.pink, media: P.mauve,
    documents: mix(P.overlay1, P.lavender, 0.1), cache: P.yellow,
  };
  const agent = apart(P.rosewater, '#ffffff', Object.values(fixed), highlight);
  const synced = apart(P.teal, P.sapphire, [...Object.values(fixed), agent], highlight);
  return { ...fixed, agent, synced, other: P.overlay0 };
};
const LA = {
  code: L.blue, agent: L.rosewater, toolchain: L.green, synced: L.teal, git: L.pink,
  media: L.mauve, documents: mix(L.overlay1, L.lavender, 0.3), cache: L.yellow,
  other: L.overlay0,
};
// Peach is 9.7 from yellow in every flavour; a fifth of a step toward red
// puts the highlight clear of every category and it still reads as peach.
const L_HIGHLIGHT = mix(L.peach, L.red, 0.25);

// A style is one light (Latte) half and one dark half per flavour. Each
// flavour is a separate palette in the gallery, with the id the app uses:
// `<style>-<flavour>`. Glass has only Frappé: it is deferred.
function style({ id, name, pitch, shape, light, dark, flavours = DARK }) {
  for (const flavour of flavours) {
    palettes.push({
      id: `${id}-${flavour.key}`, name: `${name} ${flavour.name}`, family: 'Catppuccin',
      pitch,
      flavours: { light: 'Latte', dark: flavour.name },
      shape,
      light,
      dark: dark(flavour.P, flavour.desktop),
    });
  }
}

// 1. Catppuccin: faithful. Accent blended into the canvas, rising gently
// with depth.
{
  const lFills = (a) => ramp(L.base, a, L.text, { t0: 0.22, minContrast: 4.6 });
  style({
    id: 'catppuccin', name: 'Catppuccin',
    pitch: 'The faithful one: each category accent sinks into the canvas at a moderate strength and rises a little with depth, with the full accent in the strip. Calm and even, so a large tree still reads as one surface.',
    shape: { tileRadius: 4, gapTop: 6, gapDeep: 2, strip: 'top', stripWidth: 2, sheen: 0, vibrancy: false, windowRadius: 10 },
    light: { ui: latteUi({ highlight: L_HIGHLIGHT }), categories: categories(LA, lFills), age: age(L.blue, L.overlay0, lFills) },
    dark: (P, desktop) => {
      const dFills = (a) => ramp(P.crust, a, P.text, { t0: 0.26 });
      return { ui: darkUi(P, desktop), categories: categories(darkAccents(P), dFills), age: age(P.blue, P.overlay0, dFills) };
    },
  });
}

// 2. Catppuccin Pastel: candy. Tiles are the pastels themselves; crust ink
// on them.
{
  // light: soft tints of the strong Latte accents, deepening with depth
  const lFills = (a) => ramp(L.base, a, L.text, { t0: 0.34, minContrast: 4.6 });
  const lAccent = fitContrast(L.pink, '#ffffff', 'darker');
  style({
    id: 'catppuccin-pastel', name: 'Catppuccin Pastel',
    pitch: 'Candy. Dark mode paints the pastels at full strength with dark ink on them; Latte tints the tiles softly and keeps the usual text. Playful, quick to scan by hue, and the biggest departure from a grey tool.',
    shape: { tileRadius: 5, gapTop: 8, gapDeep: 3, strip: 'dot', stripWidth: 6, sheen: 0.04, vibrancy: false, windowRadius: 10 },
    light: {
      ui: latteUi({ accent: lAccent, onAccent: '#ffffff', highlight: L_HIGHLIGHT, hover: rgba(L.text, 0.7) }),
      categories: categories(LA, lFills),
      age: age(L.sapphire, L.overlay0, lFills),
    },
    dark: (P, desktop) => {
      const dAcc = { ...darkAccents(P), other: P.overlay2 }; // overlay1 is 4.4:1 under crust ink
      // the pastel at full strength, lightening a step per depth. A pastel
      // that is already near white (the whitened Agent scratch of the
      // brighter flavours) has no room to climb four steps, so it starts a
      // shade deeper: pulled toward crust until every step is a full one.
      const dFills = (a) => {
        for (let t = 0; t <= 0.3; t += 0.01) {
          const fills = ramp(mix(a, P.crust, t), '#ffffff', P.crust, { t0: 0, minStep: 2.6 });
          if (fills.every((c, d) => d === 0 || deltaE(fills[d - 1], c) >= 2.6)) return fills;
        }
        throw new Error(`pastel ramp: ${a} cannot climb toward white`);
      };
      return {
        ui: darkUi(P, desktop, {
          accent: P.pink, onAccent: P.crust,
          hatch: rgba(P.crust, 0.18),
          hover: rgba(P.crust, 0.8),
          label: P.crust, labelDim: P.mantle, // surface0 is 2.98:1 on the Documents pastel
          marked: { fill: P.red, outline: mix(P.red, '#ffffff', 0.45), text: P.crust },
        }),
        categories: categories(dAcc, dFills, () => P.crust),
        age: age(P.sapphire, P.overlay2, dFills),
      };
    },
  });
}

// 3. Catppuccin Quiet: neutral tiles, category only in a left strip and the
// legend.
{
  // surface0 -> surface2 cannot hold text at 4.5:1 (surface2 is 3.6:1), so the
  // neutral steps run base -> surface1 instead, at 2.2 dE a step.
  const lNeutral = ramp(L.base, L.surface1, L.text, { t0: 0, minStep: 2.2 });
  // Age mode is the one place Quiet colours a tile: a faint blue that drains.
  const lAge = (s) => ramp(L.base, s, L.text, { t0: 0.2, minContrast: 4.6 });
  style({
    id: 'catppuccin-quiet', name: 'Catppuccin Quiet',
    pitch: 'Reserved. Tiles are neutral surface steps and the category lives in a slim left strip and the legend, so colour is spent only on the highlight and the marks. The closest to a professional Mac tool.',
    shape: { tileRadius: 3, gapTop: 6, gapDeep: 2, strip: 'left', stripWidth: 3, sheen: 0, vibrancy: false, windowRadius: 10 },
    light: {
      ui: latteUi({ accent: L.blue, onAccent: '#ffffff', highlight: L_HIGHLIGHT }),
      categories: categories(LA, () => lNeutral.slice()),
      age: age(L.blue, L.overlay0, lAge),
    },
    dark: (P, desktop) => {
      const dNeutral = ramp(P.base, P.surface1, P.text, { t0: 0, minStep: 2.2 });
      const dAge = (s) => ramp(P.base, s, P.text, { t0: 0.18 });
      return {
        ui: darkUi(P, desktop, { accent: P.blue, onAccent: P.crust }),
        categories: categories(darkAccents(P), () => dNeutral.slice()),
        age: age(P.blue, P.overlay0, dAge),
      };
    },
  });
}

// 4. Catppuccin Glass: vibrancy, frosted tiles, lavender accent. DEFERRED:
// in the gallery for comparison, not emitted to the app.
{
  const lDesktop = 'linear-gradient(135deg,#cfd6f0 0%,#dfe9f2 45%,#ecd6e6 100%)';
  const lFills = (a) => ramp(L.base, a, L.text, { t0: 0.24, minContrast: 4.6, lift: ['#ffffff', 0.05] });
  const lAccent = fitContrast(L.lavender, '#ffffff', 'darker'); // Latte lavender is 3.2:1 under white
  style({
    id: 'catppuccin-glass', name: 'Catppuccin Glass',
    pitch: 'Deferred. Sonoma vibrancy on Frappé: a translucent sidebar and title bar over a toned desktop, rounded frosted tiles with a soft sheen and lavender controls. For people who keep the wallpaper in view.',
    shape: { tileRadius: 6, gapTop: 6, gapDeep: 2, strip: 'top', stripWidth: 2, sheen: 0.07, vibrancy: true, windowRadius: 12 },
    flavours: [DARK[0]],
    light: {
      ui: latteUi({
        desktop: lDesktop,
        background: rgba(L.mantle, 0.78),
        sidebar: rgba(L.base, 0.74),
        inset: L.base,
        accent: lAccent, onAccent: '#ffffff',
        highlight: L_HIGHLIGHT,
        border: rgba(L.text, 0.16),
      }),
      categories: categories(LA, lFills),
      age: age(L.lavender, L.overlay0, lFills),
    },
    dark: (P) => {
      const dFills = (a) => ramp(P.base, a, P.text, { t0: 0.24, lift: ['#ffffff', 0.03] });
      return {
        ui: darkUi(P, 'linear-gradient(135deg,#3c3d63 0%,#2f4058 45%,#4a3556 100%)', {
          background: rgba(P.mantle, 0.78),
          sidebar: rgba(P.base, 0.72),
          inset: P.base,
          accent: P.lavender, onAccent: P.crust,
          border: rgba(P.text, 0.16),
          marked: { fill: mix(P.base, P.red, 0.32), outline: P.red, text: P.red },
        }),
        categories: categories(darkAccents(P), dFills),
        age: age(P.lavender, P.overlay0, dFills),
      };
    },
  });
}

// ---------------------------------------------------------------------- checks
function check(p) {
  const fails = [];
  const rows = [];
  const f2 = (x) => x.toFixed(1);
  for (const modeName of ['light', 'dark']) {
    const m = p[modeName];
    const ui = m.ui;
    const canvas = ui.inset;
    const sum = { mode: modeName };
    // labels on fills
    let minLabel = Infinity, minDim = Infinity, minStep = Infinity;
    for (const k of CATS) {
      const c = m.categories[k];
      const text = c.text || ui.label;
      c.fills.forEach((fill, d) => {
        const cr = contrast(text, fill);
        minLabel = Math.min(minLabel, cr);
        if (cr < 4.5) fails.push(`${modeName} ${k} d${d} label ${f2(cr)}`);
        if (d > 0) {
          const de = deltaE(c.fills[d - 1], fill);
          minStep = Math.min(minStep, de);
          if (de < 2) fails.push(`${modeName} ${k} d${d - 1}->d${d} dE ${f2(de)}`);
        }
      });
      const dim = contrast(ui.labelDim, c.fills[0]);
      minDim = Math.min(minDim, dim);
      if (dim < 3) fails.push(`${modeName} ${k} labelDim ${f2(dim)}`);
      // hatch: visible but quieter than the label
      const h = over(ui.hatch, c.fills[0]);
      const hde = deltaE(h, c.fills[0]);
      const hcr = contrast(h, c.fills[0]);
      if (hde < 3) fails.push(`${modeName} ${k} hatch invisible dE ${f2(hde)}`);
      if (hcr >= contrast(text, c.fills[0])) fails.push(`${modeName} ${k} hatch louder than label`);
    }
    // secondary on chrome
    const desk = gradientAverage(ui.desktop);
    const bg = ui.background.startsWith('rgba') ? over(ui.background, desk) : ui.background;
    const sb = ui.sidebar.startsWith('rgba') ? over(ui.sidebar, desk) : ui.sidebar;
    const secBg = contrast(ui.secondary, bg), secSb = contrast(ui.secondary, sb);
    if (secBg < 4.5) fails.push(`${modeName} secondary/background ${f2(secBg)}`);
    if (secSb < 4.5) fails.push(`${modeName} secondary/sidebar ${f2(secSb)}`);
    // accents pairwise, highlight vs accents and depth-0 fills
    let minPair = Infinity, minHl = Infinity;
    for (let i = 0; i < LEGEND.length; i++) {
      for (let j = i + 1; j < LEGEND.length; j++) {
        const de = deltaE(m.categories[LEGEND[i]].accent, m.categories[LEGEND[j]].accent);
        minPair = Math.min(minPair, de);
        if (de < 8) fails.push(`${modeName} accents ${LEGEND[i]}/${LEGEND[j]} dE ${f2(de)}`);
      }
    }
    for (const k of CATS) {
      const a = deltaE(ui.highlight, m.categories[k].accent);
      const f = deltaE(ui.highlight, m.categories[k].fills[0]);
      minHl = Math.min(minHl, a, f);
      if (a < 10) fails.push(`${modeName} highlight vs ${k} accent dE ${f2(a)}`);
      if (f < 10) fails.push(`${modeName} highlight vs ${k} d0 fill dE ${f2(f)}`);
    }
    // age ramp: chroma drains monotonically, newest carries the hue
    const ch = m.age.swatches.map(chroma);
    for (let i = 1; i < ch.length; i++) if (ch[i] > ch[i - 1] + 0.01) fails.push(`${modeName} age chroma not monotonic at ${i}`);
    m.age.fills.forEach((fs, b) => fs.forEach((fill, d) => {
      const cr = contrast(ui.label, fill);
      if (cr < 4.5) fails.push(`${modeName} age b${b} d${d} label ${f2(cr)}`);
      if (d > 0 && deltaE(fs[d - 1], fill) < 2) fails.push(`${modeName} age b${b} d${d} step`);
    }));
    // on-colour text
    const onA = contrast(ui.onAccent, ui.accent), onH = contrast(ui.onHighlight, ui.highlight);
    if (onA < 4.5) fails.push(`${modeName} onAccent ${f2(onA)}`);
    if (onH < 4.5) fails.push(`${modeName} onHighlight ${f2(onH)}`);
    // marked text on marked fill (not required, but worth knowing)
    const mk = contrast(ui.marked.text, ui.marked.fill);
    Object.assign(sum, { label: f2(minLabel), dim: f2(minDim), step: f2(minStep), secBg: f2(secBg), secSb: f2(secSb),
      pair: f2(minPair), hl: f2(minHl), onA: f2(onA), onH: f2(onH), marked: f2(mk) });
    rows.push(sum);
  }
  return { fails, rows };
}

// ---------------------------------------------------------------------- emit
function emit(obj, indent = 2) {
  const pad = (n) => ' '.repeat(n);
  const isFlatArray = (v) => Array.isArray(v) && v.every((x) => typeof x === 'string');
  const walk = (v, n) => {
    if (typeof v === 'string') return `'${v.replace(/'/g, "\\'")}'`;
    if (typeof v === 'number' || typeof v === 'boolean') return String(v);
    if (isFlatArray(v)) return `[${v.map((x) => walk(x, n)).join(', ')}]`;
    if (Array.isArray(v)) return `[\n${v.map((x) => pad(n + indent) + walk(x, n + indent)).join(',\n')},\n${pad(n)}]`;
    const keys = Object.keys(v);
    const inline = keys.every((k) => typeof v[k] !== 'object' || isFlatArray(v[k])) &&
      keys.length <= 3;
    if (inline) return `{ ${keys.map((k) => `${k}: ${walk(v[k], n)}`).join(', ')} }`;
    return `{\n${keys.map((k) => `${pad(n + indent)}${k}: ${walk(v[k], n + indent)}`).join(',\n')},\n${pad(n)}}`;
  };
  return walk(obj, 0);
}

let allOk = true;
for (const p of palettes) {
  const { fails, rows } = check(p);
  console.log(`\n== ${p.id}`);
  console.table(rows);
  if (fails.length) { allOk = false; console.log('FAIL', fails); }
}
if (!process.argv.includes('--check')) {
  const header = `// Catppuccin palettes for Disktree: Latte for light; Frappé, Macchiato or\n// Mocha for dark. Generated by gen-catppuccin.mjs (OKLab blends); every\n// colour is literal.\nwindow.PALETTES = window.PALETTES || [];\n`;
  const body = palettes.map((p) => `window.PALETTES.push(${emit(p)});\n`).join('\n');
  writeFileSync(new URL('./palettes-catppuccin.js', import.meta.url), header + body);
  console.log(`\nwrote palettes-catppuccin.js (${allOk ? 'all checks pass' : 'WITH FAILURES'})`);
}
process.exitCode = allOk ? 0 : 1;
