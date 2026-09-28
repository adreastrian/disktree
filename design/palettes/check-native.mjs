// Loads a palette file the way the browser would and checks the quality bar.
//   node check-native.mjs palettes-native.js [--ids aqua,graphite]
import fs from 'node:fs';
import vm from 'node:vm';
import { contrast, deltaE, over, averageOf, parseColor, rgbToOklch, rgbToHex } from './oklab.mjs';

const file = process.argv[2] || 'palettes-native.js';
const only = (process.argv.find((a) => a.startsWith('--ids=')) || '').slice(6).split(',').filter(Boolean);
const ctx = { window: {} };
vm.runInNewContext(fs.readFileSync(new URL(file, import.meta.url), 'utf8'), ctx);

const CATS = ['code', 'agent', 'toolchain', 'synced', 'git', 'media', 'documents', 'cache'];
const ALL = [...CATS, 'other'];
const REQUIRED_UI = ['desktop', 'background', 'surface', 'sidebar', 'inset', 'foreground', 'secondary', 'bright', 'border', 'divider', 'fill', 'accent', 'onAccent', 'highlight', 'onHighlight', 'danger', 'success', 'warning', 'hatch', 'hover', 'label', 'labelDim', 'marked', 'meterTrack', 'meterUsed'];

let failures = 0;
const f2 = (n) => n.toFixed(2);
const f1 = (n) => n.toFixed(1);

function check(ok, msg) {
  if (!ok) { failures++; console.log('  FAIL ' + msg); }
}

// A fill may be translucent in principle; resolve it over the canvas inset.
const solid = (c, under) => (parseColor(c).a < 1 ? over(c, under) : parseColor(c));

for (const p of ctx.window.PALETTES) {
  if (only.length && !only.includes(p.id)) continue;
  console.log(`\n== ${p.name} (${p.id}) ==`);
  for (const key of ['id', 'name', 'family', 'pitch', 'flavours', 'shape', 'light', 'dark']) check(key in p, `missing ${key}`);
  // Graphite's categories are meant to be quiet; the brief allows 5 there.
  const minCat = p.id === 'graphite' ? 5 : 8;
  for (const modeName of ['light', 'dark']) {
    const m = p[modeName];
    console.log(`-- ${modeName} (${p.flavours[modeName]})`);
    for (const k of REQUIRED_UI) check(k in m.ui, `ui.${k} missing`);
    for (const k of ['fill', 'outline', 'text']) check(k in m.ui.marked, `ui.marked.${k} missing`);
    for (const c of ALL) {
      check(m.categories[c], `category ${c} missing`);
      check(m.categories[c].fills.length === 5, `category ${c} needs 5 fills`);
    }
    check(m.age.swatches.length === 5 && m.age.fills.length === 5 && m.age.fills.every((r) => r.length === 5), 'age needs 5 swatches and 5x5 fills');

    const ui = m.ui;
    const inset = parseColor(ui.inset);
    // Tile label contrast on every fill, depth 0..4.
    let minLabel = 99, minLabelAt = '';
    let minDim = 99, minDimAt = '';
    let minHatch = 99, maxHatch = 0, hatchLouder = [];
    for (const c of ALL) {
      const cat = m.categories[c];
      const label = cat.text || ui.label;
      cat.fills.forEach((fill, d) => {
        const bg = solid(fill, inset);
        const lc = contrast(over(label, bg), bg);
        if (lc < minLabel) { minLabel = lc; minLabelAt = `${c}[${d}]`; }
        if (d === 0) {
          const dc = contrast(over(ui.labelDim, bg), bg);
          if (dc < minDim) { minDim = dc; minDimAt = `${c}[0]`; }
        }
        const hc = contrast(over(ui.hatch, bg), bg);
        minHatch = Math.min(minHatch, hc); maxHatch = Math.max(maxHatch, hc);
        if (hc >= lc) hatchLouder.push(`${c}[${d}]`);
      });
      // Depth steps monotonic in L and each step ΔE >= 2.
      const Ls = cat.fills.map((f) => rgbToOklch(solid(f, inset)).L);
      const dir = Math.sign(Ls[4] - Ls[0]);
      for (let d = 1; d < 5; d++) {
        const de = deltaE(solid(cat.fills[d - 1], inset), solid(cat.fills[d], inset));
        check(de >= 2, `${c} depth ${d - 1}->${d} ΔE ${f1(de)} < 2`);
        check(Math.sign(Ls[d] - Ls[d - 1]) === dir, `${c} depth lightness not monotonic at ${d}`);
      }
    }
    check(minLabel >= 4.5, `label contrast min ${f2(minLabel)} at ${minLabelAt}`);
    check(minDim >= 3, `labelDim contrast min ${f2(minDim)} at ${minDimAt}`);
    check(minHatch >= 1.15, `hatch barely visible: min ${f2(minHatch)}`);
    check(hatchLouder.length === 0, `hatch louder than label on ${hatchLouder.join(', ')}`);
    console.log(`  label min ${f2(minLabel)} (${minLabelAt}) · labelDim min ${f2(minDim)} (${minDimAt}) · hatch ${f2(minHatch)}..${f2(maxHatch)}`);

    // Secondary text on background and sidebar (composited for vibrancy).
    const secBg = contrast(over(ui.secondary, ui.background), ui.background);
    const sbBase = p.shape.vibrancy ? over(ui.sidebar, averageOf(ui.desktop)) : solid(ui.sidebar, ui.surface);
    const secSb = contrast(over(ui.secondary, sbBase), sbBase);
    check(secBg >= 4.5, `secondary on background ${f2(secBg)}`);
    check(secSb >= 4.5, `secondary on sidebar ${f2(secSb)}`);
    const onAcc = contrast(ui.onAccent, ui.accent);
    const onHl = contrast(ui.onHighlight, ui.highlight);
    check(onAcc >= 4.5, `onAccent/accent ${f2(onAcc)}`);
    check(onHl >= 4.5, `onHighlight/highlight ${f2(onHl)}`);
    const fgBg = contrast(over(ui.foreground, ui.background), ui.background);
    console.log(`  secondary/bg ${f2(secBg)} · secondary/sidebar ${f2(secSb)} (sidebar ≈ ${rgbToHex(sbBase)}) · onAccent ${f2(onAcc)} · onHighlight ${f2(onHl)} · fg/bg ${f2(fgBg)}`);

    // Legend accents pairwise distinct.
    let minPair = 99, minPairAt = '';
    for (let i = 0; i < CATS.length; i++) for (let j = i + 1; j < CATS.length; j++) {
      const de = deltaE(m.categories[CATS[i]].accent, m.categories[CATS[j]].accent);
      if (de < minPair) { minPair = de; minPairAt = `${CATS[i]}/${CATS[j]}`; }
    }
    check(minPair >= minCat, `category accents ΔE min ${f1(minPair)} (${minPairAt}) < ${minCat}`);
    // Highlight vs every category accent and depth-0 fill; danger likewise.
    let minHlAcc = 99, minHlAccAt = '', minHlFill = 99, minHlFillAt = '';
    let minDgAcc = 99, minDgAccAt = '', minDgFill = 99, minDgFillAt = '';
    for (const c of ALL) {
      const cat = m.categories[c];
      let de = deltaE(ui.highlight, cat.accent);
      if (de < minHlAcc) { minHlAcc = de; minHlAccAt = c; }
      de = deltaE(ui.highlight, solid(cat.fills[0], inset));
      if (de < minHlFill) { minHlFill = de; minHlFillAt = c; }
      de = deltaE(ui.danger, cat.accent);
      if (de < minDgAcc) { minDgAcc = de; minDgAccAt = c; }
      de = deltaE(ui.danger, solid(cat.fills[0], inset));
      if (de < minDgFill) { minDgFill = de; minDgFillAt = c; }
    }
    check(minHlAcc >= 10, `highlight vs accent ΔE ${f1(minHlAcc)} (${minHlAccAt})`);
    check(minHlFill >= 10, `highlight vs depth-0 fill ΔE ${f1(minHlFill)} (${minHlFillAt})`);
    check(minDgAcc >= 10, `danger vs accent ΔE ${f1(minDgAcc)} (${minDgAccAt})`);
    const hlDg = deltaE(ui.highlight, ui.danger);
    check(hlDg >= 10, `highlight vs danger ΔE ${f1(hlDg)}`);
    const markedVsFill = Math.min(...ALL.map((c) => deltaE(ui.marked.fill, solid(m.categories[c].fills[0], inset))));
    check(markedVsFill >= 6, `marked fill vs depth-0 fills ΔE ${f1(markedVsFill)}`);
    console.log(`  accents pairwise min ΔE ${f1(minPair)} (${minPairAt}) · highlight vs accents ${f1(minHlAcc)} (${minHlAccAt}) · vs fills ${f1(minHlFill)} (${minHlFillAt}) · danger vs accents ${f1(minDgAcc)} (${minDgAccAt}) · vs fills ${f1(minDgFill)} (${minDgFillAt}) · highlight/danger ${f1(hlDg)} · marked fill vs fills ${f1(markedVsFill)}`);

    // Age ramp: newest carries the accent hue, chroma drains monotonically, distance from neutral shrinks.
    const neutral = solid(m.categories.other.fills[0], inset);
    const sw = m.age.swatches.map((s) => rgbToOklch(parseColor(s)));
    const hueGap = Math.abs((((sw[0].h - rgbToOklch(parseColor(ui.accent)).h) % 360) + 540) % 360 - 180);
    check(hueGap <= 30, `newest age swatch hue ${f1(sw[0].h)} is ${f1(hueGap)}° from accent hue`);
    for (let i = 1; i < 5; i++) check(sw[i].C <= sw[i - 1].C + 1e-6, `age swatch chroma not draining at ${i}`);
    const dists = m.age.fills.map((row) => deltaE(solid(row[0], inset), neutral));
    for (let i = 1; i < 5; i++) check(dists[i] <= dists[i - 1] + 0.5, `age fill ${i} not closer to neutral than ${i - 1}`);
    for (let b = 0; b < 5; b++) for (let d = 1; d < 5; d++) {
      check(deltaE(solid(m.age.fills[b][d - 1], inset), solid(m.age.fills[b][d], inset)) >= 2, `age bucket ${b} depth ${d} ΔE < 2`);
      const ac = contrast(over(ui.label, solid(m.age.fills[b][d], inset)), solid(m.age.fills[b][d], inset));
      check(ac >= 4.5, `label on age fill ${b}[${d}] ${f2(ac)}`);
    }
    console.log(`  age swatch chroma ${sw.map((s) => f2(s.C)).join(' > ')} · fill distance from neutral ${dists.map(f1).join(' > ')}`);
  }
}
console.log(failures ? `\n${failures} failure(s)` : '\nall checks pass');
process.exit(failures ? 1 : 0);
