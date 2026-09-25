//! The menu bar and the ⌘ shortcuts.
//!
//! Every action here is a name for something `Disktree` already does from
//! a plain key; the menu and the ⌘ chords are a second way in, never a
//! second implementation. The bindings live in the
//! keymap so the menu bar shows them beside the items and so a bound chord
//! is claimed before it can reach the key listener and type into the find
//! field.

use gpui_kit::{App, KeyBinding, Menu, MenuItem, actions};

// `Eq` on top of the macro's `PartialEq`, as the lints ask of every unit
// type.
actions!(
    disktree,
    [
        #[derive(Eq)]
        Quit,
        #[derive(Eq)]
        CloseWindow,
        #[derive(Eq)]
        OpenFolder,
        #[derive(Eq)]
        ZoomIn,
        #[derive(Eq)]
        ZoomOut,
        #[derive(Eq)]
        ZoomReset,
        #[derive(Eq)]
        Rescan,
        #[derive(Eq)]
        WholeDisk,
        #[derive(Eq)]
        FocusFind,
        #[derive(Eq)]
        ToggleReview,
        #[derive(Eq)]
        ShowHelp,
    ]
);

/// Bind the chords. Called in tests too, so a test pressing `cmd-=` goes
/// through the same keymap the app does.
///
/// The `ctrl` spellings stay for a keyboard without a ⌘ key; nothing else
/// claims them.
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-o", OpenFolder, None),
        KeyBinding::new("cmd-=", ZoomIn, None),
        KeyBinding::new("cmd-+", ZoomIn, None),
        KeyBinding::new("ctrl-=", ZoomIn, None),
        KeyBinding::new("ctrl-+", ZoomIn, None),
        KeyBinding::new("cmd--", ZoomOut, None),
        KeyBinding::new("ctrl--", ZoomOut, None),
        KeyBinding::new("cmd-0", ZoomReset, None),
        KeyBinding::new("ctrl-0", ZoomReset, None),
        KeyBinding::new("cmd-r", Rescan, None),
        KeyBinding::new("cmd-shift-d", WholeDisk, None),
        KeyBinding::new("cmd-f", FocusFind, None),
        KeyBinding::new("cmd-/", ShowHelp, None),
        KeyBinding::new("cmd-?", ShowHelp, None),
    ]);
}

/// The menu bar, in the order macOS expects: the application menu, File,
/// View, Help. There is no Edit menu because nothing here is edited.
fn menus() -> Vec<Menu> {
    vec![
        Menu::new("Disktree").items([
            MenuItem::action("Keyboard Shortcuts", ShowHelp),
            MenuItem::separator(),
            MenuItem::action("Quit Disktree", Quit),
        ]),
        Menu::new("File").items([
            MenuItem::action("Open Folder\u{2026}", OpenFolder),
            MenuItem::separator(),
            MenuItem::action("Close Window", CloseWindow),
        ]),
        Menu::new("View").items([
            MenuItem::action("Zoom In", ZoomIn),
            MenuItem::action("Zoom Out", ZoomOut),
            MenuItem::action("Actual Size", ZoomReset),
            MenuItem::separator(),
            MenuItem::action("Find\u{2026}", FocusFind),
            MenuItem::action("Review Marked", ToggleReview),
            MenuItem::separator(),
            MenuItem::action("Rescan", Rescan),
            MenuItem::action("Whole Disk", WholeDisk),
        ]),
        Menu::new("Help")
            .items([MenuItem::action("Keyboard Shortcuts", ShowHelp)]),
    ]
}

/// Install the keymap, the menu bar and the app-level handlers.
///
/// Quit is the one action with no window to land in, so it is handled
/// here; everything else is answered by the window's root element, which
/// has the state in hand.
pub fn init(cx: &mut App) {
    bind_keys(cx);
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.set_menus(menus());
}
