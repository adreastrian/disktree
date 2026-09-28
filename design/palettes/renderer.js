// Disktree mock renderer: paints the explore and review screens with a
// palette's resolved colours. Plain script, no dependencies.
//
//   window.DisktreeMock.renderExplore(el, palette, 'light'|'dark', {mode:'kind'|'age'})
//   window.DisktreeMock.renderReview(el, palette, appearance)
//   window.DisktreeMock.renderTreemapOnly(el, palette, appearance, {mode})
//
// Everything renders at a fixed design size and is scaled to the container
// width with a transform, so it reads like a screenshot at any width.
(function () {
  'use strict';

  var WIN_W = 1240, WIN_H = 780, PAD = 40;
  var TITLE_H = 38, KEYBAR_H = 32, STATS_H = 32, SIDE_W = 368;
  var CANVAS_W = WIN_W - SIDE_W - 32;
  var CANVAS_H = WIN_H - TITLE_H - KEYBAR_H - STATS_H - 8;
  var MINI_W = 640, MINI_H = 400;

  var CATS = ['code', 'agent', 'toolchain', 'synced', 'git', 'media', 'documents', 'cache'];
  var CAT_LABELS = ['Code', 'Agent scratch', 'Toolchains', 'Synced', 'Git', 'Media', 'Documents', 'Cache'];
  var AGE_LABELS = ['This week', 'This month', 'Six months', 'This year', 'Older'];

  // ---- sample tree -------------------------------------------------------
  // d(name, kind, opts, children | bytesInGiB). opts: r = reclaim reason,
  // a = age bucket, u = unreadable, m = marked, own = bytes not in children.
  function d(name, kind, opts, kids) {
    var n = { name: name, kind: kind, r: opts.r || null, age: opts.a, u: !!opts.u, m: !!opts.m, own: opts.own || 0 };
    if (Array.isArray(kids)) { n.children = kids; } else { n.own = kids; }
    return n;
  }
  function aggregate(n, age) {
    if (n.age === undefined) n.age = age === undefined ? 4 : age;
    var total = n.own;
    if (n.children) {
      n.children.forEach(function (c) { total += aggregate(c, n.age); });
      n.children.sort(function (a, b) { return b.bytes - a.bytes; });
    }
    n.bytes = total;
    return total;
  }
  var TREE = d('arif', 'other', { a: 0 }, [
    d('Library', 'other', { a: 0 }, [
      d('Caches', 'cache', { r: 'regenerable', a: 0, own: 1.6 }, [
        d('Homebrew', 'cache', { r: 'regenerable' }, 12.4),
        d('com.apple.dt.Xcode', 'cache', { r: 'regenerable' }, 8.1),
        d('ms-playwright', 'cache', { r: 'regenerable' }, 4.9),
        d('JetBrains', 'cache', { r: 'regenerable', a: 1 }, 3.6),
        d('pip', 'cache', { r: 'regenerable', a: 1 }, 3.2),
        d('Google', 'cache', { r: 'regenerable' }, 2.5),
        d('com.spotify.client', 'cache', { r: 'regenerable' }, 1.9),
      ]),
      d('Developer', 'other', { a: 1 }, [
        d('CoreSimulator', 'toolchain', { r: 'sandbox layers', a: 1 }, [
          d('Devices', 'toolchain', { r: 'sandbox layers', m: true }, 18.9),
          d('Caches', 'cache', { r: 'regenerable' }, 9.5),
        ]),
        d('Xcode', 'code', { a: 1 }, [
          d('DerivedData', 'code', { r: 'build output' }, 11.2),
          d('iOS DeviceSupport', 'toolchain', { a: 2 }, 2.4),
        ]),
      ]),
      d('Application Support', 'other', { a: 1 }, [
        d('MobileSync', 'documents', { a: 3 }, 5.8),
        d('Code', 'other', {}, 5.5),
        d('Google', 'other', {}, 4.1),
        d('Slack', 'other', {}, 3.2),
      ]),
      d('Containers', 'other', { a: 2, own: 1.4 }, [
        d('com.apple.mail', 'other', { u: true }, 6.2),
        d('com.docker.docker', 'other', {}, 5.1),
        d('com.apple.Safari', 'other', {}, 2.1),
      ]),
      d('Mobile Documents', 'synced', { a: 0 }, [
        d('com~apple~CloudDocs', 'synced', {}, 7.2),
        d('iCloud~md~obsidian', 'synced', {}, 2.6),
      ]),
      d('CloudStorage', 'synced', { a: 2 }, [
        d('GoogleDrive-arif', 'synced', {}, 6.4),
      ]),
      d('Group Containers', 'other', { a: 2 }, 2.5),
      d('Logs', 'cache', { r: 'regenerable', a: 0 }, 2.1),
    ]),
    d('projects', 'code', { a: 0 }, [
      d('atlas', 'code', { a: 0 }, [
        d('target', 'cache', { r: 'build output', m: true }, 9.7),
        d('.git', 'git', {}, 3.4),
        d('node_modules', 'cache', { r: 'package store' }, 2.1),
        d('src', 'code', {}, 1.2),
        d('docs', 'code', {}, 0.4),
      ]),
      d('github.com', 'code', { a: 2 }, [
        d('omatrack', 'code', {}, [
          d('target', 'cache', { r: 'build output' }, 2.2),
          d('.git', 'git', {}, 1.8),
          d('node_modules', 'cache', { r: 'package store' }, 1.4),
          d('src', 'code', {}, 0.7),
        ]),
        d('helix', 'code', {}, [
          d('target', 'cache', { r: 'build output' }, 2.8),
          d('.git', 'git', {}, 1.6),
        ]),
        d('ghostty', 'code', {}, [
          d('.git', 'git', {}, 2.9),
          d('src', 'code', {}, 1.4),
        ]),
      ]),
      d('zed', 'code', { a: 1 }, [
        d('target', 'cache', { r: 'build output' }, 4.6),
        d('.git', 'git', {}, 3.1),
        d('crates', 'code', {}, 1.9),
      ]),
      d('disktree', 'code', { a: 0 }, [
        d('target', 'cache', { r: 'build output' }, 5.2),
        d('.git', 'git', {}, 0.9),
        d('src', 'code', {}, 0.7),
      ]),
      d('experiments', 'agent', { a: 1 }, [
        d('llm-eval', 'agent', {}, 2.8),
        d('wgpu-toy', 'agent', {}, 1.5),
        d('notes', 'agent', {}, 0.9),
      ]),
      d('ledger', 'code', { a: 3 }, [
        d('.git', 'git', {}, 2.0),
        d('node_modules', 'cache', { r: 'package store' }, 1.9),
        d('src', 'code', {}, 1.0),
      ]),
      d('scratch', 'agent', { a: 0 }, 4.1),
    ]),
    d('Pictures', 'media', { a: 3 }, [
      d('Photos Library.photoslibrary', 'media', {}, 48.7),
      d('Lightroom', 'media', { a: 2 }, 4.6),
      d('Screenshots', 'media', { a: 0 }, 3.3),
    ]),
    d('Movies', 'media', { a: 4 }, [
      d('iMovie Library.imovielibrary', 'media', {}, 18.2),
      d('footage', 'media', {}, 15.4),
      d('exports', 'media', { a: 3 }, 2.0),
    ]),
    d('.ollama', 'media', { a: 0 }, [
      d('models', 'media', {}, [
        d('blobs', 'media', {}, 22.6),
        d('manifests', 'media', {}, 0.2),
      ]),
    ]),
    d('Downloads', 'documents', { a: 0, own: 4.1 }, [
      d('installers', 'documents', { a: 1 }, 8.1),
      d('archives', 'documents', { a: 2 }, 6.2),
    ]),
    d('Documents', 'documents', { a: 3 }, [
      d('Books', 'documents', {}, 5.1),
      d('Scans', 'documents', {}, 3.5),
      d('Papers', 'documents', { a: 1 }, 3.2),
      d('Invoices', 'documents', { a: 0 }, 2.4),
    ]),
    d('Dropbox', 'synced', { r: 'sync history', a: 2 }, [
      d('Camera Uploads', 'synced', {}, 7.4),
      d('Shared', 'synced', { a: 1 }, 3.1),
      d('.dropbox.cache', 'cache', { r: 'regenerable' }, 2.1),
    ]),
    d('.cargo', 'toolchain', { r: 'reinstallable', a: 1 }, [
      d('registry', 'toolchain', { r: 'reinstallable' }, 8.6),
      d('git', 'toolchain', { r: 'reinstallable' }, 3.2),
    ]),
    d('.claude', 'agent', { a: 0 }, [
      d('worktrees', 'agent', { r: 'temporary' }, [
        d('atlas-fix', 'agent', {}, 2.6),
        d('atlas-perf', 'agent', {}, 2.1),
        d('ledger-ci', 'agent', { a: 1 }, 1.5),
      ]),
      d('projects', 'agent', {}, 2.1),
      d('cache', 'cache', { r: 'regenerable' }, 1.1),
    ]),
    d('.rustup', 'toolchain', { r: 'reinstallable', a: 1 }, [
      d('toolchains', 'toolchain', { r: 'reinstallable' }, 8.9),
    ]),
    d('.cache', 'cache', { r: 'regenerable', a: 1 }, [
      d('huggingface', 'cache', { r: 'regenerable' }, 3.9),
      d('pip', 'cache', { r: 'regenerable' }, 1.4),
      d('yarn', 'cache', { r: 'regenerable' }, 1.4),
    ]),
    d('Music', 'media', { a: 4 }, [
      d('Media', 'media', {}, 4.8),
      d('Music Library.musiclibrary', 'media', {}, 0.8),
    ]),
    d('.Trash', 'cache', { r: 'trash', a: 0 }, 4.2),
    d('Desktop', 'documents', { a: 0 }, 3.2),
    d('.npm', 'toolchain', { r: 'package store', a: 1 }, [
      d('_cacache', 'toolchain', { r: 'package store' }, 3.1),
    ]),
    d('Applications', 'toolchain', { a: 2 }, 2.6),
  ]);
  aggregate(TREE, 0);
  // Which tiles carry the selection ring and the hover outline.
  var SELECTED = 'Library/Caches', HOVER = '.ollama/models';

  // ---- helpers -----------------------------------------------------------
  function fmtSize(g) {
    if (g >= 1) return (g >= 10 ? g.toFixed(0) : g.toFixed(1)) + 'GiB';
    var m = g * 1024;
    return (m >= 10 ? m.toFixed(0) : m.toFixed(1)) + 'MiB';
  }
  function fmtLong(g) { return g.toFixed(1) + ' GiB'; }
  function esc(s) {
    return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }
  function mix(c, pct) { return 'color-mix(in srgb, ' + c + ' ' + pct + '%, transparent)'; }
  function px(n) { return Math.round(n * 100) / 100 + 'px'; }

  // Squarified layout (Bruls, Huizing, van Wijk). Items must be sorted
  // descending by v; returns rects in the same order.
  function squarify(items, rect) {
    var out = new Array(items.length);
    var total = 0;
    items.forEach(function (it) { total += it.v; });
    if (total <= 0 || rect.w <= 0 || rect.h <= 0) return out;
    var scale = (rect.w * rect.h) / total;
    var x = rect.x, y = rect.y, w = rect.w, h = rect.h;
    var i = 0, n = items.length;
    function worst(sum, mx, mn, side) {
      var s2 = sum * sum, d2 = side * side;
      return Math.max((d2 * mx) / s2, s2 / (d2 * mn));
    }
    while (i < n) {
      var side = Math.min(w, h);
      var start = i, sum = 0, mx = 0, mn = Infinity;
      while (i < n) {
        var a = items[i].v * scale;
        if (a <= 0) { out[i] = { x: x, y: y, w: 0, h: 0 }; i++; continue; }
        var nsum = sum + a, nmx = Math.max(mx, a), nmn = Math.min(mn, a);
        if (i === start || worst(nsum, nmx, nmn, side) <= worst(sum, mx, mn, side)) {
          sum = nsum; mx = nmx; mn = nmn; i++;
        } else break;
      }
      if (sum <= 0) continue;
      if (w >= h) {
        var cw = sum / h, cy = y;
        for (var k = start; k < i; k++) {
          var ih = (items[k].v * scale) / cw;
          if (out[k] === undefined) out[k] = { x: x, y: cy, w: cw, h: ih };
          cy += ih;
        }
        x += cw; w -= cw;
      } else {
        var rh = sum / w, cx = x;
        for (var j = start; j < i; j++) {
          var iw = (items[j].v * scale) / rh;
          if (out[j] === undefined) out[j] = { x: cx, y: y, w: iw, h: rh };
          cx += iw;
        }
        y += rh; h -= rh;
      }
    }
    return out;
  }

  // ---- treemap painting --------------------------------------------------
  function paintTiles(node, cell, depth, ctx, path, inheritMarked) {
    var gap = depth === 0 ? ctx.shape.gapTop : ctx.shape.gapDeep;
    var x = cell.x + gap / 2, y = cell.y + gap / 2, w = cell.w - gap, h = cell.h - gap;
    if (w < 5 || h < 5) return '';
    var ui = ctx.ui, cat = ctx.cats[node.kind] || ctx.cats.other;
    var marked = inheritMarked || node.m;
    var fill = marked ? ui.marked.fill
      : ctx.mode === 'age' ? (ctx.age.fills[node.age] || ctx.age.fills[4])[depth]
      : cat.fills[Math.min(depth, cat.fills.length - 1)];
    var open = !!node.children && depth < 2 && w >= 44 && h >= (depth === 0 ? 30 : 24);
    var band = depth === 0 ? 22 : 16;
    var labelColor = marked ? ui.marked.text : (cat.text || ui.label);
    var sw = ctx.shape.stripWidth || 0;
    var strip = depth === 0 && !marked && ctx.mode !== 'age' ? ctx.shape.strip : 'none';

    var s = '<div class="dt-tile" style="left:' + px(x) + ';top:' + px(y) + ';width:' + px(w) + ';height:' + px(h) +
      ';background:' + fill + ';border-radius:' + ctx.shape.tileRadius + 'px">';
    if (ctx.shape.sheen > 0) {
      s += '<div class="dt-ov" style="background:linear-gradient(to bottom,rgba(255,255,255,' + ctx.shape.sheen + '),rgba(255,255,255,0))"></div>';
    }
    if (node.r && !marked) {
      s += '<div class="dt-ov" style="background:repeating-linear-gradient(45deg,' + ui.hatch + ' 0 1px,transparent 1px 6px)"></div>';
    }
    var labelLeft = 5, labelTop = 3;
    if (strip === 'top') {
      s += '<div class="dt-ov" style="height:' + sw + 'px;background:' + cat.accent + '"></div>';
      labelTop += sw > 2 ? sw - 2 : 0;
    } else if (strip === 'left') {
      s += '<div class="dt-ov" style="width:' + sw + 'px;background:' + cat.accent + '"></div>';
      labelLeft += sw;
    } else if (strip === 'dot') {
      var dy = (open ? band : 18) / 2 - sw / 2;
      s += '<div class="dt-ov" style="left:5px;top:' + px(dy) + ';width:' + sw + 'px;height:' + sw + 'px;border-radius:50%;background:' + cat.accent + '"></div>';
      labelLeft += sw + 4;
    }
    // Labels.
    if (w >= 40 && h >= 12) {
      var nameStyle = 'color:' + labelColor + (depth > 0 && !marked ? ';opacity:.88' : '') + (depth === 0 ? ';font-weight:700' : '');
      var size = '<span class="dt-size" style="color:' + ui.labelDim + '">' + fmtSize(node.bytes) + '</span>';
      if (open && depth === 0) {
        s += '<div class="dt-lbl" style="left:' + labelLeft + 'px;right:5px;top:' + labelTop + 'px;display:flex;justify-content:space-between;gap:8px">' +
          '<span class="dt-name" style="' + nameStyle + '">' + esc(node.name) + '</span>' + size + '</div>';
      } else if (open) {
        s += '<div class="dt-lbl" style="left:' + labelLeft + 'px;right:5px;top:' + (labelTop - 2) + 'px;display:flex;gap:8px">' +
          '<span class="dt-name" style="' + nameStyle + '">' + esc(node.name) + '</span>' + size + '</div>';
      } else {
        s += '<div class="dt-lbl" style="left:' + labelLeft + 'px;right:5px;top:' + labelTop + 'px">' +
          '<span class="dt-name" style="' + nameStyle + '">' + esc(node.name) + '</span>' +
          (h >= 30 ? '<br>' + size : '') + '</div>';
      }
    }
    if (open) {
      var inner = { x: 0, y: band, w: w, h: h - band };
      var kids = node.children.filter(function (c) { return c.bytes > 0; });
      var rects = squarify(kids.map(function (c) { return { v: c.bytes }; }), inner);
      for (var i = 0; i < kids.length; i++) {
        if (rects[i]) s += paintTiles(kids[i], rects[i], depth + 1, ctx, path + '/' + kids[i].name, marked);
      }
    }
    if (node.u) {
      s += '<div class="dt-ov" style="left:auto;right:2px;top:2px;width:4px;height:4px;background:' + ui.warning + '"></div>';
    }
    var ring = marked ? 'inset 0 0 0 2px ' + ui.marked.outline
      : path === SELECTED ? 'inset 0 0 0 2px ' + ui.highlight
      : path === HOVER ? 'inset 0 0 0 1px ' + ui.hover : '';
    if (ring) s += '<div class="dt-ov dt-ring" style="box-shadow:' + ring + ';border-radius:' + ctx.shape.tileRadius + 'px"></div>';
    return s + '</div>';
  }

  function treemapHTML(ctx, w, h) {
    var kids = TREE.children;
    var rects = squarify(kids.map(function (c) { return { v: c.bytes }; }), { x: 0, y: 0, w: w, h: h });
    var s = '';
    for (var i = 0; i < kids.length; i++) {
      if (rects[i]) s += paintTiles(kids[i], rects[i], 0, ctx, kids[i].name, false);
    }
    return s;
  }

  function legendHTML(ctx) {
    var s = '<div class="dt-legend">';
    if (ctx.mode === 'age') {
      AGE_LABELS.forEach(function (l, i) {
        s += '<span class="dt-lg"><i style="background:' + ctx.age.swatches[i] + '"></i>' + l + '</span>';
      });
    } else {
      s += '<span class="dt-lg"><i style="background:' + ctx.cats.other.fills[0] +
        ';background-image:repeating-linear-gradient(45deg,' + ctx.ui.hatch + ' 0 1px,transparent 1px 3px)"></i>Reclaimable</span>';
      CATS.forEach(function (k, i) {
        s += '<span class="dt-lg"><i style="background:' + ctx.cats[k].accent + '"></i>' + CAT_LABELS[i] + '</span>';
      });
    }
    return s + '</div>';
  }

  // ---- chrome pieces -----------------------------------------------------
  function lights() {
    return '<div class="dt-lights"><i style="background:#ff5f57"></i><i style="background:#febc2e"></i><i style="background:#28c840"></i></div>';
  }
  function logo(ctx) {
    var c = ctx.cats;
    return '<div class="dt-logo"><div class="dt-logo-grid">' +
      '<i style="background:' + c.code.accent + '"></i><i style="background:' + c.agent.accent + '"></i>' +
      '<i style="background:' + c.synced.accent + '"></i><i style="background:' + c.toolchain.accent + '"></i>' +
      '</div><span>disktree</span></div>';
  }
  function crumbs() {
    var parts = ['Macintosh HD', 'Users', 'arif', 'Library'];
    return '<div class="dt-crumbs">' + parts.map(function (p, i) {
      return (i ? '<span class="dt-sep">/</span>' : '') +
        '<span class="dt-crumb' + (i === parts.length - 1 ? ' is-current' : '') + '">' + p + '<b>&#9662;</b></span>';
    }).join('') + '</div>';
  }
  function segmented(items, chosen, cls) {
    return '<div class="dt-seg ' + (cls || '') + '">' + items.map(function (t) {
      return '<span class="' + (t === chosen ? 'is-on' : '') + '">' + t + '</span>';
    }).join('') + '</div>';
  }
  function checkbox(label, on) {
    return '<label class="dt-check"><i class="' + (on ? 'is-on' : '') + '">' +
      (on ? '<svg viewBox="0 0 16 16" width="16" height="16"><path d="M3.5 8.5l3 3 6-6.5" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>' : '') +
      '</i>' + label + '</label>';
  }
  function keycap(k) { return '<kbd class="dt-key">' + k + '</kbd>'; }
  function hint(k, l) { return '<span class="dt-hint">' + keycap(k) + '<span>' + l + '</span></span>'; }
  function eyebrow(t, right) {
    return '<div class="dt-eyebrow"><span>' + t + '</span>' + (right ? '<span class="dt-eyebrow-r">' + right + '</span>' : '') + '</div>';
  }

  function sidePanel(ctx) {
    var c = ctx.cats, ui = ctx.ui;
    var looks = [
      ['cache', 'Caches/Homebrew', 'regenerable', 12.4],
      ['toolchain', 'Developer/CoreSimulator', 'sandbox layers', 18.9],
      ['cache', 'projects/atlas/target', 'build output', 9.7],
      ['agent', '.claude/worktrees', '3 worktrees · oldest 41 d', 6.2],
      ['cache', 'node_modules (×14)', 'package store', 4.8],
    ];
    var s = '<aside class="dt-side">';
    // SELECTION
    s += '<section class="dt-sec">' + eyebrow('Selection') +
      '<div class="dt-selname"><i style="background:' + c.cache.accent + '"></i><div><div class="dt-selh">Caches</div><div class="dt-selp">~/Library/Caches</div></div></div>' +
      '<div class="dt-big"><span>38.2</span><small>GiB</small></div>' +
      '<div class="dt-meter dt-meter5"><i style="width:9.3%;background:' + ui.highlight + '"></i></div>' +
      '<div class="dt-figs">' +
      '<div><em>Of scan</em><b>9.3%</b></div><div><em>Files</em><b>184.2k</b></div>' +
      '<div><em>Last write</em><b>3 days ago</b></div><div><em>Kind</em><b>Cache · regenerable</b></div>' +
      '</div>' +
      '<div class="dt-btns"><span class="dt-btn">Open</span><span class="dt-btn dt-btn-hl">Mark for removal</span></div>' +
      '</section>';
    // WORTH A LOOK
    s += '<section class="dt-sec dt-look">' + eyebrow('Worth a look', '<b class="dt-hl">61.4 GiB</b>') + '<div class="dt-rows">';
    looks.forEach(function (r) {
      var acc = c[r[0]].accent;
      s += '<div class="dt-row"><i style="background:' + acc + '"></i><div class="dt-rowt"><div class="dt-rowname">' + esc(r[1]) +
        '</div><div class="dt-rowdet">' + r[2] + '</div></div><div class="dt-rowr"><span>' + fmtLong(r[3]) +
        '</span><div class="dt-meter dt-meter5"><i style="width:' + Math.round((r[3] / 18.9) * 100) + '%;background:' + acc + '"></i></div></div></div>';
    });
    s += '</div></section>';
    // MARKED
    s += '<section class="dt-sec">' + eyebrow('Marked · 2', '28.6 GiB') + '<div class="dt-marks">' +
      '<div class="dt-mark"><i style="background:' + c.toolchain.accent + '"></i><span>~/Library/Developer/CoreSimulator/Devices</span><small>18.9 GiB</small><b>×</b></div>' +
      '<div class="dt-mark"><i style="background:' + c.cache.accent + '"></i><span>~/projects/atlas/target</span><small>9.7 GiB</small><b>×</b></div>' +
      '</div></section>';
    // DISK
    var used = 410.1 / 494.4, freed = 28.6 / 494.4;
    s += '<section class="dt-sec dt-disk">' + eyebrow('Disk', '<span class="dt-dim60">/dev/disk3s1</span>') +
      '<div class="dt-diskrow"><div class="dt-free"><span>84.2</span><small>GiB free</small></div><div class="dt-after">→ 112.8 GiB free</div></div>' +
      '<div class="dt-meter dt-meter8">' +
      '<i style="width:' + ((used - freed) * 100).toFixed(2) + '%;background:' + ui.meterUsed + '"></i>' +
      '<i style="left:' + ((used - freed) * 100).toFixed(2) + '%;width:' + (freed * 100).toFixed(2) + '%;background:' + mix(ui.highlight, 25) +
      ';background-image:repeating-linear-gradient(45deg,' + ui.highlight + ' 0 1px,transparent 1px 4px)"></i></div>' +
      '<div class="dt-diskline"><span>410.1 GiB used</span><span>494.4 GiB total</span></div>' +
      '<div class="dt-diskline dt-purge">12.3 GiB purgeable: freed by macOS on demand</div>' +
      '<div class="dt-review"><span>Review 2 marked · frees 28.6 GiB…</span>' + keycap('c') + '</div>' +
      '</section>';
    return s + '</aside>';
  }

  function exploreHTML(ctx) {
    var ui = ctx.ui;
    var s = '<div class="dt-titlebar">' + lights() + logo(ctx) + crumbs() + '<div class="dt-spacer"></div>' +
      segmented(['Size', 'Files', 'Age'], ctx.mode === 'age' ? 'Age' : 'Size', 'dt-seg176') +
      checkbox('Hidden files', true) + checkbox('Apparent size', false) +
      '<div class="dt-stepper"><span>Depth 3</span><b>−</b><b>+</b></div></div>';
    s += '<div class="dt-body"><div class="dt-main">' +
      '<div class="dt-stats"><div class="dt-statsl"><span style="color:' + ui.foreground + '">412.3 GiB</span> · 1.8M files · 214.6k dirs · <span style="color:' + ui.warning + '">45 not permitted</span></div>' +
      legendHTML(ctx) + '</div>' +
      '<div class="dt-canvas" style="width:' + CANVAS_W + 'px;height:' + CANVAS_H + 'px">' + treemapHTML(ctx, CANVAS_W, CANVAS_H) + '</div>' +
      '</div>' + sidePanel(ctx) + '</div>';
    s += '<div class="dt-keybar">' +
      hint('space', 'mark') + hint('enter', 'open') + hint('⌫', 'up') + hint('c', 'review') + hint('hjkl', 'move') +
      hint('/', 'filter') + hint('[ ]', 'depth') + hint('t', 'mode') + hint('0', 'reset') + hint('r', 'rescan') +
      '<div class="dt-spacer"></div>' + hint('?', 'all keys') + '<span class="dt-dim70">scan 1.8M entries · 14.2 s</span></div>';
    return s;
  }

  function reviewHTML(ctx) {
    var ui = ctx.ui;
    var rows = [
      ['Devices', '~/Library/Developer/CoreSimulator', '▓▓▓▓▓░░░', '18.9 GiB'],
      ['target', '~/projects/atlas', '▓▓░░░░░░', '9.7 GiB'],
    ];
    var folder = '<svg viewBox="0 0 16 16" width="12" height="12"><path d="M1.5 3.5h4l1.5 1.5h7.5v8h-13z" fill="none" stroke="currentColor" stroke-width="1.2"/></svg>';
    var s = '<div class="dt-titlebar">' + lights() + '<span class="dt-rvtitle">Review</span><span class="dt-rvsub">2 marked · 28.6 GiB to free</span></div>';
    s += '<div class="dt-rvbody"><div class="dt-well">';
    rows.forEach(function (r) {
      s += '<div class="dt-rvrow"><span class="dt-folder">' + folder + '</span><div class="dt-rowt"><div class="dt-rowname">' + r[0] +
        '</div><div class="dt-rowdet">' + r[1] + '</div></div><span class="dt-glyph">' + r[2] + '</span><span class="dt-rvsize">' + r[3] +
        '</span><span class="dt-btn dt-btn-sm">Unmark</span></div>';
    });
    s += '</div><div class="dt-summary">' +
      '<div class="dt-sum"><h4>What happens</h4>' + segmented(['Move to Trash', 'Delete permanently'], 'Move to Trash', 'dt-segfull') +
      '<p>Recoverable from the Finder’s Trash until it is emptied.</p></div>' +
      '<div class="dt-sum"><h4>Totals</h4><div class="dt-tot"><span>Marked</span><b>2</b></div><div class="dt-tot"><span>Acted on</span><b>2</b></div>' +
      '<div class="dt-tot"><span>Nested</span><b>0</b></div><div class="dt-tot"><span>Kept back</span><b>0</b></div><div class="dt-tot"><span>Space freed</span><b>28.6 GiB</b></div></div>' +
      '<div class="dt-sum"><h4>Volume</h4><p>84.2 GiB free · 112.8 GiB after removing 28.6 GiB</p>' +
      '<div class="dt-meter dt-meter4">' +
      '<i style="width:77.2%;background:' + ui.meterUsed + '"></i>' +
      '<i style="left:0;width:77.2%;background:' + mix(ui.danger, 55) + '"></i>' +
      '<i style="left:77.2%;width:5.8%;background:' + mix(ui.success, 65) + '"></i></div></div>' +
      '<div class="dt-spacer"></div>' +
      '<div class="dt-btns dt-btns-r"><span class="dt-btn dt-btn-bare">Back</span><span class="dt-btn dt-btn-accent">Move 2 items to Trash</span></div>' +
      '</div></div>';
    s += '<div class="dt-keybar">' + hint('enter', 'move to Trash') + hint('m', 'Trash') + hint('p', 'permanent') +
      hint('!', 'unmark all') + hint('esc', 'back') + '<div class="dt-spacer"></div><span class="dt-dim70">84.2 GiB available</span></div>';
    return s;
  }

  // ---- context, styling and mounting ------------------------------------
  function context(palette, appearance, opts) {
    var mode = palette[appearance] || palette.light || palette.dark;
    var cats = {};
    CATS.concat(['other']).forEach(function (k) {
      cats[k] = (mode.categories && mode.categories[k]) || (mode.categories && mode.categories.other) ||
        { accent: '#888', fills: ['#888', '#888', '#888', '#888', '#888'] };
    });
    var shape = palette.shape || {};
    return {
      ui: mode.ui, cats: cats, age: mode.age, appearance: appearance,
      mode: (opts && opts.mode) === 'age' ? 'age' : 'kind',
      shape: {
        tileRadius: shape.tileRadius || 0, gapTop: shape.gapTop == null ? 6 : shape.gapTop,
        gapDeep: shape.gapDeep == null ? 2 : shape.gapDeep, strip: shape.strip || 'none',
        stripWidth: shape.stripWidth == null ? 2 : shape.stripWidth, sheen: shape.sheen || 0,
        vibrancy: !!shape.vibrancy, windowRadius: shape.windowRadius == null ? 10 : shape.windowRadius,
      },
    };
  }

  function varsFor(ctx) {
    var ui = ctx.ui, s = '';
    Object.keys(ui).forEach(function (k) {
      if (k === 'marked') return;
      s += '--dt-' + k + ':' + ui[k] + ';';
    });
    s += '--dt-marked-fill:' + ui.marked.fill + ';--dt-marked-outline:' + ui.marked.outline + ';--dt-marked-text:' + ui.marked.text + ';';
    s += '--dt-winr:' + ctx.shape.windowRadius + 'px;';
    s += '--dt-titlebg:' + (ctx.shape.vibrancy ? ui.sidebar : ui.background) + ';';
    return s;
  }

  var CSS = [
    '.dt-frame{position:relative;width:100%;overflow:hidden;line-height:1.5}',
    '.dt-stage{position:absolute;left:0;top:0;transform-origin:0 0;font-family:-apple-system,BlinkMacSystemFont,"SF Pro Text",system-ui,sans-serif;font-size:12px;-webkit-font-smoothing:antialiased;color:var(--dt-foreground)}',
    '.dt-desktop{background:var(--dt-desktop);padding:' + PAD + 'px;box-sizing:border-box}',
    '.dt-win{position:relative;width:' + WIN_W + 'px;height:' + WIN_H + 'px;border-radius:var(--dt-winr);overflow:hidden;display:flex;flex-direction:column;box-shadow:0 22px 70px rgba(0,0,0,.45),0 0 0 .5px rgba(0,0,0,.35),inset 0 0 0 .5px rgba(255,255,255,.12);color:var(--dt-foreground);background:transparent;isolation:isolate}',
    '.dt-titlebar{position:relative;height:' + TITLE_H + 'px;flex:none;display:flex;align-items:center;gap:16px;padding:0 16px 0 78px;background:var(--dt-titlebg);border-bottom:1px solid var(--dt-divider);box-sizing:border-box}',
    '.dt-vib .dt-titlebar,.dt-vib .dt-side{-webkit-backdrop-filter:blur(30px) saturate(1.6);backdrop-filter:blur(30px) saturate(1.6)}',
    '.dt-lights{position:absolute;left:9px;top:13px;display:flex;gap:8px}.dt-lights i{width:12px;height:12px;border-radius:50%;display:block;box-shadow:inset 0 0 0 .5px rgba(0,0,0,.15)}',
    '.dt-logo{display:flex;align-items:center;gap:8px}.dt-logo span{font-size:18px;font-weight:700;color:var(--dt-bright);letter-spacing:-.01em}',
    '.dt-logo-grid{display:grid;grid-template-columns:8px 8px;gap:2px}.dt-logo-grid i{width:8px;height:8px;display:block}',
    '.dt-crumbs{display:flex;align-items:center;gap:2px;font-size:12px;white-space:nowrap}',
    '.dt-crumb{padding:2px 4px;border-radius:4px;color:var(--dt-secondary)}.dt-crumb.is-current{color:var(--dt-bright);font-weight:600;background:' + 'color-mix(in srgb,var(--dt-foreground) 8%,transparent)' + '}',
    '.dt-crumb b{font-weight:400;font-size:11px;color:var(--dt-secondary);margin-left:3px}.dt-sep{color:color-mix(in srgb,var(--dt-secondary) 50%,transparent)}',
    '.dt-spacer{flex:1}',
    '.dt-seg{display:inline-flex;padding:2px;border-radius:6px;background:color-mix(in srgb,var(--dt-foreground) 9%,transparent);box-sizing:border-box;white-space:nowrap}',
    '.dt-seg span{flex:1;text-align:center;padding:4px 12px;border-radius:6px;font-size:12px;color:var(--dt-foreground);line-height:1.2}',
    '.dt-seg span.is-on{background:var(--dt-accent);color:var(--dt-onAccent);font-weight:600;box-shadow:0 1px 2px rgba(0,0,0,.25)}',
    '.dt-seg176{width:176px}.dt-segfull{display:flex;width:100%}',
    '.dt-check{display:flex;align-items:center;gap:6px;white-space:nowrap}.dt-check i{width:16px;height:16px;border-radius:4px;border:1px solid color-mix(in srgb,var(--dt-foreground) 40%,transparent);box-sizing:border-box;display:block;background:var(--dt-surface)}',
    '.dt-check i.is-on{background:var(--dt-accent);border-color:var(--dt-accent)}.dt-check svg{display:block;margin:-1px}',
    '.dt-stepper{display:inline-flex;align-items:stretch;border:1px solid var(--dt-border);border-radius:6px;overflow:hidden;white-space:nowrap;line-height:1.2}',
    '.dt-stepper span{padding:4px 10px}.dt-stepper b{padding:4px 8px;font-weight:400;border-left:1px solid var(--dt-border);color:var(--dt-secondary)}',
    '.dt-body{flex:1;display:flex;min-height:0}',
    '.dt-main{flex:1;min-width:0;display:flex;flex-direction:column;background:var(--dt-background)}',
    '.dt-stats{height:' + STATS_H + 'px;box-sizing:border-box;padding:8px 16px;display:flex;align-items:center;gap:10px;font-size:11px;line-height:16px;color:var(--dt-secondary);white-space:nowrap}.dt-statsl{min-width:0;overflow:hidden;text-overflow:ellipsis}',
    '.dt-legend{margin-left:auto;display:flex;gap:6px;flex:none;letter-spacing:-.015em}.dt-lg{display:inline-flex;align-items:center;gap:3px}.dt-lg i{width:10px;height:10px;display:block;flex:none}',
    '.dt-canvas{position:relative;overflow:hidden;background:var(--dt-inset);margin:0 16px 8px;flex:none}',
    '.dt-tile{position:absolute;overflow:hidden;box-sizing:border-box}.dt-ov{position:absolute;left:0;top:0;right:0;bottom:0;pointer-events:none}.dt-ring{z-index:5}',
    '.dt-lbl{position:absolute;white-space:nowrap;overflow:hidden;font-size:12px;line-height:14px;z-index:1}.dt-name{overflow:hidden;text-overflow:clip;min-width:0}.dt-size{font-size:11px;flex:none}',
    '.dt-side{width:' + SIDE_W + 'px;flex:none;box-sizing:border-box;background:var(--dt-sidebar);border-left:1px solid var(--dt-divider);padding:14px 16px;display:flex;flex-direction:column;gap:10px;overflow:hidden}',
    '.dt-sec{flex:none;display:flex;flex-direction:column;gap:5px;padding-bottom:10px;border-bottom:1px solid var(--dt-divider)}.dt-sec:last-child{border-bottom:0;padding-bottom:0}.dt-disk{margin-top:auto}.dt-look{flex:0 1 auto;min-height:60px;overflow:hidden}',
    '.dt-eyebrow{display:flex;justify-content:space-between;align-items:baseline;font-size:11px;line-height:1.3;text-transform:uppercase;letter-spacing:.06em;color:var(--dt-secondary)}.dt-eyebrow>span:first-child{opacity:.7}.dt-eyebrow-r{text-transform:none;letter-spacing:0}',
    '.dt-hl{color:var(--dt-highlight);font-weight:600}.dt-dim60{opacity:.6}.dt-dim70{opacity:.7;font-size:11px;color:var(--dt-secondary)}',
    '.dt-selname{display:flex;align-items:center;gap:8px}.dt-selname i{width:4px;height:18px;display:block;flex:none;border-radius:1px}.dt-selh{font-size:18px;font-weight:700;color:var(--dt-bright);line-height:1.2}.dt-selp{font-size:11px;color:var(--dt-secondary)}',
    '.dt-big{display:flex;align-items:baseline;gap:6px;line-height:.95;margin:2px 0}.dt-big span{font-size:40px;font-weight:700;color:var(--dt-bright);letter-spacing:-.02em}.dt-big small{font-size:14px;color:var(--dt-secondary)}',
    '.dt-meter{position:relative;background:var(--dt-meterTrack);border-radius:2px;overflow:hidden}.dt-meter i{position:absolute;left:0;top:0;bottom:0;display:block}.dt-meter5{height:5px}.dt-meter8{height:8px;border-radius:4px}.dt-meter4{height:4px}',
    '.dt-figs{display:grid;grid-template-columns:1fr 1fr;gap:3px 12px;line-height:1.35}.dt-figs em{display:block;font-style:normal;font-size:10px;line-height:1.2;text-transform:uppercase;letter-spacing:.06em;color:var(--dt-secondary);opacity:.7}.dt-figs b{font-size:14px;font-weight:500;color:var(--dt-bright);line-height:1.25}',
    '.dt-btns{display:flex;gap:8px;margin-top:2px}.dt-btns>.dt-btn{flex:1}.dt-btns-r{justify-content:flex-end}.dt-btns-r>.dt-btn{flex:none}',
    '.dt-btn{display:inline-block;text-align:center;padding:5px 12px;border-radius:6px;background:var(--dt-fill);border:1px solid var(--dt-border);font-size:12px;line-height:1.3;color:var(--dt-foreground);white-space:nowrap;box-sizing:border-box}',
    '.dt-btn-hl{background:var(--dt-highlight);border-color:var(--dt-highlight);color:var(--dt-onHighlight);font-weight:600}',
    '.dt-btn-accent{background:var(--dt-accent);border-color:var(--dt-accent);color:var(--dt-onAccent);font-weight:600}.dt-btn-bare{background:transparent;border-color:transparent}.dt-btn-sm{padding:3px 10px;font-size:11px}',
    '.dt-rows{display:flex;flex-direction:column}.dt-row{display:flex;align-items:center;gap:8px;padding:2px 8px}.dt-row>i{width:2px;height:24px;display:block;flex:none}',
    '.dt-rowt{flex:1;min-width:0}.dt-rowname{font-size:12px;color:var(--dt-bright);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;line-height:1.2}.dt-rowdet{font-size:11px;color:var(--dt-secondary);line-height:1.2;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}',
    '.dt-rowr{width:88px;flex:none;font-size:11px;color:var(--dt-foreground);text-align:right}.dt-rowr>span{display:block;margin-bottom:2px}',
    '.dt-marks{display:flex;flex-direction:column;gap:2px;font-size:11px}.dt-mark{display:flex;align-items:center;gap:6px}.dt-mark i{width:8px;height:8px;border-radius:50%;display:block;flex:none}.dt-mark span{flex:1;color:var(--dt-foreground);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}.dt-mark small{font-size:11px;color:var(--dt-secondary)}.dt-mark b{font-weight:400;color:var(--dt-secondary);padding-left:4px}',
    '.dt-diskrow{display:flex;justify-content:space-between;align-items:baseline}.dt-free{display:flex;align-items:baseline;gap:5px;line-height:1.1}.dt-free span{font-size:26px;font-weight:700;color:var(--dt-bright);letter-spacing:-.02em}.dt-free small{font-size:12px;color:var(--dt-secondary)}.dt-after{font-size:14px;color:var(--dt-highlight);font-weight:500}',
    '.dt-diskline{display:flex;justify-content:space-between;font-size:11px;line-height:1.3;color:var(--dt-secondary)}.dt-purge{margin-top:-2px}',
    '.dt-review{display:flex;justify-content:space-between;align-items:center;gap:8px;border:1px solid var(--dt-highlight);background:color-mix(in srgb,var(--dt-highlight) 10%,transparent);border-radius:6px;padding:7px 12px;font-size:12px;line-height:1.3;color:var(--dt-bright);margin-top:2px}',
    '.dt-keybar{height:' + KEYBAR_H + 'px;box-sizing:border-box;flex:none;display:flex;align-items:center;gap:16px;padding:4px 16px;border-top:1px solid var(--dt-divider);background:var(--dt-background);white-space:nowrap}',
    '.dt-hint{display:inline-flex;align-items:center;gap:6px;font-size:11px;color:var(--dt-secondary)}',
    '.dt-key{display:inline-block;min-width:24px;box-sizing:border-box;text-align:center;padding:3px 8px;border-radius:4px;border:1px solid var(--dt-border);background:var(--dt-inset);font:700 11px/1.2 -apple-system,BlinkMacSystemFont,"SF Pro Text",system-ui,sans-serif;color:var(--dt-foreground)}',
    // Review screen
    '.dt-rvtitle{font-size:14px;font-weight:700;color:var(--dt-bright)}.dt-rvsub{font-size:11px;color:var(--dt-secondary)}',
    '.dt-rvbody{flex:1;min-height:0;display:flex;gap:16px;padding:16px;background:var(--dt-background)}',
    '.dt-well{flex:1;min-width:0;background:var(--dt-inset);border:1px solid var(--dt-border);border-radius:8px;overflow:hidden;align-self:flex-start;width:100%}',
    '.dt-rvrow{display:flex;align-items:center;gap:12px;padding:8px 12px;border-bottom:1px solid var(--dt-divider)}.dt-rvrow:last-child{border-bottom:0}',
    '.dt-folder{color:var(--dt-secondary);display:flex}.dt-glyph{width:96px;flex:none;font-size:11px;color:var(--dt-secondary);letter-spacing:1px;font-family:"SF Mono",Menlo,monospace}.dt-rvsize{width:80px;flex:none;text-align:right;color:var(--dt-bright);font-size:12px}',
    '.dt-summary{width:360px;flex:none;box-sizing:border-box;background:var(--dt-surface);border:1px solid var(--dt-border);border-radius:8px;padding:16px;display:flex;flex-direction:column;gap:24px}',
    '.dt-sum h4{margin:0 0 8px;font-size:11px;font-weight:600;text-transform:uppercase;letter-spacing:.06em;color:var(--dt-secondary)}.dt-sum p{margin:8px 0 0;font-size:11px;color:var(--dt-secondary)}',
    '.dt-tot{display:flex;justify-content:space-between;font-size:11px;line-height:1.7}.dt-tot span{color:var(--dt-secondary)}.dt-tot b{font-weight:500;color:var(--dt-bright)}',
    // Treemap-only preview
    '.dt-mini{box-sizing:border-box;background:var(--dt-background);padding:8px;display:flex;flex-direction:column;gap:6px;border-radius:8px}',
    '.dt-mini .dt-canvas{margin:0}.dt-mini .dt-legend{margin:0;flex-wrap:wrap;font-size:11px;color:var(--dt-secondary);gap:10px}',
  ].join('\n');

  function ensureStyle() {
    if (document.getElementById('dt-mock-style')) return;
    var st = document.createElement('style');
    st.id = 'dt-mock-style';
    st.textContent = CSS;
    document.head.appendChild(st);
  }

  // Mounts a stage of design size (w, h) into el and keeps it scaled to el's
  // width. Re-rendering into the same el replaces the previous stage.
  function mount(el, w, h, html, vars, cls) {
    ensureStyle();
    if (el.__dtRO) { el.__dtRO.disconnect(); el.__dtRO = null; }
    if (el.__dtOnResize) { window.removeEventListener('resize', el.__dtOnResize); el.__dtOnResize = null; }
    el.classList.add('dt-frame');
    el.innerHTML = '<div class="dt-stage ' + (cls || '') + '" style="width:' + w + 'px;height:' + h + 'px;' + vars + '">' + html + '</div>';
    var stage = el.firstChild;
    function fit() {
      var cw = el.clientWidth || w;
      var s = cw / w;
      stage.style.transform = 'scale(' + s + ')';
      el.style.height = (h * s) + 'px';
    }
    fit();
    if (typeof ResizeObserver !== 'undefined') {
      el.__dtRO = new ResizeObserver(fit);
      el.__dtRO.observe(el);
    } else {
      el.__dtOnResize = fit;
      window.addEventListener('resize', fit);
    }
    return { fit: fit, stage: stage };
  }

  function windowStage(ctx, inner) {
    return '<div class="dt-desktop" style="width:' + (WIN_W + 2 * PAD) + 'px;height:' + (WIN_H + 2 * PAD) + 'px">' +
      '<div class="dt-win' + (ctx.shape.vibrancy ? ' dt-vib' : '') + '">' + inner + '</div></div>';
  }

  function renderExplore(el, palette, appearance, opts) {
    var ctx = context(palette, appearance, opts);
    return mount(el, WIN_W + 2 * PAD, WIN_H + 2 * PAD, windowStage(ctx, exploreHTML(ctx)), varsFor(ctx));
  }
  function renderReview(el, palette, appearance) {
    var ctx = context(palette, appearance, {});
    return mount(el, WIN_W + 2 * PAD, WIN_H + 2 * PAD, windowStage(ctx, reviewHTML(ctx)), varsFor(ctx));
  }
  function renderTreemapOnly(el, palette, appearance, opts) {
    var ctx = context(palette, appearance, opts);
    var cw = MINI_W - 16, ch = MINI_H - 16 - 6 - 18;
    var html = '<div class="dt-mini" style="width:' + MINI_W + 'px;height:' + MINI_H + 'px">' +
      '<div class="dt-canvas" style="width:' + cw + 'px;height:' + ch + 'px">' + treemapHTML(ctx, cw, ch) + '</div>' +
      legendHTML(ctx) + '</div>';
    return mount(el, MINI_W, MINI_H, html, varsFor(ctx));
  }

  window.DisktreeMock = {
    renderExplore: renderExplore,
    renderReview: renderReview,
    renderTreemapOnly: renderTreemapOnly,
    tree: TREE,
    categories: CATS,
    categoryLabels: CAT_LABELS,
    ageLabels: AGE_LABELS,
    design: { width: WIN_W + 2 * PAD, height: WIN_H + 2 * PAD, window: [WIN_W, WIN_H] },
  };
})();
