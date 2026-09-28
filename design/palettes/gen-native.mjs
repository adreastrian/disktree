// Generates palettes-native.js: Aqua, Graphite and Rosé Pine, light and dark.
//   node gen-native.mjs && node check-native.mjs palettes-native.js
// All colour maths happens here in OKLab/OKLCH; the output is literal hex.
import fs from 'node:fs';
import { oklch, hexToOklch, mix, contrast, parseColor, rgbToHex, over } from './oklab.mjs';

const hueOf = (hex) => hexToOklch(hex).h;

// Fills at a hue: five lightness steps at one chroma (gamut-clipped per step).
const ramp = (h, Ls, C) => Ls.map((L) => oklch(L, C, h));

// Lower L of `hex` (same hue and chroma) until `text` reads at >= `ratio` on it.
function darkenUntil(hex, text, ratio) {
  const { C, h } = hexToOklch(hex);
  let { L } = hexToOklch(hex);
  let out = hex;
  while (contrast(text, out) < ratio && L > 0.05) { L -= 0.005; out = oklch(L, C, h); }
  return out;
}

// Age ramp: newest bucket in the accent hue, draining to the neutral depth-0 fill.
// `dir` is +1 when depth goes lighter (dark modes) and -1 when it goes darker.
function ageRamp({ hue, neutral, Cs, Lnew, dir, step, swatchL, swatchC }) {
  const n = hexToOklch(neutral);
  const buckets = Cs.map((C, i) => {
    const t = i / (Cs.length - 1);
    const L0 = Lnew + (n.L - Lnew) * t;
    return [0, 1, 2, 3, 4].map((d) => oklch(L0 + dir * step * d, C, hue));
  });
  const swatches = swatchC.map((C, i) => oklch(swatchL[i], C, hue));
  return { swatches, fills: buckets };
}

const cat = (accent, fills, text) => (text ? { accent, fills, text } : { accent, fills });

// ---------------------------------------------------------------- Aqua

function aqua() {
  // Apple's system colours, light and dark variants. systemPink is pulled a
  // little toward magenta so marks (systemRed) never read as Git.
  const sysL = { blue: '#007aff', purple: '#af52de', green: '#34c759', teal: '#30b0c7', pink: '#ff2d55', indigo: '#5856d6', yellow: '#ffcc00', gray: '#8e8e93', gray2: '#aeaeb2', red: '#ff3b30', orange: '#ff9500' };
  const sysD = { blue: '#0a84ff', purple: '#bf5af2', green: '#30d158', teal: '#40c8e0', pink: '#ff375f', indigo: '#5e5ce6', yellow: '#ffd60a', gray: '#98989d', gray2: '#636366', red: '#ff453a', orange: '#ff9f0a' };
  const magentaPink = (hex) => { const c = hexToOklch(hex); return oklch(c.L, c.C, c.h - 14); };

  const build = (sys, dark) => {
    const Ls = dark ? [0.34, 0.37, 0.4, 0.43, 0.46] : [0.94, 0.915, 0.89, 0.865, 0.84];
    const C = dark ? 0.085 : 0.07;
    const tint = (hex, c = C) => ramp(hueOf(hex), Ls, c);
    const pink = magentaPink(sys.pink);
    const categories = {
      code: cat(sys.blue, tint(sys.blue)),
      agent: cat(sys.purple, tint(sys.purple)),
      toolchain: cat(sys.green, tint(sys.green)),
      synced: cat(sys.teal, tint(sys.teal)),
      git: cat(pink, tint(pink)),
      media: cat(sys.indigo, tint(sys.indigo)),
      documents: cat(sys.gray, ramp(250, Ls, 0.012)),
      cache: cat(sys.yellow, tint(sys.yellow, dark ? 0.07 : 0.085)),
      other: cat(sys.gray2, ramp(250, Ls, 0.004)),
    };
    const accent = darkenUntil(sys.blue, '#ffffff', 4.6);
    const age = ageRamp({
      hue: hueOf(sys.blue), neutral: categories.other.fills[0],
      Cs: dark ? [0.11, 0.08, 0.05, 0.025, 0.0] : [0.1, 0.075, 0.05, 0.025, 0.0],
      Lnew: dark ? 0.37 : 0.87, dir: dark ? 1 : -1, step: dark ? 0.03 : 0.025,
      swatchL: dark ? [0.62, 0.64, 0.66, 0.68, 0.7] : [0.58, 0.62, 0.66, 0.7, 0.74],
      swatchC: [0.17, 0.12, 0.08, 0.04, 0.0],
    });
    const ui = dark ? {
      desktop: 'linear-gradient(135deg,#2a3654,#4a2f52)',
      background: '#2b2b2d', surface: '#323234', sidebar: 'rgba(46,46,50,.74)', inset: '#1e1e20',
      foreground: 'rgba(255,255,255,.85)', secondary: '#a1a1a6', bright: '#ffffff',
      border: '#48484a', divider: 'rgba(255,255,255,.1)', fill: 'rgba(255,255,255,.06)',
      accent, onAccent: '#ffffff', highlight: sys.orange, onHighlight: '#1c1c1e',
      danger: sys.red, success: sys.green, warning: sys.orange,
      hatch: 'rgba(255,255,255,.17)', hover: 'rgba(255,255,255,.6)',
      label: '#ffffff', labelDim: 'rgba(255,255,255,.55)',
      marked: { fill: oklch(0.37, 0.13, hueOf(sys.red)), outline: sys.red, text: '#ffb3ad' },
      meterTrack: 'rgba(255,255,255,.09)', meterUsed: 'rgba(255,255,255,.32)',
    } : {
      desktop: 'linear-gradient(135deg,#a8bfe6,#e5c9dc)',
      background: '#ececec', surface: '#ffffff', sidebar: 'rgba(240,240,240,.78)', inset: '#f7f7f8',
      foreground: 'rgba(0,0,0,.85)', secondary: '#5d5d62', bright: '#000000',
      border: '#d5d5d8', divider: 'rgba(0,0,0,.1)', fill: 'rgba(0,0,0,.05)',
      accent, onAccent: '#ffffff', highlight: sys.orange, onHighlight: '#1c1c1e',
      danger: sys.red, success: sys.green, warning: sys.orange,
      hatch: 'rgba(0,0,0,.17)', hover: 'rgba(0,0,0,.5)',
      label: 'rgba(0,0,0,.84)', labelDim: 'rgba(0,0,0,.52)',
      marked: { fill: oklch(0.89, 0.09, hueOf(sys.red)), outline: sys.red, text: '#b3251c' },
      meterTrack: 'rgba(0,0,0,.08)', meterUsed: 'rgba(0,0,0,.3)',
    };
    return { ui, categories, age };
  };

  return {
    id: 'aqua', name: 'Aqua', family: 'macOS',
    pitch: 'What Apple would ship: real AppKit chrome, a translucent sidebar, and the system colours as clean luminous tints. Selection and reclaim totals are system orange, so they never fight the legend.',
    flavours: { light: 'Light', dark: 'Dark' },
    shape: { tileRadius: 5, gapTop: 6, gapDeep: 2, strip: 'top', stripWidth: 2, sheen: 0.06, vibrancy: true, windowRadius: 10 },
    light: build(sysL, false),
    dark: build(sysD, true),
  };
}

// ------------------------------------------------------------- Graphite

function graphite() {
  const hues = { code: 258, agent: 305, toolchain: 150, synced: 205, git: 355, media: 55, cache: 100 };
  const build = (dark) => {
    const Ls = dark ? [0.28, 0.31, 0.34, 0.37, 0.4] : [0.945, 0.92, 0.895, 0.87, 0.845];
    const tintC = 0.013;
    const dotL = dark ? 0.72 : 0.6;
    const dotC = 0.115;
    const dot = (h, dL = 0, dC = 0) => oklch(dotL + dL, dotC + dC, h);
    const categories = {
      code: cat(dot(hues.code), ramp(hues.code, Ls, tintC)),
      agent: cat(dot(hues.agent, 0.02), ramp(hues.agent, Ls, tintC)),
      toolchain: cat(dot(hues.toolchain, -0.02), ramp(hues.toolchain, Ls, tintC)),
      synced: cat(dot(hues.synced, 0.01), ramp(hues.synced, Ls, tintC)),
      git: cat(dot(hues.git, 0.01), ramp(hues.git, Ls, tintC)),
      media: cat(dot(hues.media, -0.02), ramp(hues.media, Ls, tintC)),
      documents: cat(oklch(dark ? 0.66 : 0.56, 0.005, 262), ramp(262, Ls, 0.004)),
      cache: cat(dot(hues.cache, -0.05, -0.02), ramp(hues.cache, Ls, tintC)),
      other: cat(oklch(dark ? 0.5 : 0.72, 0.004, 262), ramp(262, Ls, 0.0)),
    };
    const accent = '#5e6ad2';
    const highlight = dark ? oklch(0.8, 0.16, 72) : oklch(0.76, 0.16, 70);
    const age = ageRamp({
      hue: hueOf(accent), neutral: categories.other.fills[0],
      Cs: [0.075, 0.055, 0.035, 0.018, 0.0],
      Lnew: dark ? 0.3 : 0.9, dir: dark ? 1 : -1, step: dark ? 0.03 : 0.025,
      swatchL: dark ? [0.7, 0.71, 0.72, 0.73, 0.74] : [0.55, 0.58, 0.61, 0.64, 0.67],
      swatchC: [0.16, 0.11, 0.07, 0.035, 0.0],
    });
    const ui = dark ? {
      desktop: 'linear-gradient(135deg,#1b1b1f,#24242a)',
      background: '#151518', surface: '#1b1b1f', sidebar: '#1b1b1f', inset: '#0f0f12',
      foreground: '#d4d4d8', secondary: '#8f9098', bright: '#f4f4f5',
      border: '#2c2c33', divider: '#26262c', fill: '#232329',
      accent, onAccent: '#ffffff', highlight, onHighlight: '#141414',
      danger: '#f2555a', success: '#3dd68c', warning: highlight,
      hatch: 'rgba(255,255,255,.15)', hover: 'rgba(255,255,255,.55)',
      label: '#ececee', labelDim: 'rgba(236,236,238,.58)',
      marked: { fill: oklch(0.3, 0.085, 25), outline: '#f2555a', text: '#ffb0b3' },
      meterTrack: '#26262c', meterUsed: '#6b6b74',
    } : {
      desktop: 'linear-gradient(135deg,#dcdde2,#ebeaef)',
      background: '#fafafa', surface: '#ffffff', sidebar: '#ffffff', inset: '#f4f4f5',
      foreground: '#27272a', secondary: '#66686f', bright: '#09090b',
      border: '#e4e4e7', divider: '#ececef', fill: '#f0f0f2',
      accent, onAccent: '#ffffff', highlight, onHighlight: '#1a1710',
      danger: '#e5484d', success: '#30a46c', warning: highlight,
      hatch: 'rgba(0,0,0,.14)', hover: 'rgba(0,0,0,.5)',
      label: '#1c1c1f', labelDim: 'rgba(28,28,31,.56)',
      marked: { fill: oklch(0.9, 0.07, 25), outline: '#e5484d', text: '#b0262b' },
      meterTrack: '#e9e9ec', meterUsed: '#a1a1aa',
    };
    return { ui, categories, age };
  };
  return {
    id: 'graphite', name: 'Graphite', family: 'Linear / Vercel',
    pitch: 'Monochrome precision in the Linear and Vercel manner: cool greys stepped by lightness, each kind marked by a small coloured dot. Indigo runs the controls and one amber carries selection and every reclaim figure.',
    flavours: { light: 'Light', dark: 'Dark' },
    shape: { tileRadius: 3, gapTop: 4, gapDeep: 1, strip: 'dot', stripWidth: 6, sheen: 0, vibrancy: false, windowRadius: 10 },
    light: build(false),
    dark: build(true),
  };
}

// ------------------------------------------------------------ Rosé Pine

function rosePine() {
  const moon = { base: '#232136', surface: '#2a273f', overlay: '#393552', muted: '#6e6a86', subtle: '#908caa', text: '#e0def4', love: '#eb6f92', gold: '#f6c177', rose: '#ea9a97', pine: '#3e8fb0', foam: '#9ccfd8', iris: '#c4a7e7', hlLow: '#2a283e', hlMed: '#44415a', hlHigh: '#56526e' };
  const dawn = { base: '#faf4ed', surface: '#fffaf3', overlay: '#f2e9e1', muted: '#9893a5', subtle: '#797593', text: '#575279', love: '#b4637a', gold: '#ea9d34', rose: '#d7827e', pine: '#286983', foam: '#56949f', iris: '#907aa9', hlLow: '#f4ede8', hlMed: '#dfdad9', hlHigh: '#cecacd' };

  const build = (p, dark) => {
    // Derived accents: a moss green between pine and gold, a periwinkle between
    // pine and iris, and gold drained toward muted for Cache.
    const moss = (() => { const a = hexToOklch(p.pine), b = hexToOklch(p.gold); return oklch((a.L + b.L) / 2, dark ? 0.1 : 0.09, 145); })();
    const peri = oklch(dark ? 0.66 : 0.55, 0.1, 275);
    const khaki = oklch(dark ? 0.7 : 0.6, dark ? 0.05 : 0.06, hueOf(p.gold));
    const paper = oklch(dark ? 0.76 : 0.55, 0.012, 290);
    const Ls = dark ? [0.34, 0.37, 0.4, 0.43, 0.46] : [0.94, 0.918, 0.896, 0.874, 0.852];
    const C = dark ? 0.045 : 0.04;
    const tint = (hex, c = C) => ramp(hueOf(hex), Ls, c);
    const categories = {
      code: cat(p.pine, tint(p.pine)),
      agent: cat(p.iris, tint(p.iris)),
      toolchain: cat(moss, tint(moss)),
      synced: cat(p.foam, tint(p.foam)),
      git: cat(p.rose, tint(p.rose)),
      media: cat(peri, tint(peri)),
      documents: cat(paper, tint(p.subtle, 0.012)),
      cache: cat(khaki, tint(p.gold, dark ? 0.045 : 0.045)),
      other: cat(p.muted, tint(p.muted, 0.008)),
    };
    const accent = dark ? p.iris : darkenUntil(p.iris, '#ffffff', 4.6);
    const secondary = dark ? mix(p.subtle, p.text, 0.12) : mix(p.subtle, p.text, 0.45);
    // love one step deeper so marks stay apart from rose (Moon) and iris (Dawn).
    const danger = dark ? oklch(0.66, 0.18, hueOf(p.love)) : oklch(0.55, 0.15, hueOf(p.love));
    const age = ageRamp({
      hue: hueOf(p.iris), neutral: categories.other.fills[0],
      Cs: [0.085, 0.065, 0.045, 0.025, 0.008],
      Lnew: dark ? 0.37 : 0.89, dir: dark ? 1 : -1, step: dark ? 0.03 : 0.022,
      swatchL: dark ? [0.78, 0.76, 0.74, 0.72, 0.7] : [0.6, 0.62, 0.64, 0.66, 0.68],
      swatchC: [0.12, 0.09, 0.06, 0.03, 0.008],
    });
    const ui = dark ? {
      desktop: 'linear-gradient(135deg,#191724,#2a273f)',
      background: p.base, surface: p.surface, sidebar: p.surface, inset: '#1f1d2e',
      foreground: p.text, secondary, bright: p.text,
      border: p.hlMed, divider: p.overlay, fill: p.hlLow,
      accent, onAccent: p.base, highlight: p.gold, onHighlight: p.base,
      danger, success: oklch(0.78, 0.12, 150), warning: p.gold,
      hatch: 'rgba(224,222,244,.16)', hover: 'rgba(224,222,244,.6)',
      label: p.text, labelDim: 'rgba(224,222,244,.6)',
      marked: { fill: oklch(0.37, 0.12, hueOf(p.love)), outline: danger, text: '#f7b3c6' },
      meterTrack: p.hlMed, meterUsed: p.subtle,
    } : {
      desktop: 'linear-gradient(135deg,#e6dcd4,#f2e9e1)',
      background: p.base, surface: p.surface, sidebar: p.surface, inset: p.hlLow,
      foreground: p.text, secondary, bright: '#4a4568',
      border: p.hlMed, divider: p.overlay, fill: p.hlLow,
      accent, onAccent: '#ffffff', highlight: p.gold, onHighlight: '#2a2740',
      danger, success: oklch(0.6, 0.11, 150), warning: p.gold,
      hatch: 'rgba(87,82,121,.2)', hover: 'rgba(87,82,121,.55)',
      label: '#4a4568', labelDim: 'rgba(74,69,104,.75)',
      marked: { fill: oklch(0.88, 0.09, hueOf(p.love)), outline: danger, text: '#9a4a63' },
      meterTrack: p.hlMed, meterUsed: p.muted,
    };
    return { ui, categories, age };
  };
  return {
    id: 'rose-pine', name: 'Rosé Pine', family: 'Rosé Pine',
    pitch: 'The soft cousin of Catppuccin: the official Dawn and Moon palettes on a violet-grey base, with the six Rosé Pine accents and three quiet derived hues for the legend. Gold marks selection and savings, love marks removal.',
    flavours: { light: 'Dawn', dark: 'Moon' },
    shape: { tileRadius: 4, gapTop: 5, gapDeep: 2, strip: 'left', stripWidth: 3, sheen: 0.03, vibrancy: false, windowRadius: 10 },
    light: build(dawn, false),
    dark: build(moon, true),
  };
}

// ---------------------------------------------------------------- Emit

function emit(p) {
  const q = (s) => `'${String(s).replace(/'/g, "\\'")}'`;
  const arr = (a) => `[${a.map(q).join(', ')}]`;
  const mode = (m) => {
    const ui = Object.entries(m.ui).map(([k, v]) => `      ${k}: ${typeof v === 'string' ? q(v) : `{ fill: ${q(v.fill)}, outline: ${q(v.outline)}, text: ${q(v.text)} }`},`).join('\n');
    const cats = Object.entries(m.categories).map(([k, v]) => `      ${k}: { accent: ${q(v.accent)}, fills: ${arr(v.fills)}${v.text ? `, text: ${q(v.text)}` : ''} },`).join('\n');
    const age = `      swatches: ${arr(m.age.swatches)},\n      fills: [\n${m.age.fills.map((r) => `        ${arr(r)},`).join('\n')}\n      ],`;
    return `    ui: {\n${ui}\n    },\n    categories: {\n${cats}\n    },\n    age: {\n${age}\n    },`;
  };
  const shape = Object.entries(p.shape).map(([k, v]) => `${k}: ${typeof v === 'string' ? q(v) : v}`).join(', ');
  return `window.PALETTES.push({
  id: ${q(p.id)},
  name: ${q(p.name)},
  family: ${q(p.family)},
  pitch:
    ${q(p.pitch)},
  flavours: { light: ${q(p.flavours.light)}, dark: ${q(p.flavours.dark)} },
  shape: { ${shape} },
  light: {
${mode(p.light)}
  },
  dark: {
${mode(p.dark)}
  },
});
`;
}

const out = `// Three palettes for Disktree beyond Catppuccin, generated by gen-native.mjs
// (OKLab maths there; this file is plain data, every colour resolved).
window.PALETTES = window.PALETTES || [];
${[aqua(), graphite(), rosePine()].map(emit).join('\n')}`;
fs.writeFileSync(new URL('palettes-native.js', import.meta.url), out);
console.log('wrote palettes-native.js');
