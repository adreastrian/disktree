//! The Settings window: appearance, theme and dark flavour.
//!
//! A second, small, fixed-size window, laid out the way System Settings
//! lays out a pane: a label column on the left, the control on the right.
//! There is only ever one: Disktree ▸ Settings… (`⌘ ,`) opens it, or brings
//! the open one forward. Escape and `⌘ W` close it; closing it leaves the
//! treemap where it was.
//!
//! Every control applies its choice at once through [`settings::choose`],
//! so the treemap behind the window, the window itself and the View menu
//! all change together, and the choice is saved for next time.

use disktree_core::classify::Category;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, Bounds, Context, Div, FocusHandle,
    FontWeight, InteractiveElement as _, IntoElement, ParentElement, Render,
    Styled, Window, WindowHandle, WindowOptions, div, px, relative,
};

use crate::controls::{ChoiceItem, button_group, card_group};
use crate::menu;
use crate::palette;
use crate::settings;
use crate::theme::{
    self, ActiveTheme as _, AppearanceChoice, Flavour, Theme, ThemeChoice,
    ThemeId,
};
use crate::ui::{BASE_REM, radius, size, space, text};

/// The key context the window's own bindings (Escape, Tab) live in.
pub const CONTEXT: &str = "DisktreeSettings";

// Tab walks the controls. gpui keeps the tab order but presses nothing on
// its own, and the treemap window spends Tab on the next sibling, so the
// walk is bound here, for this window only.
gpui_kit::actions!(
    disktree,
    [
        #[derive(Eq)]
        NextControl,
        #[derive(Eq)]
        PreviousControl,
    ]
);

/// The root view of the Settings window.
pub struct SettingsWindow {
    /// The window's own focus, taken when nothing else in it has it, so
    /// Escape and `⌘ W` always have somewhere to land.
    pub focus: FocusHandle,
}

impl std::fmt::Debug for SettingsWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SettingsWindow").finish_non_exhaustive()
    }
}

impl SettingsWindow {
    pub fn new(cx: &mut Context<'_, Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
        }
    }
}

/// The Settings window that is open, if one is, so a second `⌘ ,` brings
/// it forward instead of opening another.
#[derive(Debug, Default)]
struct OpenWindow(Option<WindowHandle<SettingsWindow>>);
impl gpui_kit::Global for OpenWindow {}

/// Open Settings, or bring the open Settings window to the front.
pub fn open(cx: &mut App) -> WindowHandle<SettingsWindow> {
    // A closed window's handle stays in the global; the window list says
    // whether it is still one of ours.
    if let Some(handle) = cx.try_global::<OpenWindow>().and_then(|w| w.0)
        && cx.windows().contains(&handle)
    {
        let _ = handle.update(cx, |this, window, cx| {
            window.focus(&this.focus, cx);
            window.activate_window();
        });
        return handle;
    }
    let bounds = Bounds::centered(
        None,
        gpui_kit::size(
            px(size::SETTINGS_WINDOW.0 * BASE_REM),
            px(size::SETTINGS_WINDOW_HEIGHT.0 * BASE_REM),
        ),
        cx,
    );
    let handle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(gpui_kit::WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui_kit::TitlebarOptions {
                    title: Some("Settings".into()),
                    ..Default::default()
                }),
                // A settings pane has one size, as the system's has: the
                // content is three rows and never scrolls.
                is_resizable: false,
                is_minimizable: false,
                ..Default::default()
            },
            |_, cx| cx.new(SettingsWindow::new),
        )
        .expect("open the settings window");
    let _ = handle.update(cx, |this, window, cx| {
        window.focus(&this.focus, cx);
        window.activate_window();
    });
    cx.set_global(OpenWindow(Some(handle)));
    handle
}

impl Render for SettingsWindow {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let choice = theme::choice(cx);

        let appearance = button_group(
            "settings-appearance",
            AppearanceChoice::ALL
                .into_iter()
                .map(|choice| ChoiceItem::new(choice.key(), choice.name()))
                .collect(),
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
        .w(size::RANKING_CHOICE);

        // Each card is drawn in its own theme, in the appearance the window
        // is showing, so the cards are a preview and not a legend.
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
                thumbnail(&preview).into_any_element()
            })
            .collect::<Vec<AnyElement>>();
        let themes = card_group(
            "settings-theme",
            ThemeId::ALL
                .into_iter()
                .map(|id| ChoiceItem::new(id.key(), id.name()))
                .collect(),
            cards,
            ThemeId::ALL.iter().position(|&it| it == choice.theme),
            move |index, _, cx| {
                let theme = ThemeId::ALL[index];
                settings::choose(ThemeChoice { theme, ..choice }, cx);
            },
            window,
            cx,
        );

        let flavour = choice.theme.has_flavours().then(|| {
            button_group(
                "settings-flavour",
                Flavour::ALL
                    .into_iter()
                    .map(|flavour| {
                        ChoiceItem::new(flavour.key(), flavour.name())
                    })
                    .collect(),
                Flavour::ALL.iter().position(|&it| it == choice.flavour),
                move |index, _, cx| {
                    let flavour = Flavour::ALL[index];
                    settings::choose(ThemeChoice { flavour, ..choice }, cx);
                },
                window,
                cx,
            )
            .w(size::RANKING_CHOICE)
        });

        div()
            .id("disktree-settings")
            .debug_selector(|| "disktree-settings".into())
            .track_focus(&self.focus)
            .key_context(CONTEXT)
            .on_action(|_: &menu::CloseWindow, window, _| {
                window.remove_window();
            })
            .on_action(|_: &NextControl, window, cx| window.focus_next(cx))
            .on_action(|_: &PreviousControl, window, cx| {
                window.focus_prev(cx);
            })
            .flex()
            .flex_col()
            .size_full()
            .p(space::LG + space::XXS)
            .gap(space::XL)
            .bg(theme.background)
            .text_color(theme.foreground)
            .font_family(theme.font.clone())
            .text_size(text::BODY)
            .child(row("Appearance", appearance, &theme))
            .child(row("Theme", themes, &theme))
            .when_some(flavour, |root, flavour| {
                root.child(row("Dark flavour", flavour, &theme))
            })
    }
}

/// One settings row: the label in the left column, the control beside it.
fn row(label: &'static str, control: impl IntoElement, theme: &Theme) -> Div {
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(space::LG)
        .child(
            div()
                .w(size::SETTINGS_LABEL)
                .flex_shrink_0()
                // The control's first line, not the row's top, is what the
                // label should sit on.
                .pt(space::XS + space::XXS)
                .text_right()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.secondary)
                .child(label),
        )
        .child(div().flex_1().min_w_0().child(control))
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
