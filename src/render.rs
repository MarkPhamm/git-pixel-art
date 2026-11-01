//! A pixel-art dbt lineage graph colored from the account's languages, on an opaque
//! night-blue canvas.
//!
//! Sources are drawn as database blocks in their language colors, models as dark cards
//! with a stripe in the color of the source they read from, and pipes in a tint of the
//! same lineage. Packets ride the pipes in each frame of a behavior's loop. Streaks put
//! "tests passed" badges and a green glow on the marts.
//!
//! Every graph shares one banner-shaped native canvas of [`FRAME_WIDTH_PIXELS`] by
//! [`FRAME_HEIGHT_PIXELS`], which the encoders in [`crate::output`] scale up exactly.
//! Positions live in [`crate::layout`], colors in [`crate::palette`], streak strengths
//! in [`crate::badges`], and packets in [`crate::flow`].

use crate::badges::{StreakBadges, calculate_streak_badges};
use crate::behavior::Behavior;
use crate::development::PipelineDevelopment;
use crate::flow::{FlowState, calculate_flow, still_flow};
use crate::layout::{DagLayout, Layer, layout_dag};
use crate::palette::{
    ACCENT, BACKGROUND, CURSOR, GRID_DOT, LABEL, NODE_BORDER, NODE_FILL, NODE_HIGHLIGHT,
    NODE_SHADOW, PIPE, Rgb, TEST_PASS, apply_vibrancy, darken, dim, language_rgb, lighten, mix,
    settle_into_night,
};

/// Width of the native canvas every graph is drawn on, at every growth level.
pub const FRAME_WIDTH_PIXELS: u32 = 384;
/// Height of the native canvas; the canvas is a two-to-one banner.
pub const FRAME_HEIGHT_PIXELS: u32 = 192;

const GRID_SPACING_PIXELS: i32 = 8;
const LABEL_TOP_PIXELS: i32 = 8;
const BADGE_SIZE_PIXELS: i32 = 7;
/// The check mark inside a badge, as offsets from its top left.
const CHECK_MARK_PIXELS: [[i32; 2]; 5] = [[1, 3], [2, 4], [3, 3], [4, 2], [5, 1]];
/// A dimmed graph keeps this share of its color.
const DIM_SHARE: f64 = 0.75;

/// A renderer-owned, fixed-size frame with row-major RGBA pixels.
#[derive(Debug, Eq, PartialEq)]
pub struct Frame {
    rgba_pixels: Vec<[u8; 4]>,
}

impl Frame {
    /// The pixels as bytes, row by row from the top left, four channels per pixel: red,
    /// green, blue, alpha. The length is the canvas area times four.
    pub fn as_rgba_bytes(&self) -> &[u8] {
        self.rgba_pixels.as_flattened()
    }

    /// Build a frame from chosen pixels, for encoder tests that need colors no graph has.
    #[cfg(test)]
    pub(crate) fn from_rgba_pixels(rgba_pixels: Vec<[u8; 4]>) -> Self {
        Self { rgba_pixels }
    }
}

/// Render the graph still, with one packet halfway along every pipe. Contribution
/// totals set how many layers and models it has, languages color its sources, and
/// streaks set its badges and glow as of the observation date. Identical inputs
/// produce identical pixels without I/O, clock reads, or input mutation.
pub fn render_still(development: &PipelineDevelopment) -> Frame {
    let layout = layout_dag(development);
    let flow = still_flow(&layout);
    draw_graph(&layout, &calculate_streak_badges(development), &flow)
}

/// Render one frame of a behavior's seamless loop, drawn like `render_still` with the
/// packets, pulse, flash, and dimming `flow::calculate_flow` gives; frame numbers wrap
/// around the loop of `flow::frame_count(behavior)` frames. Identical inputs produce
/// identical pixels without I/O, clock reads, or input mutation.
pub fn render_flow_frame(
    development: &PipelineDevelopment,
    behavior: Behavior,
    frame_index: u32,
) -> Frame {
    let layout = layout_dag(development);
    let flow = calculate_flow(behavior, &layout, frame_index);
    draw_graph(&layout, &calculate_streak_badges(development), &flow)
}

/// An opaque canvas that ignores drawing outside its bounds.
struct Canvas {
    pixels: Vec<Rgb>,
}

impl Canvas {
    fn new() -> Self {
        Self {
            pixels: vec![BACKGROUND; (FRAME_WIDTH_PIXELS * FRAME_HEIGHT_PIXELS) as usize],
        }
    }

    fn set(&mut self, x: i32, y: i32, color: Rgb) {
        if (0..FRAME_WIDTH_PIXELS as i32).contains(&x)
            && (0..FRAME_HEIGHT_PIXELS as i32).contains(&y)
        {
            self.pixels[(y * FRAME_WIDTH_PIXELS as i32 + x) as usize] = color;
        }
    }

    fn fill_rect(&mut self, [x, y, width, height]: [i32; 4], color: Rgb) {
        for row in y..y + height {
            for column in x..x + width {
                self.set(column, row, color);
            }
        }
    }

    fn outline_rect(&mut self, [x, y, width, height]: [i32; 4], color: Rgb) {
        for column in x..x + width {
            self.set(column, y, color);
            self.set(column, y + height - 1, color);
        }
        for row in y..y + height {
            self.set(x, row, color);
            self.set(x + width - 1, row, color);
        }
    }

    fn into_frame(self) -> Frame {
        Frame {
            rgba_pixels: self
                .pixels
                .into_iter()
                .map(|[red, green, blue]| [red, green, blue, 255])
                .collect(),
        }
    }
}

fn draw_graph(layout: &DagLayout, badges: &StreakBadges, flow: &FlowState) -> Frame {
    let mut canvas = Canvas::new();
    for y in
        (GRID_SPACING_PIXELS / 2..FRAME_HEIGHT_PIXELS as i32).step_by(GRID_SPACING_PIXELS as usize)
    {
        for x in (GRID_SPACING_PIXELS / 2..FRAME_WIDTH_PIXELS as i32)
            .step_by(GRID_SPACING_PIXELS as usize)
        {
            canvas.set(x, y, GRID_DOT);
        }
    }
    for column in &layout.columns {
        draw_label(
            &mut canvas,
            column.left_x,
            LABEL_TOP_PIXELS,
            label_for(column.layer),
        );
    }
    let lineage_colors: Vec<Rgb> = layout
        .sources
        .iter()
        .map(|source| {
            settle_into_night(apply_vibrancy(
                language_rgb(source.color),
                badges.color_vibrancy,
            ))
        })
        .collect();

    for edge in &layout.edges {
        let pipe_color = mix(PIPE, lineage_colors[layout.nodes[edge.from].lineage], 0.375);
        for &[x, y] in &edge.pixels {
            canvas.set(x, y, pipe_color);
        }
    }

    let mart_indices: Vec<usize> = (0..layout.nodes.len())
        .filter(|&node_index| layout.nodes[node_index].layer == Layer::Mart)
        .collect();
    let badge_count = (badges.badge_strength * mart_indices.len() as f64 - 1e-9)
        .ceil()
        .max(0.0) as usize;
    for (node_index, node) in layout.nodes.iter().enumerate() {
        let lineage_color = lineage_colors[node.lineage];
        let [x, y, width, height] = node.rect;
        if node.layer == Layer::Source {
            let body = lighten(lineage_color, flow.source_pulse * 0.5);
            canvas.fill_rect(node.rect, body);
            canvas.fill_rect([x, y, width, 1], lighten(body, 0.125));
            canvas.fill_rect([x, y + 3, width, 1], darken(body, 0.25));
            canvas.fill_rect([x, y + height - 1, width, 1], darken(body, 0.5));
            continue;
        }
        let is_mart = node.layer == Layer::Mart;
        let flash = if is_mart { flow.mart_flash } else { 0.0 };
        if is_mart && badges.glow_strength > 0.0 {
            canvas.outline_rect(
                [x - 1, y - 1, width + 2, height + 2],
                mix(BACKGROUND, TEST_PASS, 0.25 + 0.5 * badges.glow_strength),
            );
        }
        canvas.fill_rect(node.rect, mix(NODE_FILL, ACCENT, flash * 0.25));
        canvas.fill_rect([x + 1, y + 1, width - 2, 1], NODE_HIGHLIGHT);
        canvas.fill_rect([x + 1, y + height - 2, width - 2, 1], NODE_SHADOW);
        canvas.outline_rect(node.rect, mix(NODE_BORDER, ACCENT, flash));
        canvas.fill_rect([x + 1, y + 1, 3, height - 2], lineage_color);
        let line_lengths = [
            12 + (node_index as i32 * 7) % 16,
            8 + (node_index as i32 * 5) % 12,
        ];
        for (line_index, line_length) in line_lengths.into_iter().enumerate() {
            canvas.fill_rect(
                [x + 7, y + 4 + 3 * line_index as i32, line_length, 1],
                NODE_BORDER,
            );
        }
    }

    for &mart_index in mart_indices.iter().take(badge_count) {
        let [x, y, width, _] = layout.nodes[mart_index].rect;
        let badge_color = mix(NODE_BORDER, TEST_PASS, badges.badge_strength);
        let badge_left = x + width - BADGE_SIZE_PIXELS + 3;
        let badge_top = y - 3;
        canvas.fill_rect(
            [
                badge_left + 1,
                badge_top,
                BADGE_SIZE_PIXELS - 2,
                BADGE_SIZE_PIXELS,
            ],
            badge_color,
        );
        canvas.fill_rect(
            [
                badge_left,
                badge_top + 1,
                BADGE_SIZE_PIXELS,
                BADGE_SIZE_PIXELS - 2,
            ],
            badge_color,
        );
        for [check_x, check_y] in CHECK_MARK_PIXELS {
            canvas.set(badge_left + check_x, badge_top + check_y, BACKGROUND);
        }
    }

    for packet in &flow.packets {
        let edge = &layout.edges[packet.edge_index];
        let pixel_index =
            (packet.progress.clamp(0.0, 1.0) * (edge.pixels.len() - 1) as f64).round() as usize;
        let [x, y] = edge.pixels[pixel_index];
        let lineage_color = lineage_colors[layout.nodes[edge.from].lineage];
        canvas.fill_rect([x - 1, y - 1, 3, 3], lighten(lineage_color, 0.25));
        canvas.set(x, y, lighten(lineage_color, 0.75));
    }

    if flow.dim > 0.0 {
        for pixel in &mut canvas.pixels {
            if *pixel != BACKGROUND && *pixel != GRID_DOT {
                *pixel = dim(*pixel, flow.dim * DIM_SHARE);
            }
        }
    }
    if flow.cursor_visible {
        let [x, y, width, height] = layout.nodes[0].rect;
        canvas.fill_rect([x + width + 3, y + height / 2 - 3, 3, 7], CURSOR);
    }
    canvas.into_frame()
}

fn label_for(layer: Layer) -> &'static str {
    match layer {
        Layer::Source => "SRC",
        Layer::Staging => "STG",
        Layer::Intermediate => "INT",
        Layer::Mart => "MART",
    }
}

/// Draw capital letters in a three-by-five pixel font, one blank column apart.
fn draw_label(canvas: &mut Canvas, left_x: i32, top_y: i32, text: &str) {
    for (letter_index, letter) in text.chars().enumerate() {
        let rows = glyph_rows(letter);
        for (row_index, row_bits) in rows.into_iter().enumerate() {
            for column_index in 0..3 {
                if row_bits & (0b100 >> column_index) != 0 {
                    canvas.set(
                        left_x + 4 * letter_index as i32 + column_index,
                        top_y + row_index as i32,
                        LABEL,
                    );
                }
            }
        }
    }
}

/// Each row of a letter, top first, with the leftmost pixel in the highest of three bits.
fn glyph_rows(letter: char) -> [u8; 5] {
    match letter {
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'C' => [0b111, 0b100, 0b100, 0b100, 0b111],
        'G' => [0b111, 0b100, 0b101, 0b101, 0b111],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'N' => [0b110, 0b101, 0b101, 0b101, 0b101],
        'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b111, 0b100, 0b111, 0b001, 0b111],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        _ => [0; 5],
    }
}
