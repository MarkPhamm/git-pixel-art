//! The colors the lineage graph is drawn in: a Tokyo Night theme around GitHub's
//! language colors.
//!
//! Every blend rounds its amount to a few fixed steps, so a frame keeps a small, exact
//! set of colors that one GIF palette holds.

use crate::history::LanguageColor;

/// One opaque color, as red, green, and blue channels.
pub type Rgb = [u8; 3];

/// The canvas behind everything.
pub const BACKGROUND: Rgb = [0x0b, 0x0f, 0x19];
/// The faint dot grid on the canvas.
pub const GRID_DOT: Rgb = [0x16, 0x1c, 0x2c];
/// A pipe before it takes on its lineage's tint.
pub const PIPE: Rgb = [0x2a, 0x31, 0x50];
/// The body of a model node.
pub const NODE_FILL: Rgb = [0x1a, 0x21, 0x33];
/// A model node's outline.
pub const NODE_BORDER: Rgb = [0x3b, 0x42, 0x61];
/// The lit top row of a node.
pub const NODE_HIGHLIGHT: Rgb = [0x41, 0x48, 0x68];
/// The shaded bottom row of a node.
pub const NODE_SHADOW: Rgb = [0x10, 0x14, 0x1f];
/// The profile's accent, used for flashing marts.
pub const ACCENT: Rgb = [0x88, 0xc0, 0xd0];
/// Passing tests.
pub const TEST_PASS: Rgb = [0x9e, 0xce, 0x6a];
/// Layer labels.
pub const LABEL: Rgb = [0x56, 0x5f, 0x89];
/// The paused cursor.
pub const CURSOR: Rgb = [0xc0, 0xca, 0xf5];
/// A language GitHub gives no color, and the source of an account with no language.
pub const UNCOLORED_LANGUAGE: Rgb = [0x8b, 0x94, 0x9e];

const WHITE: Rgb = [0xff; 3];
/// Blend amounts round to this many steps between the two colors.
const BLEND_STEPS: f64 = 8.0;
/// Vibrancy rounds to this many steps per unit.
const VIBRANCY_STEPS: f64 = 4.0;
/// A dull language color keeps this share of its saturation.
const DULL_SATURATION: f64 = 0.4;
/// A vivid language color pushes its saturation up by this share.
const VIVID_SATURATION_GAIN: f64 = 0.35;
/// A dimmed color turns grey at this share of its brightness.
const DIM_BRIGHTNESS: f64 = 0.55;

/// A language's color, or [`UNCOLORED_LANGUAGE`] for a language without one.
pub fn language_rgb(language_color: Option<LanguageColor>) -> Rgb {
    language_color.map_or(UNCOLORED_LANGUAGE, |LanguageColor(channels)| channels)
}

/// Blend from one color toward another; the amount, from zero through one, rounds to
/// one of a few fixed steps.
pub fn mix(from: Rgb, to: Rgb, amount: f64) -> Rgb {
    let stepped_amount = (amount.clamp(0.0, 1.0) * BLEND_STEPS).round() / BLEND_STEPS;
    [0, 1, 2].map(|channel| {
        (f64::from(from[channel])
            + (f64::from(to[channel]) - f64::from(from[channel])) * stepped_amount)
            .round() as u8
    })
}

/// Blend toward white.
pub fn lighten(color: Rgb, amount: f64) -> Rgb {
    mix(color, WHITE, amount)
}

/// Blend toward the background.
pub fn darken(color: Rgb, amount: f64) -> Rgb {
    mix(color, BACKGROUND, amount)
}

/// Adjust a language color for the longest streak: vibrancy 0 is dull, 1 the color as
/// GitHub gives it, and 2 extra saturated. Vibrancy rounds to quarter steps.
pub fn apply_vibrancy(color: Rgb, vibrancy: f64) -> Rgb {
    let stepped_vibrancy = (vibrancy.clamp(0.0, 2.0) * VIBRANCY_STEPS).round() / VIBRANCY_STEPS;
    let saturation_scale = if stepped_vibrancy <= 1.0 {
        DULL_SATURATION + (1.0 - DULL_SATURATION) * stepped_vibrancy
    } else {
        1.0 + VIVID_SATURATION_GAIN * (stepped_vibrancy - 1.0)
    };
    let gray = luminance(color);
    color.map(|channel| {
        (gray + (f64::from(channel) - gray) * saturation_scale)
            .round()
            .clamp(0.0, 255.0) as u8
    })
}

/// Blend toward a darkened grey of the same brightness, for a paused graph.
pub fn dim(color: Rgb, amount: f64) -> Rgb {
    let gray = (luminance(color) * DIM_BRIGHTNESS).round() as u8;
    mix(color, [gray; 3], amount)
}

fn luminance(color: Rgb) -> f64 {
    0.299 * f64::from(color[0]) + 0.587 * f64::from(color[1]) + 0.114 * f64::from(color[2])
}
