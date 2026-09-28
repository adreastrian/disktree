// Builds disktree-palettes.html, the single-file gallery, by inlining the
// scripts index.src.html loads so the page opens from anywhere, unserved.
//   node build-gallery.mjs
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';

const here = (name) => fileURLToPath(new URL(name, import.meta.url));
const src = fs.readFileSync(here('index.src.html'), 'utf8');
const out = src.replace(/<script src="([^"]+)"><\/script>/g, (_, file) => {
  // A closing script tag inside the inlined source would end the block early.
  const body = fs.readFileSync(here(file), 'utf8').replace(/<\/script>/g, '<\\/script>');
  return `<script>\n// ---- ${file}\n${body}\n</script>`;
});
fs.writeFileSync(here('disktree-palettes.html'), out);
console.log('wrote disktree-palettes.html');
