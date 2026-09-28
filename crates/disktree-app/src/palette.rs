//! Colour that means something.
//!
//! A tile's hue says what kind of data it is ([`Category`]), and deeper
//! tiles step away from the canvas so nesting reads without borders.
//! Reclaimable space is a hatch, not a colour, so "what is it" and "can it
//! go" are read independently.
//!
//! One strong colour is kept apart: the highlight. It marks the selection,
//! the main action, reclaimable totals and the free space after a removal,
//! and nothing else, so the eye goes straight to it.
//!
//! Every colour here is read from the active theme's tables
//! ([`crate::themes`]), where it was generated and checked: label contrast
//! on every fill, every legend colour apart from every other and from the
//! highlight, every depth a visible step. Nothing is derived at run time.

use disktree_core::classify::Category;
use gpui_kit::{Hsla, Rgba, rgba};

use crate::theme::Theme;
use crate::themes::{Color, DEPTHS};

/// Every category, in the order the theme tables list them.
pub const CATEGORIES: [Category; 9] = [
    Category::Code,
    Category::AgentScratch,
    Category::Toolchain,
    Category::Synced,
    Category::Git,
    Category::Media,
    Category::Documents,
    Category::Cache,
    Category::Other,
];

/// Where `category` sits in [`CATEGORIES`] and the theme tables.
pub fn category_index(category: Category) -> usize {
    CATEGORIES
        .iter()
        .position(|&known| known == category)
        .unwrap_or(CATEGORIES.len() - 1)
}

fn hsla(color: Color) -> Hsla {
    Hsla::from(rgba(color))
}

/// Depths beyond the table's last step draw as the last step.
fn depth_index(depth: u32) -> usize {
    (depth as usize).min(DEPTHS - 1)
}

/// The fill for a tile of `category`, `depth` levels into the view.
pub fn category_fill(theme: &Theme, category: Category, depth: u32) -> Hsla {
    hsla(
        theme.tiles.categories[category_index(category)].fills
            [depth_index(depth)],
    )
}

/// The saturated version of a category's hue: the strip over a top-level
/// directory and the legend swatch.
pub fn category_accent(theme: &Theme, category: Category) -> Hsla {
    hsla(theme.tiles.categories[category_index(category)].accent)
}

/// The ink for a name on a `category` fill at any depth: the theme's label
/// colour, unless the palette sets its own for fills that colour cannot
/// read on. Never dimmed: the generators cleared it at 4.5:1 as it is, and
/// an opacity would spend margin the palette does not have.
pub fn category_label(theme: &Theme, category: Category) -> Hsla {
    theme.tiles.categories[category_index(category)]
        .text
        .map_or(theme.label, hsla)
}

/// The age ramp, newest first: this week, this month, this half-year, this
/// year, older.
pub const AGE_BUCKETS: [(i64, &str); 5] = [
    (7, "This week"),
    (30, "This month"),
    (182, "Six months"),
    (365, "This year"),
    (i64::MAX, "Older"),
];

/// Which [`AGE_BUCKETS`] entry an age in days falls in.
pub fn age_bucket(days: i64) -> usize {
    AGE_BUCKETS
        .iter()
        .position(|(limit, _)| days <= *limit)
        .unwrap_or(AGE_BUCKETS.len() - 1)
}

/// The fill for age mode: recent writes carry colour, and it drains out of
/// a tile as it goes untouched.
pub fn age_fill(theme: &Theme, bucket: usize, depth: u32) -> Hsla {
    let bucket = bucket.min(AGE_BUCKETS.len() - 1);
    hsla(theme.tiles.age.fills[bucket][depth_index(depth)])
}

/// The age swatch for the legend.
pub fn age_accent(theme: &Theme, bucket: usize) -> Hsla {
    hsla(theme.tiles.age.swatches[bucket.min(AGE_BUCKETS.len() - 1)])
}

/// The one strong colour: selection, the main action, what can be had back.
pub const fn highlight(theme: &Theme) -> Hsla {
    theme.highlight
}

/// Text on a filled highlight.
pub const fn on_highlight(theme: &Theme) -> Hsla {
    theme.on_highlight
}

/// The diagonal hatch over reclaimable space: quiet enough to leave the hue
/// readable, visible on every fill.
pub const fn hatch(theme: &Theme) -> Hsla {
    theme.hatch
}

/// Linear interpolation between two colours, in RGB: interpolating hue
/// would drag a colour around the wheel on its way to a grey.
pub fn mix(from: Hsla, to: Hsla, t: f32) -> Hsla {
    let t = t.clamp(0.0, 1.0);
    let lerp = |a: f32, b: f32| (b - a).mul_add(t, a);
    let (a, b) = (from.to_rgb(), to.to_rgb());
    Hsla::from(Rgba {
        r: lerp(a.r, b.r),
        g: lerp(a.g, b.g),
        b: lerp(a.b, b.b),
        a: lerp(a.a, b.a),
    })
}

/// A tile's name on top of an age fill: the theme's label colour, at any
/// depth.
pub const fn label_color(theme: &Theme) -> Hsla {
    theme.label
}

#[cfg(test)]
mod tests {
    use gpui_kit::base::ThemeAppearance;

    use super::*;
    use crate::theme::{AppearanceChoice, Flavour, ThemeChoice, ThemeId};

    /// Every palette the tables hold: each theme in each appearance, the
    /// Catppuccin ones once per flavour. The other themes ignore the
    /// flavour, so each of their halves is visited three times; the checks
    /// are cheap and the loop stays simple.
    fn every_theme() -> impl Iterator<Item = Theme> {
        ThemeId::ALL.into_iter().flat_map(|theme| {
            Flavour::ALL.into_iter().flat_map(move |flavour| {
                [ThemeAppearance::Light, ThemeAppearance::Dark].map(
                    move |appearance| {
                        Theme::resolve(
                            ThemeChoice {
                                theme,
                                flavour,
                                appearance: AppearanceChoice::System,
                            },
                            appearance,
                        )
                    },
                )
            })
        })
    }

    /// Relative luminance, as WCAG defines it, of a colour laid over
    /// `under`: a translucent label is judged as it lands.
    fn luminance(color: Hsla, under: Hsla) -> f32 {
        let over = mix(under, Hsla { a: 1.0, ..color }, color.a).to_rgb();
        let channel = |c: f32| {
            if c <= 0.039_28 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.0722f32.mul_add(
            channel(over.b),
            0.2126f32.mul_add(channel(over.r), 0.7152 * channel(over.g)),
        )
    }

    /// WCAG contrast of `text` on `fill`.
    fn contrast(text: Hsla, fill: Hsla) -> f32 {
        let (a, b) = (luminance(text, fill), luminance(fill, fill));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// `OKLab`, the space the generators keep their distances in, so the
    /// thresholds here are the ones they were checked against.
    fn oklab(color: Hsla) -> [f32; 3] {
        let rgb = color.to_rgb();
        let linear = |c: f32| {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        let (red, green, blue) = (linear(rgb.r), linear(rgb.g), linear(rgb.b));
        // The cone responses, cube-rooted, then the Lab axes.
        let long = 0.051_457_1f32
            .mul_add(blue, 0.412_221_47f32.mul_add(red, 0.536_332_55 * green))
            .cbrt();
        let medium = 0.107_396_58f32
            .mul_add(blue, 0.211_903_5f32.mul_add(red, 0.680_699_5 * green))
            .cbrt();
        let short = 0.629_978_1f32
            .mul_add(blue, 0.088_302_46f32.mul_add(red, 0.281_718_84 * green))
            .cbrt();
        [
            0.004_072_047f32.mul_add(
                -short,
                0.210_454_26f32.mul_add(long, 0.793_617_8 * medium),
            ),
            0.450_593_7f32.mul_add(
                short,
                1.977_998_5f32.mul_add(long, -2.428_592_2 * medium),
            ),
            0.808_675_8f32.mul_add(
                -short,
                0.025_904_037f32.mul_add(long, 0.782_771_8 * medium),
            ),
        ]
    }

    fn delta_e(a: Hsla, b: Hsla) -> f32 {
        let (a, b) = (oklab(a), oklab(b));
        (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2])
    }

    // The generators' bars, in OKLab units, less a hair for the trip
    // through `Hsla`'s floats.
    const APART: f32 = 0.079;
    const CLEAR: f32 = 0.099;
    const STEP: f32 = 0.019;

    #[test]
    fn every_label_reads_on_every_fill() {
        for theme in every_theme() {
            for category in CATEGORIES {
                for depth in 0..DEPTHS as u32 {
                    let label = category_label(&theme, category);
                    let fill = category_fill(&theme, category, depth);
                    let ratio = contrast(label, fill);
                    assert!(
                        ratio >= 4.45,
                        "{} {category:?} depth {depth}: {ratio:.2}",
                        theme.name
                    );
                }
            }
            for bucket in 0..AGE_BUCKETS.len() {
                for depth in 0..DEPTHS as u32 {
                    let fill = age_fill(&theme, bucket, depth);
                    let ratio = contrast(label_color(&theme), fill);
                    assert!(
                        ratio >= 4.45,
                        "{} age {bucket} depth {depth}: {ratio:.2}",
                        theme.name
                    );
                }
            }
        }
    }

    #[test]
    fn every_legend_colour_is_its_own() {
        for theme in every_theme() {
            let accents: Vec<(Category, Hsla)> = Category::LEGEND
                .iter()
                .map(|&category| (category, category_accent(&theme, category)))
                .collect();
            for (index, (left, a)) in accents.iter().enumerate() {
                for (right, b) in &accents[index + 1..] {
                    let apart = delta_e(*a, *b);
                    assert!(
                        apart >= APART,
                        "{} {left:?}/{right:?}: {apart:.3}",
                        theme.name
                    );
                }
            }
        }
    }

    #[test]
    fn the_highlight_is_not_a_category_colour() {
        for theme in every_theme() {
            let highlight = highlight(&theme);
            for category in CATEGORIES {
                let accent =
                    delta_e(highlight, category_accent(&theme, category));
                let fill =
                    delta_e(highlight, category_fill(&theme, category, 0));
                assert!(
                    accent >= CLEAR && fill >= CLEAR,
                    "{} {category:?}: accent {accent:.3}, fill {fill:.3}",
                    theme.name
                );
            }
        }
    }

    #[test]
    fn every_depth_is_a_visible_step() {
        for theme in every_theme() {
            for category in CATEGORIES {
                for depth in 1..DEPTHS as u32 {
                    let step = delta_e(
                        category_fill(&theme, category, depth - 1),
                        category_fill(&theme, category, depth),
                    );
                    assert!(
                        step >= STEP,
                        "{} {category:?} depth {depth}: {step:.3}",
                        theme.name
                    );
                }
            }
            for bucket in 0..AGE_BUCKETS.len() {
                for depth in 1..DEPTHS as u32 {
                    let step = delta_e(
                        age_fill(&theme, bucket, depth - 1),
                        age_fill(&theme, bucket, depth),
                    );
                    assert!(
                        step >= STEP,
                        "{} age {bucket} depth {depth}: {step:.3}",
                        theme.name
                    );
                }
            }
        }
    }

    #[test]
    fn depths_past_the_table_draw_as_the_last_step() {
        for theme in every_theme() {
            let last = category_fill(&theme, Category::Code, 4);
            assert_eq!(category_fill(&theme, Category::Code, 9), last);
            let last = age_fill(&theme, 0, 4);
            assert_eq!(age_fill(&theme, 0, 40), last);
        }
    }

    #[test]
    fn age_buckets_cover_every_age_in_order() {
        assert_eq!(age_bucket(0), 0);
        assert_eq!(age_bucket(8), 1);
        assert_eq!(age_bucket(100), 2);
        assert_eq!(age_bucket(300), 3);
        assert_eq!(age_bucket(5000), 4);
        // The newest bucket carries the colour; the oldest is nearly grey.
        for theme in every_theme() {
            let chroma = |bucket| {
                let [_, a, b] = oklab(age_accent(&theme, bucket));
                a.hypot(b)
            };
            assert!(chroma(0) > chroma(4), "{}", theme.name);
        }
    }

    #[test]
    fn mix_clamps_its_parameter() {
        let theme = every_theme().next().expect("a theme");
        let clamped_low = mix(theme.background, theme.accent, -1.0).l;
        let clamped_high = mix(theme.background, theme.accent, 2.0).l;
        assert!((clamped_low - theme.background.l).abs() < 1e-3);
        assert!((clamped_high - theme.accent.l).abs() < 1e-3);
    }
}
