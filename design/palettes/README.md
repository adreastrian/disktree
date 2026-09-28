# Palettes

The source of every theme Disktree ships. The colours are generated, checked
and then emitted into `crates/disktree-app/src/themes.rs`; nothing in the app
derives a colour at run time.

## Files

| file | what |
| --- | --- |
| `SCHEMA.md` | the palette object: ui tokens, categories, age ramp, shape |
| `UI-SPEC.md` | where each token is used on screen |
| `oklab.mjs` | the colour kit: sRGB ↔ OKLab, mixing, ΔE, WCAG contrast |
| `gen-catppuccin.mjs` | Catppuccin, Catppuccin Pastel and Catppuccin Quiet: Latte for light; Frappé, Macchiato and Mocha for dark. Writes `palettes-catppuccin.js` and runs its own checks. Also holds the deferred Glass style, which is never emitted. |
| `gen-native.mjs` | Aqua, Graphite and Rosé Pine. Writes `palettes-native.js`. |
| `check-native.mjs` | the quality bar for a palette file: label contrast ≥ 4.5:1 on every fill, legend accents pairwise ΔE ≥ 8, the highlight ΔE ≥ 10 from every accent and fill, depth steps ΔE ≥ 2, a monotonic age ramp |
| `emit-rust.mjs` | writes `themes.rs` from the two palette files, through rustfmt |
| `index.src.html`, `renderer.js` | the gallery: mock explore and review screens in every palette |
| `build-gallery.mjs` | inlines the scripts into `disktree-palettes.html`, the single file to open or send |

## Regenerating

From this directory, after editing a generator:

```sh
node gen-catppuccin.mjs            # prints the checks; must say all pass
node gen-native.mjs && node check-native.mjs
node emit-rust.mjs                 # crates/disktree-app/src/themes.rs
node build-gallery.mjs             # disktree-palettes.html
```

Then `make ci` at the root: `palette.rs` and the window harness test every
theme, flavour and appearance in the emitted table.

The palette ids are the `ThemeId::key()` values, with `-<flavour>` appended
for the Catppuccin styles; `emit-rust.mjs` maps them to the enum variants.
