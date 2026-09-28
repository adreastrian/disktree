//! The app's palette, and its projection into gpui-base.
//!
//! Two palettes, one per system appearance, built from the colours macOS
//! draws its own windows with, so the app sits beside Finder rather than
//! beside a terminal. Which one is active follows the system: [`init`]
//! picks by the platform's appearance, and `main` re-applies on every change
//! the window reports.
//!
//! The one deliberate departure from the system palette is amber: the
//! treemap uses `warning` as its single highlight (selection, the main
//! action, reclaimable space), so both palettes keep it the system's orange
//! rather than muting it into a caution tint.

use gpui_kit::base::actions::{Confirm, SelectLeft, SelectRight};
use gpui_kit::base::{ColorTokens, RadiusTokens, ThemeAppearance};
use gpui_kit::{
    App, Global, Hsla, KeyBinding, SharedString, WindowAppearance, px, rgb,
    rgba,
};

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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AppearanceChoice {
    #[default]
    System,
    Light,
    Dark,
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pub name: SharedString,
    pub appearance: ThemeAppearance,
    /// Window chrome: the panel, the bars, the review summary.
    pub background: Hsla,
    /// A raised sheet: menus, dialogs, the help overlay.
    pub surface: Hsla,
    /// A sunken well: the treemap canvas, scrolling lists, key caps.
    pub inset: Hsla,
    pub foreground: Hsla,
    pub secondary: Hsla,
    pub bright: Hsla,
    pub accent: Hsla,
    pub on_accent: Hsla,
    pub selection: Hsla,
    pub border: Hsla,
    pub danger: Hsla,
    pub warning: Hsla,
    pub success: Hsla,
    pub font: SharedString,
    pub mono: SharedString,
}
impl Global for Theme {}

impl Theme {
    /// The system light appearance: a grey window around white content.
    pub fn light() -> Self {
        Self {
            name: "macOS Light".into(),
            appearance: ThemeAppearance::Light,
            background: rgb(0xec_ec_ec).into(),
            surface: rgb(0xff_ff_ff).into(),
            inset: rgb(0xf6_f6_f6).into(),
            // Label colours are translucent black, as AppKit's are, so text
            // keeps its weight over the tinted surfaces it lands on.
            foreground: rgba(0x00_00_00_d9).into(),
            secondary: rgb(0x6e_6e_73).into(),
            bright: rgb(0x00_00_00).into(),
            accent: rgb(0x00_7a_ff).into(),
            on_accent: rgb(0xff_ff_ff).into(),
            selection: rgba(0x00_7a_ff_2e).into(),
            border: rgb(0xd1_d1_d6).into(),
            danger: rgb(0xff_3b_30).into(),
            warning: rgb(0xff_95_00).into(),
            success: rgb(0x34_c7_59).into(),
            font: SYSTEM_FONT.into(),
            mono: MONO_FONT.into(),
        }
    }

    /// The system dark appearance: a near-black window, content one step up.
    pub fn dark() -> Self {
        Self {
            name: "macOS Dark".into(),
            appearance: ThemeAppearance::Dark,
            background: rgb(0x1e_1e_1e).into(),
            surface: rgb(0x2a_2a_2a).into(),
            inset: rgb(0x23_23_23).into(),
            foreground: rgba(0xff_ff_ff_d9).into(),
            secondary: rgb(0x98_98_9d).into(),
            bright: rgb(0xff_ff_ff).into(),
            accent: rgb(0x0a_84_ff).into(),
            on_accent: rgb(0xff_ff_ff).into(),
            selection: rgba(0x0a_84_ff_47).into(),
            border: rgb(0x3a_3a_3c).into(),
            danger: rgb(0xff_45_3a).into(),
            warning: rgb(0xff_9f_0a).into(),
            success: rgb(0x30_d1_58).into(),
            font: SYSTEM_FONT.into(),
            mono: MONO_FONT.into(),
        }
    }

    /// The palette `choice` draws in `appearance`.
    // Placeholder until the theme table lands: every theme is still the
    // system palette.
    pub fn resolve(choice: ThemeChoice, appearance: ThemeAppearance) -> Self {
        let _ = choice;
        match appearance {
            ThemeAppearance::Light => Self::light(),
            ThemeAppearance::Dark => Self::dark(),
        }
    }

    /// No paint at all, in the label colour so a hover that later fills the
    /// same edge or face animates from the right hue.
    pub fn transparent(&self) -> Hsla {
        self.foreground.opacity(0.)
    }

    /// Shared control colours, all tints of the label colour so they read
    /// the same way over either window background.
    pub fn normal_fill(&self) -> Hsla {
        self.foreground.opacity(0.04)
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
    pub fn divider(&self) -> Hsla {
        self.foreground.opacity(0.12)
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
