//! The app's palette, and its projection into gpui-base.
//!
//! A theme is a [`ThemeId`]: one style drawn as a light and a dark palette,
//! the Catppuccin styles with a choice of dark [`Flavour`]. The palettes
//! themselves are tables in [`crate::themes`], generated and checked in
//! `design/palettes`; [`Theme::resolve`] reads the one a [`ThemeChoice`]
//! names and turns its colours into the tokens the views draw with. Which
//! half is on screen follows the [`AppearanceChoice`]: the system's
//! appearance by default, and `main` re-applies on every change the window
//! reports.
//!
//! Every palette keeps one strong colour apart, the highlight: the treemap
//! uses it for the selection, the main action and reclaimable space, and
//! for nothing else, so the eye goes straight to it.

use gpui_kit::base::actions::{Confirm, SelectLeft, SelectRight};
use gpui_kit::base::{ColorTokens, RadiusTokens, ThemeAppearance};
use gpui_kit::{
    App, Global, Hsla, KeyBinding, SharedString, WindowAppearance, px, rgba,
};

use crate::themes;
use crate::ui::{BASE_REM, radius, text};

/// The face the whole interface is set in: the system font on macOS.
///
/// GPUI maps `.SystemUIFont` to `.AppleSystemUIFont`, which is how `AppKit`
/// itself names San Francisco, so this resolves on every macOS without an
/// installed font of that name.
const SYSTEM_FONT: &str = ".SystemUIFont";

/// Preferred and guaranteed monospace faces. SF Mono only exists
/// system-wide where Terminal or Xcode has registered it, so it is checked
/// for before use; Menlo ships with every macOS.
const MONO_FONT: &str = "SF Mono";
const MONO_FALLBACK: &str = "Menlo";

/// The key context in which a choice group listens for arrow keys.
pub const OPTION_GROUP_CONTEXT: &str = "DisktreeOptionGroup";

/// A theme the person picks in Settings: one style, drawn as a light and a
/// dark palette. Which of the two is on screen is [`AppearanceChoice`]'s
/// call, not the theme's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThemeId {
    #[default]
    Catppuccin,
    CatppuccinPastel,
    CatppuccinQuiet,
    Aqua,
    Graphite,
    RosePine,
}

#[expect(dead_code, reason = "Settings and the View menu use it next")]
impl ThemeId {
    /// Every shipped theme, in the order Settings and the menu list them.
    pub const ALL: [Self; 6] = [
        Self::Catppuccin,
        Self::CatppuccinPastel,
        Self::CatppuccinQuiet,
        Self::Aqua,
        Self::Graphite,
        Self::RosePine,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Catppuccin => "Catppuccin",
            Self::CatppuccinPastel => "Catppuccin Pastel",
            Self::CatppuccinQuiet => "Catppuccin Quiet",
            Self::Aqua => "Aqua",
            Self::Graphite => "Graphite",
            Self::RosePine => "Ros\u{e9} Pine",
        }
    }

    /// The name written to the settings file. Kept apart from [`name`] so
    /// a display name can change without forgetting what people chose.
    ///
    /// [`name`]: Self::name
    pub const fn key(self) -> &'static str {
        match self {
            Self::Catppuccin => "catppuccin",
            Self::CatppuccinPastel => "catppuccin-pastel",
            Self::CatppuccinQuiet => "catppuccin-quiet",
            Self::Aqua => "aqua",
            Self::Graphite => "graphite",
            Self::RosePine => "rose-pine",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|id| id.key() == key)
    }

    /// Catppuccin's dark palette comes in three flavours; light is always
    /// Latte. The other themes have one dark palette.
    pub const fn has_flavours(self) -> bool {
        matches!(
            self,
            Self::Catppuccin | Self::CatppuccinPastel | Self::CatppuccinQuiet
        )
    }
}

/// Which Catppuccin flavour a Catppuccin theme is dark in. Ignored by the
/// themes without flavours.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Flavour {
    #[default]
    Frappe,
    Macchiato,
    Mocha,
}

#[expect(dead_code, reason = "Settings and the View menu use it next")]
impl Flavour {
    pub const ALL: [Self; 3] = [Self::Frappe, Self::Macchiato, Self::Mocha];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Frappe => "Frapp\u{e9}",
            Self::Macchiato => "Macchiato",
            Self::Mocha => "Mocha",
        }
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::Frappe => "frappe",
            Self::Macchiato => "macchiato",
            Self::Mocha => "mocha",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|flavour| flavour.key() == key)
    }
}

/// Light, dark, or whatever System Settings says.
#[expect(dead_code, reason = "Settings and the View menu use it next")]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AppearanceChoice {
    #[default]
    System,
    Light,
    Dark,
}

#[expect(dead_code, reason = "Settings and the View menu use it next")]
impl AppearanceChoice {
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    pub const fn name(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|choice| choice.key() == key)
    }

    /// The palette half to draw, given what the window reports. The
    /// vibrant variants are the same two appearances seen through a
    /// translucent material.
    pub const fn resolve(self, window: WindowAppearance) -> ThemeAppearance {
        match (self, window) {
            (Self::Light, _)
            | (
                Self::System,
                WindowAppearance::Light | WindowAppearance::VibrantLight,
            ) => ThemeAppearance::Light,
            (Self::Dark, _)
            | (
                Self::System,
                WindowAppearance::Dark | WindowAppearance::VibrantDark,
            ) => ThemeAppearance::Dark,
        }
    }
}

/// What the person chose in Settings. The installed [`Theme`] is derived
/// from it and the window's appearance, and re-derived when either changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ThemeChoice {
    pub theme: ThemeId,
    pub flavour: Flavour,
    pub appearance: AppearanceChoice,
}
impl Global for ThemeChoice {}

/// The current choice, or the default before one is set.
pub fn choice(cx: &App) -> ThemeChoice {
    cx.try_global::<ThemeChoice>().copied().unwrap_or_default()
}

/// Make `choice` the current one and redraw every window in it.
#[expect(dead_code, reason = "Settings and the View menu use it next")]
pub fn select(choice: ThemeChoice, cx: &mut App) {
    cx.set_global(choice);
    refresh(cx.window_appearance(), cx);
}

/// Re-derive the installed palette from the current choice and what the
/// window reports: called at launch, on a new choice, and whenever the
/// system appearance changes.
pub fn refresh(window: WindowAppearance, cx: &mut App) {
    let choice = choice(cx);
    Theme::resolve(choice, choice.appearance.resolve(window)).apply(cx);
}

/// The colours and tile geometry on screen: one half of a [`ThemeId`],
/// resolved from the tables in [`themes`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    /// The theme and the half: "Catppuccin Frappé", "Aqua Dark".
    pub name: SharedString,
    pub appearance: ThemeAppearance,
    /// Window chrome: the panel, the bars, the review summary.
    pub background: Hsla,
    /// A raised sheet: menus, dialogs, the help overlay.
    pub surface: Hsla,
    /// The side panel.
    pub sidebar: Hsla,
    /// A sunken well: the treemap canvas, scrolling lists, key caps.
    pub inset: Hsla,
    pub foreground: Hsla,
    pub secondary: Hsla,
    pub bright: Hsla,
    pub accent: Hsla,
    pub on_accent: Hsla,
    pub selection: Hsla,
    pub border: Hsla,
    /// A rule between regions of the chrome.
    pub divider: Hsla,
    /// The resting fill of a control.
    pub fill: Hsla,
    /// The one strong colour: selection, the main action, what can be had
    /// back.
    pub highlight: Hsla,
    pub on_highlight: Hsla,
    pub danger: Hsla,
    pub warning: Hsla,
    pub success: Hsla,
    /// The diagonal hatch over reclaimable space.
    pub hatch: Hsla,
    /// The outline of the tile under the pointer.
    pub hover: Hsla,
    /// A tile's name on its fill, and the dimmer size beside it.
    pub label: Hsla,
    pub label_dim: Hsla,
    /// A tile marked for removal: its fill, its ring, its name.
    pub marked_fill: Hsla,
    pub marked_outline: Hsla,
    pub marked_text: Hsla,
    /// The disk meter: the whole volume, and the part in use.
    pub meter_track: Hsla,
    pub meter_used: Hsla,
    /// The category and age fills, read by [`crate::palette`].
    pub tiles: &'static themes::Palette,
    /// The tile geometry: corners, gaps, the strip, the sheen.
    pub shape: &'static themes::Shape,
    pub font: SharedString,
    pub mono: SharedString,
}
impl Global for Theme {}

impl Theme {
    /// The palette `choice` draws in `appearance`.
    pub fn resolve(choice: ThemeChoice, appearance: ThemeAppearance) -> Self {
        let tiles = themes::palette(choice.theme, choice.flavour, appearance);
        let ui = &tiles.ui;
        let hsla = |color: themes::Color| Hsla::from(rgba(color));
        let accent = hsla(ui.accent);
        // The selection tint is the accent seen through the text it sits
        // behind; a dark canvas needs more of it to show at all.
        let selection = match appearance {
            ThemeAppearance::Light => accent.opacity(0.18),
            ThemeAppearance::Dark => accent.opacity(0.28),
        };
        Self {
            name: format!("{} {}", choice.theme.name(), tiles.name).into(),
            appearance,
            background: hsla(ui.background),
            surface: hsla(ui.surface),
            sidebar: hsla(ui.sidebar),
            inset: hsla(ui.inset),
            foreground: hsla(ui.foreground),
            secondary: hsla(ui.secondary),
            bright: hsla(ui.bright),
            accent,
            on_accent: hsla(ui.on_accent),
            selection,
            border: hsla(ui.border),
            divider: hsla(ui.divider),
            fill: hsla(ui.fill),
            highlight: hsla(ui.highlight),
            on_highlight: hsla(ui.on_highlight),
            danger: hsla(ui.danger),
            warning: hsla(ui.warning),
            success: hsla(ui.success),
            hatch: hsla(ui.hatch),
            hover: hsla(ui.hover),
            label: hsla(ui.label),
            label_dim: hsla(ui.label_dim),
            marked_fill: hsla(ui.marked_fill),
            marked_outline: hsla(ui.marked_outline),
            marked_text: hsla(ui.marked_text),
            meter_track: hsla(ui.meter_track),
            meter_used: hsla(ui.meter_used),
            tiles,
            shape: themes::shape(choice.theme),
            font: SYSTEM_FONT.into(),
            mono: MONO_FONT.into(),
        }
    }

    /// No paint at all, in the label colour so a hover that later fills the
    /// same edge or face animates from the right hue.
    pub fn transparent(&self) -> Hsla {
        self.foreground.opacity(0.)
    }

    /// Shared control colours: the resting fill is the palette's own, the
    /// rest are tints of the label colour so they read the same way over
    /// either window background.
    pub const fn normal_fill(&self) -> Hsla {
        self.fill
    }
    pub fn hover_fill(&self) -> Hsla {
        self.foreground.opacity(0.08)
    }
    pub fn selected_fill(&self) -> Hsla {
        self.foreground.opacity(0.18)
    }
    pub fn pressed_fill(&self) -> Hsla {
        self.foreground.opacity(0.22)
    }
    pub fn control_border(&self) -> Hsla {
        self.foreground.opacity(0.4)
    }
    pub fn focus_border(&self) -> Hsla {
        self.foreground.opacity(0.25)
    }

    /// The palette as gpui-base sees it, so base's own dialogs and tooltips
    /// pick up the same colours.
    pub const fn tokens(&self) -> ColorTokens {
        ColorTokens {
            background: self.background,
            foreground: self.foreground,
            surface: self.surface,
            surface_foreground: self.foreground,
            primary: self.accent,
            primary_foreground: self.on_accent,
            secondary: self.surface,
            secondary_foreground: self.foreground,
            muted: self.inset,
            muted_foreground: self.secondary,
            accent: self.selection,
            accent_foreground: self.bright,
            destructive: self.danger,
            destructive_foreground: self.on_accent,
            border: self.border,
            input: self.border,
            ring: self.accent,
            selection: self.selection,
        }
    }

    /// Install this palette: base tokens, the app global, and a redraw of
    /// every window.
    pub fn apply(mut self, cx: &mut App) {
        if self.mono.as_ref() == MONO_FONT && !has_font(cx, MONO_FONT) {
            self.mono = MONO_FALLBACK.into();
        }
        let base = gpui_kit::base::Theme::global_mut(cx);
        base.appearance = self.appearance;
        base.tokens.colors = self.tokens();
        // gpui-base keeps its tokens in pixels; these are the 100% zoom
        // snapshot of the rem tokens the app lays out with.
        let to_px = |rems: gpui_kit::Rems| rems.to_pixels(px(BASE_REM));
        base.tokens.radius = RadiusTokens {
            none: px(0.),
            sm: to_px(radius::KEYCAP),
            md: to_px(radius::CONTROL),
            lg: to_px(radius::SURFACE),
            xl: to_px(radius::DIALOG),
            full: px(9999.),
        };
        let type_scale = &mut base.tokens.typography;
        type_scale.sans = self.font.clone();
        type_scale.mono = self.mono.clone();
        for (token, size) in [
            (&mut type_scale.xs, text::CAPTION),
            (&mut type_scale.sm, text::CAPTION),
            (&mut type_scale.md, text::BODY),
            (&mut type_scale.lg, text::TITLE),
            (&mut type_scale.xl, text::HEADING),
            (&mut type_scale.mono_md, text::BODY),
        ] {
            token.size = to_px(size);
            token.line_height = token.size * 1.5;
        }
        cx.set_global(self);
        cx.refresh_windows();
    }
}

fn has_font(cx: &App, family: &str) -> bool {
    cx.text_system()
        .all_font_names()
        .iter()
        .any(|name| name == family)
}

pub trait ActiveTheme {
    fn theme(&self) -> &Theme;
}
impl ActiveTheme for App {
    fn theme(&self) -> &Theme {
        self.global::<Theme>()
    }
}

/// Bring up gpui-base, the choice-group keys, and the palette matching the
/// system's current appearance.
pub fn init(cx: &mut App) {
    gpui_kit::base::init(cx);
    cx.bind_keys([
        KeyBinding::new("left", SelectLeft, Some(OPTION_GROUP_CONTEXT)),
        KeyBinding::new("h", SelectLeft, Some(OPTION_GROUP_CONTEXT)),
        KeyBinding::new("right", SelectRight, Some(OPTION_GROUP_CONTEXT)),
        KeyBinding::new("l", SelectRight, Some(OPTION_GROUP_CONTEXT)),
        KeyBinding::new(
            "enter",
            Confirm { secondary: false },
            Some(OPTION_GROUP_CONTEXT),
        ),
        KeyBinding::new(
            "space",
            Confirm { secondary: false },
            Some(OPTION_GROUP_CONTEXT),
        ),
    ]);
    refresh(cx.window_appearance(), cx);
}
