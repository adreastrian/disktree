//! The scale this app draws with: spacing, type, icons and region widths.
//!
//! Everything is in `rem`, following the gpui-kit design guide: type, spacing,
//! controls and icons share one zoom axis, so interface zoom (`⌘ =`,
//! `⌘ -`, `⌘ 0`) keeps every relationship intact instead of only
//! enlarging text. Choose a step by what two things mean to each other, not by
//! the pixels it happens to resolve to today.
//!
//! Radius follows macOS: controls round a little, sheets and dialogs more,
//! so a nested surface always sits inside a softer one and the corners stay
//! concentric.
//!
//! Pixels remain only where a value is physical: hairline borders, the
//! treemap's own geometry (laid out in viewport pixels, then scaled by the
//! view), and positions that come from the pointer.

use gpui_kit::{Pixels, Point, Rems, point, px};

/// The guide's semantic spacing scale: 2, 4, 8, 12, 16, 24 and 32 px at the
/// default 16 px rem.
pub mod space {
    use super::Rems;

    /// Optical correction: an icon baseline, a compact separator.
    pub const XXS: Rems = Rems(0.125);
    /// Parts of one control: icon and label, title and its description.
    pub const XS: Rems = Rems(0.25);
    /// Closely related controls: a button group, dialog actions.
    pub const SM: Rems = Rems(0.5);
    /// One content group: columns of a row, compact form rows.
    pub const MD: Rems = Rems(0.75);
    /// Separate groups in one section, and region padding.
    pub const LG: Rems = Rems(1.0);
    /// Separate sections.
    pub const XL: Rems = Rems(1.5);
    /// A major region boundary: empty-state breathing room.
    pub const XXL: Rems = Rems(2.0);
}

/// Type steps. Compact, as a utility's should be: body is one step below
/// the system's 13 px so the panel fits beside the treemap without scrolling.
pub mod text {
    use super::Rems;

    /// Metadata, key caps and tooltips.
    pub const CAPTION: Rems = Rems(0.6875);
    /// Body text and control labels.
    pub const BODY: Rems = Rems(0.75);
    /// Window, section and dialog titles.
    pub const TITLE: Rems = Rems(0.875);
    /// The app name and the selection's name.
    pub const HEADING: Rems = Rems(1.125);
    /// A figure worth reading from across the room: the free space.
    pub const FIGURE: Rems = Rems(1.625);
    /// The selection's size: the one number the panel exists to show.
    pub const DISPLAY: Rems = Rems(2.5);
}

/// Corner radii, one per surface tier.
pub mod radius {
    use super::Rems;

    /// A key cap: small enough to still read as a key.
    pub const KEYCAP: Rems = Rems(0.25);
    /// Buttons, choice groups, check boxes, chips.
    pub const CONTROL: Rems = Rems(0.375);
    /// Menus, tooltips, list wells.
    pub const SURFACE: Rems = Rems(0.5);
    /// Dialogs and the help overlay.
    pub const DIALOG: Rems = Rems(0.625);
}

/// Icon slots, sized with the text they sit beside.
pub mod icon {
    use super::Rems;

    pub const SM: Rems = Rems(0.75);
    pub const MD: Rems = Rems(0.875);
    pub const LG: Rems = Rems(1.375);
}

/// Region and lane widths. Each is the comfortable default for its content at
/// the default rem; they scale with zoom like everything else.
pub mod size {
    use super::Rems;

    /// The title bar, which the top row is drawn in. Taller than the
    /// system's 28 px band so the segmented control and the check boxes
    /// keep their air; the traffic lights are centred on it.
    pub const TITLE_BAR: Rems = Rems(2.375);
    /// A crumb's sibling menu.
    pub const SIBLING_MENU: Rems = Rems(24.0);
    pub const SIBLING_MENU_HEIGHT: Rems = Rems(32.0);
    /// A list row's share bar.
    pub const ROW_BAR: Rems = Rems(5.5);
    /// A legend or identity swatch.
    pub const SWATCH: Rems = Rems(0.625);
    /// A thin meter: share of the scan, the disk.
    pub const METER: Rems = Rems(0.3125);
    /// The meter on the scanning panel.
    pub const SCANNING_METER: Rems = Rems(26.25);
    /// The Size | Files | Age choice in the settings row.
    pub const RANKING_CHOICE: Rems = Rems(11.0);
    /// The review screen's summary column.
    pub const REVIEW_SUMMARY: Rems = Rems(22.5);
    /// The review list's share-bar lane.
    pub const SHARE_LANE: Rems = Rems(6.0);
    /// The review list's size lane: right-aligned, so sizes compare.
    pub const SIZE_LANE: Rems = Rems(5.0);
    /// The cursor tooltip.
    pub const TOOLTIP: Rems = Rems(16.75);
    /// A confirmation dialog.
    pub const DIALOG: Rems = Rems(26.25);
    /// The keyboard overlay.
    pub const HELP: Rems = Rems(32.5);
    /// The key column of the keyboard overlay.
    pub const KEY_LANE: Rems = Rems(7.0);
}

/// What macOS draws over the title bar. In pixels, since the traffic
/// lights are the system's and do not follow interface zoom.
pub mod chrome {
    use super::{Pixels, Point, point, px, size};

    /// The close button's left edge, where Zed and gpui's own title bar
    /// put it.
    const LIGHTS_X: Pixels = px(9.);
    /// A standard window button is 12 px tall.
    const LIGHT_HEIGHT: Pixels = px(12.);
    /// Where the bar's content starts: the three lights end at 61 px
    /// (three 12 px buttons, two 8 px gaps, the lead-in), and the rest is
    /// the gap Finder leaves before its first control.
    pub const TRAFFIC_LIGHTS: Pixels = px(78.);

    /// The traffic lights' origin for a title bar of [`size::TITLE_BAR`]
    /// at `rem` pixels: centred on the bar, so they move when interface
    /// zoom changes the bar's height.
    pub fn traffic_light_position(rem: f32) -> Point<Pixels> {
        let bar = px(size::TITLE_BAR.0 * rem);
        point(LIGHTS_X, (bar - LIGHT_HEIGHT) * 0.5)
    }
}

/// Interface zoom steps, as a factor of the 16 px default rem.
pub const ZOOM_STEPS: [f32; 7] = [0.75, 0.875, 1.0, 1.125, 1.25, 1.5, 1.75];

/// The default rem, in pixels.
pub const BASE_REM: f32 = 16.0;
