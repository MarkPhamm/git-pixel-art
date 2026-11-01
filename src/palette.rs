//! The colors the lineage graph is drawn in: a night-room blue around GitHub's
//! language colors, pulled down so they sit in the dark instead of glowing.
//!
//! Every blend rounds its amount to a few fixed steps, so a frame keeps a small, exact
//! set of colors that one GIF palette holds.

use crate::history::LanguageColor;

/// One opaque color, as red, green, and blue channels.
pub type Rgb = [u8; 3];

/// The canvas behind everything, near the darkest blue of the night scene.
pub const BACKGROUND: Rgb = [0x07, 0x0a, 0x12];
/// The faint dot grid on the canvas.
pub const GRID_DOT: Rgb = [0x12, 0x17, 0x24];
/// A pipe before it takes on its lineage's tint.
pub const PIPE: Rgb = [0x1e, 0x27, 0x3c];
/// The body of a model node.
pub const NODE_FILL: Rgb = [0x12, 0x18, 0x26];
/// A model node's outline.
pub const NODE_BORDER: Rgb = [0x2a, 0x34, 0x4c];
/// The lit top row of a node.
pub const NODE_HIGHLIGHT: Rgb = [0x32, 0x3c, 0x56];
/// The shaded bottom row of a node.
pub const NODE_SHADOW: Rgb = [0x08, 0x0c, 0x14];
/// The monitor cyan, used for flashing marts.
pub const ACCENT: Rgb = [0x6a, 0x96, 0xa6];
/// Passing tests, at the brightness of the room's plants.
pub const TEST_PASS: Rgb = [0x6e, 0x8f, 0x4e];
/// Layer labels.
pub const LABEL: Rgb = [0x45, 0x50, 0x68];
/// The paused cursor.
pub const CURSOR: Rgb = [0xa8, 0xb4, 0xd0];
/// A language GitHub gives no color, and the source of an account with no language.
pub const UNCOLORED_LANGUAGE: Rgb = [0x6a, 0x73, 0x84];

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

/// The night room's wall. Language colors lean toward it, which drops both brightness
/// and saturation without giving every source the same hue.
const NIGHT_WALL: Rgb = [0x24, 0x30, 0x44];
const NIGHT_DARKEN: f64 = 0.25;
const NIGHT_WALL_MIX: f64 = 0.5;

/// A language's color, or [`UNCOLORED_LANGUAGE`] for a language without one.
pub fn language_rgb(language_color: Option<LanguageColor>) -> Rgb {
    language_color.map_or(UNCOLORED_LANGUAGE, |LanguageColor(channels)| channels)
}

/// Pull a color into the night scene so a source reads as a dim block, not a lamp.
pub fn settle_into_night(color: Rgb) -> Rgb {
    mix(darken(color, NIGHT_DARKEN), NIGHT_WALL, NIGHT_WALL_MIX)
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
