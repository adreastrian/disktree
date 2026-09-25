//! The controls the app is built from: buttons, a choice group, a check box,
//! dialogs, tooltips, icons and key caps, each a thin coat of the app's
//! palette over gpui-base's behaviour.
//!
//! gpui-base owns focus, activation, roles and dialog lifecycle; this file
//! only decides how each control looks. Corners are rounded the way macOS
//! rounds its own controls, and every size is a rem token from [`crate::ui`].

use std::rc::Rc;
use std::time::Duration;

use gpui_kit::base::actions::{Confirm, SelectLeft, SelectRight};
use gpui_kit::base::{
    AlertDialog, ButtonStyles, Checkbox, CheckboxIndicator, CheckboxState,
    DialogBackdrop, DialogDescription, DialogPopup, DialogTitle, Radio,
    RadioGroup, StyledExt as _, Tooltip,
};
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyElement, App, ClickEvent, Context, Div, ElementId, Entity, FocusHandle,
    FontWeight, InteractiveElement, Interactivity, IntoElement, MouseButton,
    ParentElement, Render, RenderOnce, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, div, rems,
    rgb, svg,
};

pub use gpui_kit::assets::IconName;

use crate::theme::{ActiveTheme as _, OPTION_GROUP_CONTEXT, Theme};
use crate::ui::{radius, size, space, text};

/// How much a button asks for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Outline,
    #[default]
    Secondary,
    Danger,
}

/// A button whose hover, pressed and focus styles are withheld while it is
/// disabled, whichever order the builder calls arrive in.
#[derive(IntoElement)]
pub struct Button {
    base: gpui_kit::base::Button,
    disabled: bool,
    hover: Option<Box<StyleRefinement>>,
    active: Option<Box<StyleRefinement>>,
    focus_visible: Option<Box<StyleRefinement>>,
}

impl std::fmt::Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Button")
            .field("disabled", &self.disabled)
            .finish_non_exhaustive()
    }
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: gpui_kit::base::Button::new(id),
            disabled: false,
            hover: None,
            active: None,
            focus_visible: None,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self.base = self.base.disabled(disabled);
        self
    }

    pub fn styles(
        mut self,
        build: impl FnOnce(ButtonStyles) -> ButtonStyles,
    ) -> Self {
        self.base = self.base.styles(build);
        self
    }

    pub fn accessibility_label(
        mut self,
        label: impl Into<SharedString>,
    ) -> Self {
        self.base = self.base.accessibility_label(label);
        self
    }

    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.base = self.base.tab_stop(tab_stop);
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.base = self.base.on_click(handler);
        self
    }
}

impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl InteractiveElement for Button {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }

    fn hover(
        mut self,
        build: impl FnOnce(StyleRefinement) -> StyleRefinement,
    ) -> Self {
        self.hover = Some(Box::new(build(StyleRefinement::default())));
        self
    }

    fn focus_visible(
        mut self,
        build: impl FnOnce(StyleRefinement) -> StyleRefinement,
    ) -> Self {
        self.focus_visible = Some(Box::new(build(StyleRefinement::default())));
        self
    }
}

impl StatefulInteractiveElement for Button {
    fn active(
        mut self,
        build: impl FnOnce(StyleRefinement) -> StyleRefinement,
    ) -> Self {
        self.active = Some(Box::new(build(StyleRefinement::default())));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let mut base = self.base;
        if !self.disabled {
            if let Some(style) = self.hover {
                base = base.hover(|_| *style);
            }
            if let Some(style) = self.active {
                base = base.active(|_| *style);
            }
            if let Some(style) = self.focus_visible {
                base = base.focus_visible(|_| *style);
            }
        }
        base
    }
}

/// A push button. Primary is filled with the accent, as the default button
/// of a macOS sheet is; the others are bordered or bare.
pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
    cx: &App,
) -> Button {
    let t = cx.theme();
    let (bg, fg, border) = match variant {
        ButtonVariant::Primary => (t.accent, t.on_accent, t.accent),
        ButtonVariant::Outline => (t.normal_fill(), t.foreground, t.border),
        ButtonVariant::Secondary => {
            (t.transparent(), t.foreground, t.transparent())
        }
        ButtonVariant::Danger => (t.normal_fill(), t.danger, t.border),
    };
    let interaction_border = if variant == ButtonVariant::Primary {
        t.accent
    } else {
        t.focus_border()
    };
    // A filled button darkens under the pointer; the others take a wash.
    let (hover_bg, pressed_bg) = if variant == ButtonVariant::Primary {
        (t.accent.opacity(0.88), t.accent.opacity(0.76))
    } else {
        (t.hover_fill(), t.pressed_fill())
    };
    let label = label.into();
    Button::new(id)
        .accessibility_label(label.clone())
        .when(!label.is_empty(), |button| button.child(label))
        .py(space::XS + space::XXS)
        .px(space::SM + space::XXS)
        .gap(space::SM)
        .border_1()
        .rounded(radius::CONTROL)
        .border_color(border)
        .font_family(t.font.clone())
        .text_size(text::BODY)
        .font_weight(FontWeight::NORMAL)
        .bg(bg)
        .text_color(fg)
        .hover(move |s| s.bg(hover_bg).border_color(interaction_border))
        .focus_visible(move |s| s.bg(hover_bg).border_color(interaction_border))
        .active(move |s| s.bg(pressed_bg))
        .styles(|s| {
            s.selected(|s| s.bg(t.selected_fill()))
                .disabled(|s| s.opacity(0.45))
        })
}

/// A labelled check box with a square indicator that fills with the accent
/// when checked, as the system's does.
pub fn checkbox(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    state: CheckboxState,
    cx: &App,
) -> Checkbox {
    let t = cx.theme();
    let label = label.into();
    Checkbox::new(id)
        .state(state)
        .accessibility_label(label.clone())
        .flex()
        .items_center()
        .gap(space::SM)
        .min_h(rems(1.75))
        .px(space::XS)
        .border_1()
        .rounded(radius::CONTROL)
        .border_color(t.transparent())
        .text_color(t.foreground)
        .font_family(t.font.clone())
        .text_size(text::BODY)
        .child(
            CheckboxIndicator::new()
                .state(state)
                .size(rems(1.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(radius::KEYCAP)
                .border_1()
                .border_color(t.control_border())
                .bg(t.surface)
                .text_color(t.foreground)
                .styles(|s| {
                    s.checked(|s| s.bg(t.accent).border_color(t.accent))
                        .indeterminate(|s| {
                            s.bg(t.accent).border_color(t.accent)
                        })
                })
                .when(state != CheckboxState::Unchecked, |indicator| {
                    indicator.child(
                        icon(if state == CheckboxState::Checked {
                            IconName::Check
                        } else {
                            IconName::Minus
                        })
                        .size(rems(0.875))
                        .text_color(t.on_accent),
                    )
                }),
        )
        .child(label)
        .hover(|s| s.bg(t.hover_fill()).border_color(t.focus_border()))
        .focus_visible(|s| s.bg(t.hover_fill()).border_color(t.focus_border()))
        .active(|s| s.bg(t.pressed_fill()))
        .styles(|s| s.disabled(|s| s.opacity(0.45)))
}

/// One option of a [`button_group`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChoiceItem {
    pub value: SharedString,
    pub label: SharedString,
    pub disabled: bool,
}

impl ChoiceItem {
    pub fn new(
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }

    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

type Change = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// Where the keyboard is inside a choice group, kept across frames by the
/// group's element id.
struct Cursor {
    focus: FocusHandle,
    index: Option<usize>,
    was_focused: bool,
}

/// A single-choice setting as a segmented control: one keyboard stop for
/// the whole group, Left/Right (or h/l) to move the cursor, Return/Space to
/// commit. The caller owns `selected` and changes it in `on_change`.
pub fn button_group(
    id: impl Into<ElementId>,
    items: Vec<ChoiceItem>,
    selected: Option<usize>,
    on_change: impl Fn(usize, &mut Window, &mut App) + 'static,
    window: &mut Window,
    cx: &mut App,
) -> RadioGroup {
    let id = id.into();
    let cursor = cursor(&id, &items, selected, window, cx);
    let on_change: Change = Rc::new(move |index, window, cx| {
        if selected != Some(index) {
            on_change(index, window, cx);
        }
    });
    let t = cx.theme().clone();
    let count = items.len();
    let rows = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let change = on_change.clone();
            let focus = cursor.read(cx).focus.clone();
            let chosen = selected == Some(index);
            Radio::new(index)
                .checked(chosen)
                .disabled(item.disabled)
                .accessibility_label(item.label.clone())
                .set_position(index + 1, count)
                .tab_stop(false)
                .flex()
                .items_center()
                .flex_1()
                .justify_center()
                .px(space::MD)
                .py(space::XS)
                .border_1()
                .rounded(radius::KEYCAP + space::XXS)
                .border_color(t.transparent())
                .font_family(t.font.clone())
                .text_size(text::BODY)
                .text_color(t.foreground)
                // The chosen segment is filled with the accent, as a
                // selected segment in a macOS toolbar is. A raised pill in
                // the sheet colour was tried first and vanished on the dark
                // palette, where the sheet is barely lighter than the well.
                .when(chosen, |row| {
                    row.bg(t.accent)
                        .text_color(t.on_accent)
                        .font_weight(FontWeight::SEMIBOLD)
                        .shadow_sm()
                })
                .when(
                    focus.is_focused(window)
                        && cursor.read(cx).index == Some(index),
                    |row| row.border_color(t.accent),
                )
                .when(!item.disabled && !chosen, |row| {
                    row.hover(|row| row.bg(t.hover_fill()))
                        .active(|row| row.bg(t.pressed_fill()))
                })
                .styles(|styles| styles.disabled(|row| row.opacity(0.45)))
                .child(item.label.clone())
                .on_change(move |_, _, window, cx| {
                    focus.focus(window, cx);
                    change(index, window, cx);
                })
                .into_any_element()
        })
        .collect::<Vec<_>>();
    navigate(
        RadioGroup::new(id)
            .axis(gpui_kit::Axis::Horizontal)
            .flex()
            .w_full()
            .gap(space::XXS)
            .p(space::XXS)
            .rounded(radius::CONTROL)
            .bg(t.selected_fill().opacity(0.5))
            .children(rows),
        cursor,
        items,
        on_change,
        cx,
    )
}

fn cursor(
    id: &ElementId,
    items: &[ChoiceItem],
    selected: Option<usize>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<Cursor> {
    let enabled = |i: usize| items.get(i).is_some_and(|item| !item.disabled);
    let initial = selected
        .filter(|&i| enabled(i))
        .or_else(|| items.iter().position(|item| !item.disabled));
    let cursor =
        window.use_keyed_state((id.clone(), "cursor"), cx, |_, cx| Cursor {
            focus: cx.focus_handle(),
            index: initial,
            was_focused: false,
        });
    cursor.update(cx, |state, _| {
        let focused = state.focus.is_focused(window);
        // Arriving by Tab lands the cursor on the current choice; a stale
        // cursor (the option it pointed at got disabled) resets too.
        if (focused && !state.was_focused) || !state.index.is_some_and(enabled)
        {
            state.index = initial;
        }
        state.was_focused = focused;
    });
    cursor
}

fn navigate<T: StatefulInteractiveElement + FluentBuilder>(
    root: T,
    cursor: Entity<Cursor>,
    items: Vec<ChoiceItem>,
    change: Change,
    cx: &App,
) -> T {
    let focus = cursor.read(cx).focus.clone();
    let enabled: Vec<_> = items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| (!item.disabled).then_some(i))
        .collect();
    let mut root = root
        .key_context(OPTION_GROUP_CONTEXT)
        .when(!enabled.is_empty(), |root| {
            root.track_focus(&focus.clone().tab_stop(true))
        });
    if !enabled.is_empty() {
        let pointer_focus = focus;
        root = root.on_mouse_down(MouseButton::Left, move |_, window, cx| {
            pointer_focus.focus(window, cx);
        });
    }
    let left = cursor.clone();
    let left_items = enabled.clone();
    root = root.on_action(move |_: &SelectLeft, window, cx| {
        left.update(cx, |state, cx| step(state, &left_items, false, cx));
        window.refresh();
    });
    let right = cursor.clone();
    let right_items = enabled;
    root = root.on_action(move |_: &SelectRight, window, cx| {
        right.update(cx, |state, cx| step(state, &right_items, true, cx));
        window.refresh();
    });
    root.on_action(move |_: &Confirm, window, cx| {
        if let Some(index) = cursor
            .read(cx)
            .index
            .filter(|&i| items.get(i).is_some_and(|item| !item.disabled))
        {
            change(index, window, cx);
        }
    })
}

fn step(
    cursor: &mut Cursor,
    enabled: &[usize],
    forward: bool,
    cx: &mut Context<'_, Cursor>,
) {
    if enabled.is_empty() {
        return;
    }
    let current = cursor
        .index
        .and_then(|i| enabled.iter().position(|&v| v == i))
        .unwrap_or(0);
    cursor.index = Some(
        enabled[if forward {
            (current + 1).min(enabled.len() - 1)
        } else {
            current.saturating_sub(1)
        }],
    );
    cx.notify();
}

/// A modal decision. Base keeps its alert role: a press on the backdrop
/// never dismisses it.
pub fn alert_dialog(focus: &FocusHandle, cx: &mut App) -> AlertDialog {
    AlertDialog::new(cx)
        .focus_handle(focus.clone())
        .backdrop(dialog_backdrop())
}

fn dialog_backdrop() -> DialogBackdrop {
    DialogBackdrop::new()
        .absolute()
        .size_full()
        .bg(gpui_kit::Hsla::from(rgb(0)).opacity(0.4))
}

/// The sheet a dialog's content sits on.
pub fn dialog_popup(cx: &App) -> DialogPopup {
    let t = cx.theme();
    DialogPopup::new()
        .relative()
        .flex()
        .flex_col()
        .gap(space::MD + space::XXS)
        .w(size::DIALOG)
        .max_w(gpui_kit::relative(1.))
        .p(space::LG + space::XXS)
        .border_1()
        .border_color(t.border)
        .rounded(radius::DIALOG)
        .shadow_lg()
        .bg(t.surface)
        .text_color(t.foreground)
        .font_family(t.font.clone())
        .text_size(text::BODY)
}

pub fn dialog_title(title: impl Into<SharedString>, cx: &App) -> DialogTitle {
    DialogTitle::new()
        .text_size(text::TITLE)
        .font_weight(FontWeight::BOLD)
        .text_color(cx.theme().foreground)
        .child(title.into())
}

pub fn dialog_description(
    text: impl Into<SharedString>,
    cx: &App,
) -> DialogDescription {
    DialogDescription::new()
        .text_color(cx.theme().secondary)
        .child(text.into())
}

/// A dialog footer action. Danger keeps a plain face and a red label, so
/// the destructive choice is never the one the eye lands on first.
pub fn dialog_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
    cx: &App,
) -> Button {
    let t = cx.theme();
    let button = button(id, label, variant, cx);
    match variant {
        ButtonVariant::Primary => button,
        ButtonVariant::Outline | ButtonVariant::Secondary => button
            .bg(t.surface)
            .border_color(t.border)
            .text_color(t.foreground),
        ButtonVariant::Danger => button
            .bg(t.surface)
            .border_color(t.border)
            .text_color(t.danger),
    }
}

/// A tooltip surface: a small rounded card in the sheet colour.
pub fn tooltip(text: impl Into<SharedString>, cx: &App) -> Tooltip {
    let t = cx.theme();
    Tooltip::new("disktree-tooltip")
        .max_w(rems(20.))
        .px(space::SM + space::XXS)
        .py(space::XS + space::XXS)
        .border_1()
        .rounded(radius::CONTROL)
        .border_color(t.border)
        .shadow_md()
        .bg(t.surface)
        .text_color(t.foreground)
        .font_family(t.font.clone())
        .text_size(text::CAPTION)
        .child(text.into())
}

/// Attach a tooltip without changing the control's type. GPUI owns the
/// hover delay, placement and dismissal.
pub fn with_tooltip<T: StatefulInteractiveElement>(
    control: T,
    text: impl Into<SharedString>,
) -> T {
    let text = text.into();
    control
        .tooltip_show_delay(Duration::from_millis(400))
        .tooltip(move |_, cx| cx.new(|_| TooltipText(text.clone())).into())
}

struct TooltipText(SharedString);

impl Render for TooltipText {
    fn render(
        &mut self,
        _: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl IntoElement {
        div()
            .debug_selector(|| "disktree-tooltip-surface".into())
            .child(tooltip(self.0.clone(), cx))
    }
}

/// A horizontal rule.
pub fn separator(cx: &App) -> Div {
    div()
        .w_full()
        .h(rems(0.0625))
        .flex_shrink_0()
        .bg(cx.theme().divider())
}

/// A key the reader can press, drawn as a small cap.
pub fn keycap(key: impl Into<SharedString>, cx: &App) -> Div {
    let t = cx.theme();
    div()
        // A glyph like ⌘ is as wide as the cap is tall, so the cap needs
        // more room at the sides than a caption line usually gets, or the
        // symbol sits on the border; and a floor on the width keeps a
        // single letter the same shape as a chord.
        .min_w(rems(1.5))
        .px(space::SM)
        .py(space::XXS + space::XXS / 2.)
        .flex()
        .justify_center()
        .flex_shrink_0()
        .border_1()
        .rounded(radius::KEYCAP)
        .border_color(t.border)
        .bg(t.inset)
        .font_family(t.font.clone())
        .text_size(text::CAPTION)
        .font_weight(FontWeight::BOLD)
        .text_color(t.foreground)
        .child(key.into())
}

/// The tone of a notice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Neutral,
    Warning,
}

impl Status {
    /// The theme colour that carries this status.
    pub const fn color(self, theme: &Theme) -> gpui_kit::Hsla {
        match self {
            Self::Neutral => theme.secondary,
            Self::Warning => theme.warning,
        }
    }
}

/// An icon from gpui-kit's Lucide bundle, in the surrounding text colour
/// unless told otherwise.
#[derive(IntoElement)]
pub struct Icon {
    name: IconName,
    style: StyleRefinement,
}

impl std::fmt::Debug for Icon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Icon").finish_non_exhaustive()
    }
}

impl Styled for Icon {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Icon {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        // An svg only paints once its own colour is set, so the inherited
        // one is resolved here; the caller's refinements still win.
        svg()
            .path(self.name.path())
            .text_color(window.text_style().color)
            .refine_style(&self.style)
    }
}

pub fn icon(name: IconName) -> Icon {
    Icon {
        name,
        style: StyleRefinement::default(),
    }
    .size(rems(1.))
    .flex_shrink_0()
}
