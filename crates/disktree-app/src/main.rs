//! `disktree`: find what is eating a volume, mark it, and remove it.
//!
//! The window opens on a treemap of the scanned root — the home directory
//! unless another path is given — with a breadcrumb bar, a selection line, and a
//! live free-space meter. Marking is non-destructive until the review screen
//! is confirmed.

mod controls;
mod git;
mod marks;
mod menu;
mod palette;
mod state;
#[cfg(test)]
mod tests;
mod theme;
mod treemap_view;
mod ui;
mod views;
mod widgets;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::{Context as _, Result};
use disktree_core::scan::ScanOptions;
use gpui_kit::{
    AppContext as _, AsyncApp, WindowHandle, WindowOptions, px, size,
};
use state::Disktree;

/// What the command line asked for.
#[derive(Debug)]
struct Args {
    root: PathBuf,
    options: ScanOptions,
    depth: u32,
}

const USAGE: &str = "\
disktree — a treemap of what is using your disk

usage: disktree [OPTIONS] [PATH]

arguments:
  PATH              directory to scan (default: the home directory)

The window opens on a treemap of the root, largest first. Space marks the
selected tile, Enter opens it, c reviews the marked list, ? lists every key.

options:
  -a, --apparent-size   measure apparent length instead of allocated blocks
  -l, --follow-links    follow symlinks
  -H, --no-hidden       skip dotfiles and dot-directories
  -D, --disk            scan the whole disk the home directory is on
  -x, --one-filesystem  stay on PATH's volume (the default; kept for old
                        invocations)
  -X, --cross-filesystems
                        also measure other disks, network shares and pseudo
                        filesystems mounted below PATH (off by default)
  -d, --depth N         how many levels to draw at once (1-6, default 3)
      --metric files    rank by file count instead of bytes
  -h, --help            show this help
";

fn main() -> Result<()> {
    let args = parse_args()?;
    let depth = args.depth;

    // The Finder's Open With and a folder dropped on the Dock icon arrive
    // as file URLs, possibly before the window exists.
    let opened = Rc::new(RefCell::new(OpenRequests::default()));
    let app = gpui_kit::application().with_assets(gpui_kit::assets::Assets);
    app.on_open_urls({
        let opened = Rc::clone(&opened);
        move |urls| opened.borrow_mut().receive(&urls)
    });
    app.run(move |cx| {
        theme::init(cx);
        menu::init(cx);
        // A folder the platform handed over at launch wins over the
        // command line: it is what the person just pointed at.
        let root = opened
            .borrow_mut()
            .take_pending()
            .unwrap_or_else(|| args.root.clone());
        let title = format!(
            "Disktree \u{b7} {}",
            marks::display_path(
                &root,
                std::env::var_os("HOME").map(PathBuf::from).as_deref(),
            )
        );
        let options = args.options.clone();
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(gpui_kit::WindowBounds::Windowed(
                        gpui_kit::Bounds::new(
                            gpui_kit::point(px(120.), px(90.)),
                            size(px(1440.), px(900.)),
                        ),
                    )),
                    // The top row is drawn in the title bar's band, as
                    // Finder and Xcode draw theirs, so the window keeps
                    // no separate bar that repeats the trail. The title
                    // stays set: Mission Control, the Dock and ⌘⇥ show
                    // it.
                    titlebar: Some(gpui_kit::TitlebarOptions {
                        title: Some(title.into()),
                        appears_transparent: true,
                        traffic_light_position: Some(
                            ui::chrome::traffic_light_position(ui::BASE_REM),
                        ),
                    }),
                    // The bar moves the window and answers a double click
                    // itself; AppKit would otherwise do both as well, and
                    // hold every click in the band while it waits for a
                    // second one.
                    app_owns_titlebar_drag: true,
                    // Below this the treemap stops being readable, so ask
                    // the compositor not to go there.
                    window_min_size: Some(size(px(900.), px(600.))),
                    ..Default::default()
                },
                move |_, cx| {
                    cx.new(|cx| {
                        Disktree::new(root.clone(), options.clone(), depth, cx)
                    })
                },
            )
            .expect("open the disktree window");

        // The treemap owns the keyboard from the first frame; there is no
        // text field to focus first.
        let _ = window.update(cx, |this, window, cx| {
            let focus = this.focus.clone();
            window.focus(&focus, cx);
            // The palette was chosen from the platform's appearance
            // before the window existed; from here on the window's own
            // reports keep it in step with System Settings.
            theme::Theme::for_appearance(window.appearance()).apply(cx);
            window
                .observe_window_appearance(|window, cx| {
                    theme::Theme::for_appearance(window.appearance()).apply(cx);
                })
                .detach();
        });
        // One window is the whole app: there is no document to keep open
        // behind it, so closing it (⌘W, the red button) quits rather than
        // leaving a bare menu bar.
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        opened.borrow_mut().attach(cx.to_async(), window);
        // Launched from the bundle, the window would otherwise open behind
        // whatever was in front.
        cx.activate(true);
    });
    Ok(())
}

/// Folders the platform asks to open, and where to deliver them.
#[derive(Default)]
struct OpenRequests {
    /// A folder that arrived before the window existed.
    pending: Option<PathBuf>,
    /// The window to hand later folders to, once there is one.
    target: Option<(AsyncApp, WindowHandle<Disktree>)>,
}

impl OpenRequests {
    /// The platform's `openURLs`: the last folder named wins, and anything
    /// that is not a folder is ignored.
    fn receive(&mut self, urls: &[String]) {
        let Some(path) = urls.iter().rev().find_map(|url| folder_from_url(url))
        else {
            return;
        };
        match &self.target {
            None => self.pending = Some(path),
            Some((cx, window)) => cx.update(|cx| {
                window
                    .update(cx, |this, window, cx| {
                        this.open_folder(path, cx);
                        window.set_window_title(&this.window_title());
                    })
                    .ok();
                cx.activate(true);
            }),
        }
    }

    const fn take_pending(&mut self) -> Option<PathBuf> {
        self.pending.take()
    }

    fn attach(&mut self, cx: AsyncApp, window: WindowHandle<Disktree>) {
        self.target = Some((cx, window));
    }
}

/// The directory a `file://` URL names, or `None` for anything else: a
/// file, a remote URL, or an escape that is not UTF-8.
fn folder_from_url(url: &str) -> Option<PathBuf> {
    let rest = url.strip_prefix("file://")?;
    // `file://localhost/path` and `file:///path` both name a local path.
    let path = rest.strip_prefix("localhost").unwrap_or(rest);
    if !path.starts_with('/') {
        return None;
    }
    let path = PathBuf::from(percent_decode(path)?);
    path.is_dir().then_some(path)
}

/// Undo the URL's `%XX` escapes, such as the `%20` in a folder name with a
/// space. Rejects an escape that is not a byte, and a result that is not
/// UTF-8, since a macOS path always is.
fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3)?;
            let hex = std::str::from_utf8(hex).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn parse_args() -> Result<Args> {
    let mut root: Option<PathBuf> = None;
    let mut options = ScanOptions::default();
    let mut depth = 3_u32;
    let mut disk = false;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            "-a" | "--apparent-size" => options.apparent_size = true,
            "-l" | "--follow-links" => options.follow_links = true,
            "-H" | "--no-hidden" => options.include_hidden = false,
            // Staying on one volume is the default; the flag is kept so
            // old invocations still work.
            "-x" | "--one-filesystem" => options.one_filesystem = true,
            "-X" | "--cross-filesystems" => options.one_filesystem = false,
            "-D" | "--disk" => disk = true,
            "-d" | "--depth" => {
                let value = args.next().context("--depth needs a number")?;
                depth = value.parse().context("--depth needs a number")?;
                anyhow::ensure!(
                    (1..=6).contains(&depth),
                    "--depth must be 1 to 6"
                );
            }
            "--metric" => {
                let value = args.next().context("--metric needs a value")?;
                options.metric = match value.as_str() {
                    "files" => disktree_core::tree::Metric::Files,
                    "bytes" | "size" => disktree_core::tree::Metric::Bytes,
                    other => anyhow::bail!(
                        "unknown metric {other}; try bytes or files"
                    ),
                };
            }
            other if other.starts_with('-') => {
                anyhow::bail!("unknown option {other}\n\n{USAGE}");
            }
            path => {
                anyhow::ensure!(root.is_none(), "only one path can be scanned");
                root = Some(PathBuf::from(path));
            }
        }
    }

    anyhow::ensure!(
        !(disk && root.is_some()),
        "--disk and a PATH cannot be combined"
    );
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let root = match root {
        _ if disk => home
            .as_deref()
            .and_then(disktree_core::space::volume_root_for)
            .unwrap_or_else(|| PathBuf::from("/")),
        Some(root) => root,
        None => std::env::var_os("HOME")
            .map(PathBuf::from)
            .context("no path given and HOME is not set")?,
    };
    // Store the depth as the initial view setting rather than a scan option: it
    // is a display choice the run-time `[` and `]` keys also change.
    // Canonical, so a later widening recognises this tree in the wider walk.
    let root = root.canonicalize().unwrap_or(root);
    let metadata = std::fs::metadata(&root)
        .with_context(|| format!("cannot read {}", root.display()))?;
    anyhow::ensure!(metadata.is_dir(), "{} is not a directory", root.display());

    Ok(Args {
        root,
        options,
        depth: depth.clamp(1, 6),
    })
}
