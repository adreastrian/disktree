// Small colour kit shared by gen-native.mjs and check-native.mjs.
// sRGB <-> OKLab / OKLCH, gamut clipping by chroma, WCAG contrast, ΔE.

export function hexToRgb(hex) {
  const h = hex.replace('#', '');
  const n = parseInt(h.length === 3 ? h.split('').map((c) => c + c).join('') : h.slice(0, 6), 16);
  const a = h.length === 8 ? parseInt(h.slice(6, 8), 16) / 255 : 1;
  return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255, a };
}

export function parseColor(str) {
  str = str.trim();
  if (str.startsWith('#')) return hexToRgb(str);
  const m = str.match(/rgba?\(([^)]+)\)/);
  if (m) {
    const p = m[1].split(',').map((s) => parseFloat(s));
    return { r: p[0], g: p[1], b: p[2], a: p.length > 3 ? p[3] : 1 };
  }
  throw new Error('cannot parse colour ' + str);
}

export function rgbToHex({ r, g, b }) {
  const c = (v) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, '0');
  return '#' + c(r) + c(g) + c(b);
}

// Alpha composite `top` (with alpha) over opaque `under`.
export function over(top, under) {
  const t = typeof top === 'string' ? parseColor(top) : top;
  const u = typeof under === 'string' ? parseColor(under) : under;
  const a = t.a ?? 1;
  return { r: t.r * a + u.r * (1 - a), g: t.g * a + u.g * (1 - a), b: t.b * a + u.b * (1 - a), a: 1 };
}

const lin = (c) => {
  c /= 255;
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
};
const unlin = (c) => {
  const v = c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055;
  return v * 255;
};

export function luminance(rgb) {
  return 0.2126 * lin(rgb.r) + 0.7152 * lin(rgb.g) + 0.0722 * lin(rgb.b);
}

export function contrast(a, b) {
  const la = luminance(typeof a === 'string' ? parseColor(a) : a);
  const lb = luminance(typeof b === 'string' ? parseColor(b) : b);
  const [hi, lo] = la > lb ? [la, lb] : [lb, la];
  return (hi + 0.05) / (lo + 0.05);
}

export function rgbToOklab(rgb) {
  const r = lin(rgb.r), g = lin(rgb.g), b = lin(rgb.b);
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return {
    L: 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    a: 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    b: 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  };
}

export function oklabToRgbRaw({ L, a, b }) {
  const l_ = L + 0.3963377774 * a + 0.2158037573 * b;
  const m_ = L - 0.1055613458 * a - 0.0638541728 * b;
  const s_ = L - 0.0894841775 * a - 1.291485548 * b;
  const l = l_ ** 3, m = m_ ** 3, s = s_ ** 3;
  return {
    r: 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    g: -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    b: -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  };
}

const inGamut = ({ r, g, b }) => r >= -1e-4 && r <= 1 + 1e-4 && g >= -1e-4 && g <= 1 + 1e-4 && b >= -1e-4 && b <= 1 + 1e-4;

export function oklchToRgb({ L, C, h }) {
  const rad = (h * Math.PI) / 180;
  let lo = 0, hi = C;
  let best = oklabToRgbRaw({ L, a: 0, b: 0 });
  // Binary search the largest chroma that stays inside sRGB at this L and hue.
  for (let i = 0; i < 24; i++) {
    const mid = (lo + hi) / 2;
    const cand = oklabToRgbRaw({ L, a: mid * Math.cos(rad), b: mid * Math.sin(rad) });
    if (inGamut(cand)) { best = cand; lo = mid; } else hi = mid;
  }
  if (inGamut(oklabToRgbRaw({ L, a: C * Math.cos(rad), b: C * Math.sin(rad) }))) {
    best = oklabToRgbRaw({ L, a: C * Math.cos(rad), b: C * Math.sin(rad) });
  }
  return { r: unlin(Math.min(1, Math.max(0, best.r))), g: unlin(Math.min(1, Math.max(0, best.g))), b: unlin(Math.min(1, Math.max(0, best.b))), a: 1 };
}

export function rgbToOklch(rgb) {
  const { L, a, b } = rgbToOklab(rgb);
  let h = (Math.atan2(b, a) * 180) / Math.PI;
  if (h < 0) h += 360;
  return { L, C: Math.hypot(a, b), h };
}

export const hexToOklch = (hex) => rgbToOklch(parseColor(hex));
export const oklch = (L, C, h) => rgbToHex(oklchToRgb({ L, C, h }));

// ΔE in OKLab, scaled ×100 so that ~2 is a just-noticeable step.
export function deltaE(a, b) {
  const A = rgbToOklab(typeof a === 'string' ? parseColor(a) : a);
  const B = rgbToOklab(typeof b === 'string' ? parseColor(b) : b);
  return 100 * Math.hypot(A.L - B.L, A.a - B.a, A.b - B.b);
}

// Linear blend of two hexes in OKLab, t in 0..1 toward `to`.
export function mix(from, to, t) {
  const A = rgbToOklab(parseColor(from));
  const B = rgbToOklab(parseColor(to));
  const M = { L: A.L + (B.L - A.L) * t, a: A.a + (B.a - A.a) * t, b: A.b + (B.b - A.b) * t };
  const lch = { L: M.L, C: Math.hypot(M.a, M.b), h: ((Math.atan2(M.b, M.a) * 180) / Math.PI + 360) % 360 };
  return rgbToHex(oklchToRgb(lch));
}

// Average of the two stops of a `linear-gradient(...,#a,#b)` string, or the colour itself.
export function averageOf(desktop) {
  const stops = desktop.match(/#[0-9a-fA-F]{6}/g);
  if (!stops) return parseColor(desktop);
  const labs = stops.map((s) => rgbToOklab(parseColor(s)));
  const m = labs.reduce((acc, l) => ({ L: acc.L + l.L / labs.length, a: acc.a + l.a / labs.length, b: acc.b + l.b / labs.length }), { L: 0, a: 0, b: 0 });
  return oklchToRgb({ L: m.L, C: Math.hypot(m.a, m.b), h: ((Math.atan2(m.b, m.a) * 180) / Math.PI + 360) % 360 });
}
