//! The Settings sheet: appearance, theme and dark flavour.
//!
//! A panel over the treemap in the main window, the way `⌘ ,` opens
//! Settings in the apps that have no second window to keep. Disktree ▸
//! Settings… (`⌘ ,`) opens it and closes it again; so do Escape, the
//! close button and a press on the dimmed backdrop. The panel owns the
//! keyboard while it is open: Tab walks its controls and nothing reaches
//! the treemap underneath.
//!
//! Every control applies its choice at once through [`settings::choose`],
//! so the treemap behind the sheet, the sheet itself and the View menu all
//! change together, and the choice is saved for next time.

use disktree_core::classify::Category;
use gpui_kit::base::{RadioGroup, ThemeAppearance};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, Context, Div, FocusHandle, FontWeight, InteractiveElement as _,
    IntoElement, ParentElement, StatefulInteractiveElement as _, Styled,
    Window, div, relative,
};

use crate::controls::{
    ButtonVariant, ChoiceItem, IconName, button, card_group, choice_focus,
    icon, sheet_dialog,
};
use crate::palette;
use crate::settings;
use crate::state::Disktree;
use crate::theme::{
    self, ActiveTheme as _, AppearanceChoice, Flavour, Theme, ThemeChoice,
    ThemeId,
};
use crate::ui::{icon as icon_size, radius, size, space, text};
use crate::widgets::eyebrow;

/// The key context the sheet's own bindings (Tab, Shift-Tab) live in.
pub const CONTEXT: &str = "DisktreeSettings";

// Tab walks the controls. gpui keeps the tab order but presses nothing on
// its own, and the treemap spends Tab on the next sibling, so the walk is
// bound here, for the sheet only.
gpui_kit::actions!(
    disktree,
    [
        #[derive(Eq)]
        NextControl,
        #[derive(Eq)]
        PreviousControl,
    ]
);

/// The debug selector of the sheet's first control, where focus lands on
/// opening.
const FIRST_CONTROL: &str = "settings-appearance";

/// The sheet, drawn over the window while `app.settings_open`.
pub fn settings_overlay(
    app: &Disktree,
    window: &mut Window,
    cx: &mut Context<'_, Disktree>,
) -> impl IntoElement {
    let theme = cx.theme().clone();
    let choice = theme::choice(cx);

    // Opening puts focus on the sheet's host; the first control is the
    // place to be, and it exists only once the sheet is being drawn.
    if app.settings_focus.is_focused(window) {
        let first = choice_focus(FIRST_CONTROL, window, cx);
        window.focus(&first, cx);
    }

    let header = div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .flex_shrink_0()
        .px(space::LG + space::XXS)
        .pt(space::LG)
        .pb(space::MD)
        .child(
            div()
                .text_size(text::TITLE)
                .font_weight(FontWeight::BOLD)
                .text_color(theme.bright)
                .child("Settings"),
        )
        .child(
            button("settings-close", "", ButtonVariant::Secondary, cx)
                .accessibility_label("Close")
                .debug_selector(|| "settings-close".into())
                .p(space::XS + space::XXS)
                .text_color(theme.secondary)
                .child(icon(IconName::Close).size(icon_size::MD))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.close_settings(cx);
                    this.apply_focus(window, cx);
                })),
        );

    let body = div()
        .id("settings-body")
        .overflow_y_scroll()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .gap(space::XL)
        .px(space::LG + space::XXS)
        .pb(space::LG + space::XXS)
        .child(section("Appearance", appearance(choice, window, cx), cx))
        .child(section("Theme", themes(choice, window, cx), cx))
        .when(choice.theme.has_flavours(), |body| {
            body.child(
                section("Dark flavour", flavours(choice, window, cx), cx)
                    .child(
                        div()
                            .text_size(text::CAPTION)
                            .text_color(theme.secondary)
                            .child("Light mode uses Latte."),
                    ),
            )
        });

    let panel = div()
        .id("disktree-settings")
        .debug_selector(|| "disktree-settings".into())
        // A press on the panel is not a press on the backdrop behind it.
        .occlude()
        .flex()
        .flex_col()
        .w(size::SETTINGS_PANEL)
        .max_w(relative(1.))
        // Shorter than the window, so the body scrolls before the panel
        // runs off the screen at the minimum window size.
        .max_h(relative(0.92))
        .border_1()
        .border_color(theme.border)
        .rounded(radius::DIALOG)
        .shadow_lg()
        .bg(theme.surface)
        .text_color(theme.foreground)
        .font_family(theme.font)
        .text_size(text::BODY)
        .child(header)
        .child(body);

    let close = cx.entity().downgrade();
    let sheet = sheet_dialog(&app.settings_focus, cx)
        .open(true)
        // Return is spent by the control that has focus; the sheet itself
        // has nothing to confirm.
        .on_ok(|_, _, _| false)
        .on_cancel(move |_, window, cx| {
            let _ = close.update(cx, |this, cx| {
                this.close_settings(cx);
                this.apply_focus(window, cx);
            });
            false
        })
        .child(panel);

    let next = app.settings_focus.clone();
    let previous = app.settings_focus.clone();
    div()
        .key_context(CONTEXT)
        .on_action(move |_: &NextControl, window, cx| {
            step(&next, true, window, cx);
        })
        .on_action(move |_: &PreviousControl, window, cx| {
            step(&previous, false, window, cx);
        })
        .child(sheet)
}

/// Move focus to the next (or previous) control inside `within`.
///
/// The tab order is the window's: the sheet is drawn last, so the stop
/// after its last control is the treemap's first, and the one before its
/// first is the treemap's last. Stepping on until focus is back in the
/// sheet makes the walk cycle within it.
fn step(
    within: &FocusHandle,
    forward: bool,
    window: &mut Window,
    cx: &mut App,
) {
    // The window has a handful of stops outside the sheet; well within
    // this many steps the walk is back inside.
    const LIMIT: usize = 32;
    for _ in 0..LIMIT {
        if forward {
            window.focus_next(cx);
        } else {
            window.focus_prev(cx);
        }
        if within.contains_focused(window, cx) {
            return;
        }
    }
}

/// An eyebrow over a control.
fn section(label: &'static str, control: impl IntoElement, cx: &App) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(space::SM)
        .child(eyebrow(label, cx))
        .child(control)
}

/// System, Light and Dark, each a small picture of a window in that
/// appearance: System is half and half, as System Settings draws it.
fn appearance(
    choice: ThemeChoice,
    window: &mut Window,
    cx: &mut App,
) -> RadioGroup {
    let light = Theme::resolve(choice, ThemeAppearance::Light);
    let dark = Theme::resolve(choice, ThemeAppearance::Dark);
    let cards = AppearanceChoice::ALL
        .into_iter()
        .map(|it| {
            (
                ChoiceItem::new(it.key(), it.name()),
                appearance_swatch(it, &light, &dark).into_any_element(),
            )
        })
        .collect();
    card_group(
        "settings-appearance",
        cards,
        size::SWATCH_CARD,
        AppearanceChoice::ALL
            .iter()
            .position(|&it| it == choice.appearance),
        move |index, _, cx| {
            let appearance = AppearanceChoice::ALL[index];
            settings::choose(
                ThemeChoice {
                    appearance,
                    ..choice
                },
                cx,
            );
        },
        window,
        cx,
    )
}

/// A card for every theme, each drawn in its own colours in the appearance
/// the window is showing, so the cards are a preview and not a legend.
fn themes(
    choice: ThemeChoice,
    window: &mut Window,
    cx: &mut App,
) -> RadioGroup {
    let shown = choice.appearance.resolve(window.appearance());
    let cards = ThemeId::ALL
        .into_iter()
        .map(|id| {
            let preview = Theme::resolve(
                ThemeChoice {
                    theme: id,
                    ..choice
                },
                shown,
            );
            (
                ChoiceItem::new(id.key(), id.name()),
                thumbnail(&preview).into_any_element(),
            )
        })
        .collect();
    card_group(
        "settings-theme",
        cards,
        size::THEME_CARD,
        ThemeId::ALL.iter().position(|&it| it == choice.theme),
        move |index, _, cx| {
            let theme = ThemeId::ALL[index];
            settings::choose(ThemeChoice { theme, ..choice }, cx);
        },
        window,
        cx,
    )
}

/// The three dark flavours, each a swatch of its own dark palette: the
/// flavour is a dark-mode choice, so the swatch is dark whatever the
/// window shows.
fn flavours(
    choice: ThemeChoice,
    window: &mut Window,
    cx: &mut App,
) -> RadioGroup {
    let cards = Flavour::ALL
        .into_iter()
        .map(|flavour| {
            let palette = Theme::resolve(
                ThemeChoice { flavour, ..choice },
                ThemeAppearance::Dark,
            );
            (
                ChoiceItem::new(flavour.key(), flavour.name()),
                flavour_swatch(&palette).into_any_element(),
            )
        })
        .collect();
    card_group(
        "settings-flavour",
        cards,
        size::SWATCH_CARD,
        Flavour::ALL.iter().position(|&it| it == choice.flavour),
        move |index, _, cx| {
            let flavour = Flavour::ALL[index];
            settings::choose(ThemeChoice { flavour, ..choice }, cx);
        },
        window,
        cx,
    )
}

/// The categories a swatch shows, in the order they tend to appear on a
/// home directory: enough to tell the palettes apart, not the whole legend.
const SWATCH_CATEGORIES: [Category; 6] = [
    Category::Code,
    Category::Media,
    Category::Documents,
    Category::Toolchain,
    Category::Cache,
    Category::Git,
];

/// The frame every swatch sits in: a small rounded window.
fn swatch_frame(theme: &Theme) -> Div {
    div()
        .flex()
        .w(size::SWATCH_THUMB)
        .h(size::SWATCH_THUMB * 0.64)
        .rounded(radius::CONTROL)
        .overflow_hidden()
        .border_1()
        .border_color(theme.border)
}

/// A flavour: its dark background with a row of its category accents.
fn flavour_swatch(palette: &Theme) -> Div {
    swatch_frame(palette)
        .items_center()
        .justify_center()
        .gap(space::XS)
        .bg(palette.background)
        .children(SWATCH_CATEGORIES.into_iter().map(|category| {
            div()
                .size(space::SM)
                .rounded_full()
                .bg(palette::category_accent(palette, category))
        }))
}

/// An appearance: a window in that palette, or one of each side by side
/// for System.
fn appearance_swatch(
    appearance: AppearanceChoice,
    light: &Theme,
    dark: &Theme,
) -> Div {
    let frame = swatch_frame(light).flex_row();
    match appearance {
        AppearanceChoice::System => {
            frame.child(window_half(light)).child(window_half(dark))
        }
        AppearanceChoice::Light => frame.child(window_half(light)),
        AppearanceChoice::Dark => frame.child(window_half(dark)),
    }
}

/// Half (or all) of a window in `theme`: the title band with the accent,
/// and a well with two tiles in it.
fn window_half(theme: &Theme) -> Div {
    div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .bg(theme.background)
        .child(
            div()
                .w_full()
                .h(relative(0.28))
                .flex_shrink_0()
                .flex()
                .items_center()
                .px(space::XS)
                .child(
                    div()
                        .w(space::SM)
                        .h(space::XS)
                        .rounded(radius::KEYCAP)
                        .bg(theme.accent),
                ),
        )
        .child(
            div()
                .flex_1()
                .w_full()
                .flex()
                .flex_row()
                .gap(space::XXS)
                .p(space::XXS)
                .bg(theme.inset)
                .child(
                    div()
                        .flex_1()
                        .h_full()
                        .rounded(radius::KEYCAP)
                        .bg(palette::category_fill(theme, Category::Code, 0)),
                )
                .child(
                    div()
                        .flex_1()
                        .h_full()
                        .rounded(radius::KEYCAP)
                        .bg(palette::category_fill(theme, Category::Media, 0)),
                ),
        )
}

/// A mini treemap in `theme`'s colours: a title band, a few top-level
/// directories with their strips, two nested tiles, and one tile in the
/// highlight, so every colour the mosaic uses is on the card.
///
/// The geometry is fractions of the card, laid by hand once: it is a
/// picture of a treemap, not a layout of one.
fn thumbnail(theme: &Theme) -> Div {
    // (category, x, y, width, height, depth). Parents before children, so
    // a nested tile paints over the one it is in.
    const TILES: [(Category, f32, f32, f32, f32, u32); 9] = [
        (Category::Code, 0.0, 0.0, 0.45, 0.56, 0),
        (Category::Code, 0.03, 0.16, 0.19, 0.36, 1),
        (Category::Git, 0.24, 0.16, 0.18, 0.36, 1),
        (Category::Media, 0.45, 0.0, 0.32, 0.56, 0),
        (Category::Cache, 0.77, 0.0, 0.23, 0.56, 0),
        (Category::Toolchain, 0.0, 0.56, 0.3, 0.44, 0),
        (Category::Documents, 0.3, 0.56, 0.28, 0.44, 0),
        (Category::Synced, 0.58, 0.56, 0.22, 0.44, 0),
        (Category::Other, 0.8, 0.56, 0.2, 0.44, 0),
    ];
    /// The tile the selection sits on: the highlight ring, and the label.
    const SELECTED: usize = 3;
    /// The strip over a top-level directory, as a share of its height.
    const STRIP: f32 = 0.14;

    let tiles = TILES.into_iter().enumerate().map(
        |(index, (category, x, y, w, h, depth))| {
            let mut tile = div()
                .absolute()
                .left(relative(x))
                .top(relative(y))
                .w(relative(w))
                .h(relative(h))
                .border_1()
                .border_color(theme.inset)
                .bg(palette::category_fill(theme, category, depth));
            if depth == 0 {
                tile = tile.child(
                    div()
                        .w_full()
                        .h(relative(STRIP))
                        .bg(palette::category_accent(theme, category)),
                );
            }
            if index == SELECTED {
                tile = tile.border_2().border_color(palette::highlight(theme));
            }
            tile
        },
    );
    div()
        .flex()
        .flex_col()
        .w(size::THEME_THUMB)
        .h(size::THEME_THUMB * 0.7)
        .rounded(radius::CONTROL)
        .overflow_hidden()
        .border_1()
        .border_color(theme.border)
        .bg(theme.background)
        // The title band, with the accent as its one control.
        .child(
            div()
                .w_full()
                .h(relative(0.16))
                .flex_shrink_0()
                .flex()
                .items_center()
                .px(space::XS)
                .child(
                    div()
                        .w(space::SM)
                        .h(space::XS)
                        .rounded(radius::KEYCAP)
                        .bg(theme.accent),
                ),
        )
        .child(
            div()
                .relative()
                .flex_1()
                .w_full()
                .bg(theme.inset)
                .children(tiles),
        )
}
