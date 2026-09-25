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

    /// The palette for a window appearance. The vibrant variants are the
    /// same two appearances seen through a translucent material.
    pub fn for_appearance(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Light | WindowAppearance::VibrantLight => {
                Self::light()
            }
            WindowAppearance::Dark | WindowAppearance::VibrantDark => {
                Self::dark()
            }
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
    Theme::for_appearance(cx.window_appearance()).apply(cx);
}
