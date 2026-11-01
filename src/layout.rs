//! Where the lineage graph's nodes and pipes sit on the canvas.
//!
//! The graph reads left to right in dbt's layers: one source per leading language,
//! then staging, intermediate, and mart models as contributions grow. Every edge is an
//! orthogonal pixel pipe from a node's right side to a child's left side. The account
//! seed decides the wiring, so one account always gets the same graph shape, and
//! growth only adds nodes rather than rerolling the ones already there.

use std::cmp::Ordering;

use crate::development::{
    INTERMEDIATE_LAYER_CONTRIBUTIONS, PipelineDevelopment, STAGING_LAYER_CONTRIBUTIONS,
    calculate_growth_fraction,
};
use crate::history::LanguageColor;
use crate::render::{FRAME_HEIGHT_PIXELS, FRAME_WIDTH_PIXELS};

/// At most this many languages become sources; the rest fold into none.
pub const MAX_SOURCE_COUNT: usize = 5;
/// The widest intermediate layer, reached at full growth.
pub const MAX_INTERMEDIATE_COUNT: usize = 3;
/// The widest mart layer, reached at full growth.
pub const MAX_MART_COUNT: usize = 3;

const MARGIN_LEFT_PIXELS: i32 = 16;
const MARGIN_RIGHT_PIXELS: i32 = 16;
/// Leaves a band above the graph for the layer labels.
const MARGIN_TOP_PIXELS: i32 = 22;
const MARGIN_BOTTOM_PIXELS: i32 = 12;
const NODE_WIDTH_PIXELS: i32 = 40;
const MODEL_HEIGHT_PIXELS: i32 = 12;
const SOURCE_MIN_HEIGHT_PIXELS: i32 = 10;
const SOURCE_MAX_HEIGHT_PIXELS: i32 = 44;
const SOURCE_GAP_PIXELS: i32 = 6;
/// Pipes from neighboring parents, or into neighboring children, whichever column is
/// wider, turn this far apart, so their risers stay distinct.
const RISER_SPACING_PIXELS: i32 = 3;
/// Chance, out of 256, that a model reads from a second parent.
const SECOND_PARENT_CHANCE: u64 = 110;

/// A dbt layer, in the order data flows through them.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Layer {
    /// Raw data, one node per leading language.
    Source,
    /// One cleaned model per source.
    Staging,
    /// Joins and reshapes between staging and marts.
    Intermediate,
    /// The models people read.
    Mart,
}

/// One column of the graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DagColumn {
    /// The layer the column holds.
    pub layer: Layer,
    /// The column's left edge, in native pixels.
    pub left_x: i32,
}

/// One node: a source or a model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DagNode {
    /// The layer the node belongs to.
    pub layer: Layer,
    /// `[x, y, width, height]` in native pixels.
    pub rect: [i32; 4],
    /// The source this node mainly reads from, as an index into the sources; a source
    /// is its own lineage.
    pub lineage: usize,
}

/// One pipe from a parent's right side to a child's left side.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DagEdge {
    /// The parent, as an index into the nodes.
    pub from: usize,
    /// The child, as an index into the nodes.
    pub to: usize,
    /// Which gap between columns the pipe crosses, counted from zero at the sources.
    pub gap: usize,
    /// Every pixel of the pipe in the order data travels it, without repeats.
    pub pixels: Vec<[i32; 2]>,
}

/// One source: the language it stands for and that language's color.
#[derive(Clone, Debug, PartialEq)]
pub struct DagSource {
    /// GitHub's language name; `None` stands for an account with no counted language.
    pub language_name: Option<String>,
    /// The language's share of counted repositories, from zero through one.
    pub share: f64,
    /// GitHub's color for the language; `None` means it has none.
    pub color: Option<LanguageColor>,
}

/// The whole graph, ready to draw.
#[derive(Clone, Debug, PartialEq)]
pub struct DagLayout {
    /// The columns, left to right.
    pub columns: Vec<DagColumn>,
    /// The sources, in the order of their nodes.
    pub sources: Vec<DagSource>,
    /// Every node, column by column and top to bottom within each column. Sources
    /// come first, in the order of `sources`.
    pub nodes: Vec<DagNode>,
    /// Every pipe, ordered by gap, then parent, then child.
    pub edges: Vec<DagEdge>,
}

impl DagLayout {
    /// The number of gaps between columns that pipes cross.
    pub fn gap_count(&self) -> usize {
        self.columns.len() - 1
    }
}

/// Lay out the account's lineage graph.
///
/// The leading languages, by share and then by name, become up to
/// [`MAX_SOURCE_COUNT`] sources whose heights follow their shares; an account with no
/// counted language gets one uncolored source. Below
/// [`STAGING_LAYER_CONTRIBUTIONS`] sources feed marts directly; from there a staging
/// model per source sits between them, and from [`INTERMEDIATE_LAYER_CONTRIBUTIONS`]
/// an intermediate layer follows. The intermediate and mart layers widen along
/// [`calculate_growth_fraction`]. Each staging model reads its own source; every other
/// model reads the parent nearest its position, and sometimes a second parent the
/// account seed picks. Every parent feeds at least one child. Identical inputs
/// produce identical layouts without I/O, clock reads, or input mutation.
pub fn layout_dag(development: &PipelineDevelopment) -> DagLayout {
    let sources = choose_sources(development);
    let growth_fraction = calculate_growth_fraction(development.total_contributions);
    let widen = |max_count: usize| 1 + (growth_fraction * (max_count - 1) as f64).round() as usize;
    let mut layer_counts = vec![(Layer::Source, sources.len())];
    if development.total_contributions >= STAGING_LAYER_CONTRIBUTIONS {
        layer_counts.push((Layer::Staging, sources.len()));
    }
    if development.total_contributions >= INTERMEDIATE_LAYER_CONTRIBUTIONS {
        layer_counts.push((Layer::Intermediate, widen(MAX_INTERMEDIATE_COUNT)));
    }
    layer_counts.push((Layer::Mart, widen(MAX_MART_COUNT)));

    let column_count = layer_counts.len() as i32;
    let usable_width =
        FRAME_WIDTH_PIXELS as i32 - MARGIN_LEFT_PIXELS - MARGIN_RIGHT_PIXELS - NODE_WIDTH_PIXELS;
    let columns: Vec<DagColumn> = layer_counts
        .iter()
        .enumerate()
        .map(|(column_index, &(layer, _))| DagColumn {
            layer,
            left_x: MARGIN_LEFT_PIXELS + usable_width * column_index as i32 / (column_count - 1),
        })
        .collect();

    let mut nodes = place_sources(&sources, columns[0].left_x);
    let mut column_ranges = Vec::with_capacity(layer_counts.len());
    column_ranges.push(0..nodes.len());
    let mut edges = Vec::new();
    let mut seed_state = development.identity_seed ^ 0x6c69_6e65_6167_6521;
    for (column_index, &(layer, node_count)) in layer_counts.iter().enumerate().skip(1) {
        let parent_range = column_ranges[column_index - 1].clone();
        let tops = spread_models(node_count);
        let wiring = if layer == Layer::Staging {
            (0..node_count).map(|child| vec![child]).collect()
        } else {
            wire_children(parent_range.len(), node_count, &mut seed_state)
        };
        let first_child = nodes.len();
        for (child_offset, parent_offsets) in wiring.iter().enumerate() {
            let primary_parent = parent_range.start + parent_offsets[0];
            nodes.push(DagNode {
                layer,
                rect: [
                    columns[column_index].left_x,
                    tops[child_offset],
                    NODE_WIDTH_PIXELS,
                    MODEL_HEIGHT_PIXELS,
                ],
                lineage: nodes[primary_parent].lineage,
            });
        }
        for (child_offset, parent_offsets) in wiring.iter().enumerate() {
            let child = first_child + child_offset;
            for &parent_offset in parent_offsets {
                let parent = parent_range.start + parent_offset;
                let (lane, lane_count) = if parent_range.len() > node_count {
                    (parent_offset, parent_range.len())
                } else {
                    (child_offset, node_count)
                };
                let riser_offset =
                    (lane as i32 * 2 - lane_count as i32 + 1) * RISER_SPACING_PIXELS / 2;
                edges.push(DagEdge {
                    from: parent,
                    to: child,
                    gap: column_index - 1,
                    pixels: trace_pipe(&nodes[parent].rect, &nodes[child].rect, riser_offset),
                });
            }
        }
        column_ranges.push(first_child..nodes.len());
    }
    edges.sort_by_key(|edge| (edge.gap, edge.from, edge.to));
    DagLayout {
        columns,
        sources,
        nodes,
        edges,
    }
}

fn choose_sources(development: &PipelineDevelopment) -> Vec<DagSource> {
    let mut ranked: Vec<(&String, f64)> = development
        .language_shares
        .iter()
        .map(|(language_name, &share)| (language_name, share))
        .collect();
    ranked.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.0.cmp(right.0))
    });
    let sources: Vec<DagSource> = ranked
        .into_iter()
        .take(MAX_SOURCE_COUNT)
        .map(|(language_name, share)| DagSource {
            language_name: Some(language_name.clone()),
            share,
            color: development
                .language_colors
                .get(language_name)
                .copied()
                .flatten(),
        })
        .collect();
    if sources.is_empty() {
        vec![DagSource {
            language_name: None,
            share: 1.0,
            color: None,
        }]
    } else {
        sources
    }
}

/// Stack the sources in the first column, centered, each as tall as its share earns.
fn place_sources(sources: &[DagSource], left_x: i32) -> Vec<DagNode> {
    let available_height = FRAME_HEIGHT_PIXELS as i32 - MARGIN_TOP_PIXELS - MARGIN_BOTTOM_PIXELS;
    let source_count = sources.len() as i32;
    let spare_height = available_height
        - source_count * SOURCE_MIN_HEIGHT_PIXELS
        - (source_count - 1) * SOURCE_GAP_PIXELS;
    let heights: Vec<i32> = sources
        .iter()
        .map(|source| {
            (SOURCE_MIN_HEIGHT_PIXELS + (source.share * spare_height as f64).round() as i32)
                .min(SOURCE_MAX_HEIGHT_PIXELS)
        })
        .collect();
    let stack_height = heights.iter().sum::<i32>() + (source_count - 1) * SOURCE_GAP_PIXELS;
    let mut top = MARGIN_TOP_PIXELS + (available_height - stack_height) / 2;
    heights
        .iter()
        .enumerate()
        .map(|(source_index, &height)| {
            let node = DagNode {
                layer: Layer::Source,
                rect: [left_x, top, NODE_WIDTH_PIXELS, height],
                lineage: source_index,
            };
            top += height + SOURCE_GAP_PIXELS;
            node
        })
        .collect()
}

/// Tops of evenly spaced model nodes in one column.
fn spread_models(node_count: usize) -> Vec<i32> {
    let available_height = FRAME_HEIGHT_PIXELS as i32 - MARGIN_TOP_PIXELS - MARGIN_BOTTOM_PIXELS;
    let slot_height = available_height / node_count as i32;
    (0..node_count as i32)
        .map(|slot| {
            MARGIN_TOP_PIXELS + slot * slot_height + (slot_height - MODEL_HEIGHT_PIXELS) / 2
        })
        .collect()
}

/// Each child's parents, as offsets into the parent column, primary parent first.
fn wire_children(parent_count: usize, child_count: usize, seed_state: &mut u64) -> Vec<Vec<usize>> {
    let nearest_parent = |child: usize| {
        if child_count == 1 {
            (parent_count - 1) / 2
        } else {
            (child * (parent_count - 1) + (child_count - 1) / 2) / (child_count - 1)
        }
    };
    let mut wiring: Vec<Vec<usize>> = (0..child_count)
        .map(|child| vec![nearest_parent(child)])
        .collect();
    for parents in &mut wiring {
        let roll = next_random(seed_state);
        if parent_count > 1 && roll % 256 < SECOND_PARENT_CHANCE {
            let other_parent =
                (parents[0] + 1 + (roll >> 8) as usize % (parent_count - 1)) % parent_count;
            parents.push(other_parent);
        }
    }
    for parent in 0..parent_count {
        if !wiring.iter().any(|parents| parents.contains(&parent)) {
            let child = if parent_count == 1 {
                0
            } else {
                (parent * (child_count - 1) + (parent_count - 1) / 2) / (parent_count - 1)
            };
            wiring[child].push(parent);
        }
    }
    wiring
}

/// The pixels of one pipe: out of the parent's right side, along to the riser, up or
/// down to the child's row, and into the child's left side.
fn trace_pipe(parent_rect: &[i32; 4], child_rect: &[i32; 4], riser_offset: i32) -> Vec<[i32; 2]> {
    let start = [
        parent_rect[0] + parent_rect[2],
        parent_rect[1] + parent_rect[3] / 2,
    ];
    let end = [child_rect[0] - 1, child_rect[1] + child_rect[3] / 2];
    let riser_x = ((start[0] + end[0]) / 2 + riser_offset).clamp(start[0] + 2, end[0] - 2);
    let mut pixels = Vec::new();
    let mut push = |pixel: [i32; 2]| {
        if pixels.last() != Some(&pixel) {
            pixels.push(pixel);
        }
    };
    for x in start[0]..=riser_x {
        push([x, start[1]]);
    }
    let step = if end[1] >= start[1] { 1 } else { -1 };
    let mut y = start[1];
    while y != end[1] {
        y += step;
        push([riser_x, y]);
    }
    for x in riser_x..=end[0] {
        push([x, end[1]]);
    }
    pixels
}

/// SplitMix64: a small, well-mixed generator, so wiring depends only on the seed.
fn next_random(seed_state: &mut u64) -> u64 {
    *seed_state = seed_state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut mixed = *seed_state;
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    mixed ^ (mixed >> 31)
}
