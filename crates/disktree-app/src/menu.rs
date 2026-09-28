//! The menu bar and the ⌘ shortcuts.
//!
//! Every action here is a name for something `Disktree` already does from
//! a plain key; the menu and the ⌘ chords are a second way in, never a
//! second implementation. The bindings live in the
//! keymap so the menu bar shows them beside the items and so a bound chord
//! is claimed before it can reach the key listener and type into the find
//! field.
//!
//! The theme items are the exception: they carry their choice with them
//! and are answered here, at the app level, so they work from whichever
//! window is in front, including Settings. The menu bar is static once
//! set, so [`sync`] sets it again whenever the choice changes and the
//! check marks follow.

use gpui_kit::{App, KeyBinding, Menu, MenuItem, actions};

use crate::settings;
use crate::theme::{self, AppearanceChoice, Flavour, ThemeChoice, ThemeId};

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
        OpenSettings,
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

/// Pick a theme. One action with the theme inside it rather than six unit
/// actions: the menu is built from `ThemeId::ALL`, so adding a theme is one
/// line in `theme.rs` and nothing here. `no_json` because nothing loads
/// these from a keymap file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, gpui_kit::Action)]
#[action(namespace = disktree, no_json)]
pub struct SelectTheme(pub ThemeId);

/// Light, dark, or the system's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, gpui_kit::Action)]
#[action(namespace = disktree, no_json)]
pub struct SelectAppearance(pub AppearanceChoice);

/// The dark flavour of a Catppuccin theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, gpui_kit::Action)]
#[action(namespace = disktree, no_json)]
pub struct SelectFlavour(pub Flavour);

/// Bind the chords. Called in tests too, so a test pressing `cmd-=` goes
/// through the same keymap the app does.
///
/// The `ctrl` spellings stay for a keyboard without a ⌘ key; nothing else
/// claims them.
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-,", OpenSettings, None),
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
        // Settings is a sheet-like window: Escape closes it, as it does
        // a dialog. The treemap keeps its own Escape.
        KeyBinding::new(
            "escape",
            CloseWindow,
            Some(crate::settings_view::CONTEXT),
        ),
        KeyBinding::new(
            "tab",
            crate::settings_view::NextControl,
            Some(crate::settings_view::CONTEXT),
        ),
        KeyBinding::new(
            "shift-tab",
            crate::settings_view::PreviousControl,
            Some(crate::settings_view::CONTEXT),
        ),
    ]);
}

/// The menu bar, in the order macOS expects: the application menu, File,
/// View, Help. There is no Edit menu because nothing here is edited.
pub fn menus(choice: ThemeChoice) -> Vec<Menu> {
    vec![
        Menu::new("Disktree").items([
            MenuItem::action("Keyboard Shortcuts", ShowHelp),
            MenuItem::separator(),
            MenuItem::action("Settings\u{2026}", OpenSettings),
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
            MenuItem::submenu(theme_menu(choice)),
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

/// View ▸ Theme: every theme, then the appearance, then, for a theme that
/// has them, the dark flavours; the current one of each is checked.
fn theme_menu(choice: ThemeChoice) -> Menu {
    let mut items: Vec<MenuItem> = ThemeId::ALL
        .into_iter()
        .map(|id| {
            MenuItem::action(id.name(), SelectTheme(id))
                .checked(choice.theme == id)
        })
        .collect();
    items.push(MenuItem::separator());
    items.extend(AppearanceChoice::ALL.into_iter().map(|appearance| {
        MenuItem::action(appearance.name(), SelectAppearance(appearance))
            .checked(choice.appearance == appearance)
    }));
    if choice.theme.has_flavours() {
        items.push(MenuItem::separator());
        items.extend(Flavour::ALL.into_iter().map(|flavour| {
            MenuItem::action(flavour.name(), SelectFlavour(flavour))
                .checked(choice.flavour == flavour)
        }));
    }
    Menu::new("Theme").items(items)
}

/// Set the menu bar for the current choice. The platform keeps a copy, so
/// this is the only way a check mark moves.
pub fn sync(cx: &App) {
    cx.set_menus(menus(theme::choice(cx)));
}

/// Install the keymap, the menu bar and the app-level handlers.
///
/// Quit, Settings and the theme items have no window to land in, so they
/// are handled here; everything else is answered by the window's root
/// element, which has the state in hand.
pub fn init(cx: &mut App) {
    bind_keys(cx);
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &OpenSettings, cx| {
        crate::settings_view::open(cx);
    });
    cx.on_action(|action: &SelectTheme, cx| {
        let choice = ThemeChoice {
            theme: action.0,
            ..theme::choice(cx)
        };
        settings::choose(choice, cx);
    });
    cx.on_action(|action: &SelectAppearance, cx| {
        let choice = ThemeChoice {
            appearance: action.0,
            ..theme::choice(cx)
        };
        settings::choose(choice, cx);
    });
    cx.on_action(|action: &SelectFlavour, cx| {
        let choice = ThemeChoice {
            flavour: action.0,
            ..theme::choice(cx)
        };
        settings::choose(choice, cx);
    });
    sync(cx);
}
