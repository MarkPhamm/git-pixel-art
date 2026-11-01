//! Data moving over time: where the packets are in each frame of a behavior's loop.

use std::f64::consts::TAU;

use crate::behavior::Behavior;
use crate::layout::DagLayout;

/// Frames per second of every behavior's loop; GIF frame delays derive from it.
pub const FRAMES_PER_SECOND: u32 = 15;

const BATCH_FRAME_COUNT: u32 = 120;
const STREAMING_FRAME_COUNT: u32 = 120;
const INGEST_FRAME_COUNT: u32 = 90;
const IDLE_FRAME_COUNT: u32 = 90;
const PAUSED_FRAME_COUNT: u32 = 120;
/// Share of a batch loop the wave spends crossing the graph; the marts flash for the
/// rest.
const BATCH_TRAVEL_SHARE: f64 = 0.85;
/// Share of an idle loop the lone packet spends crossing the graph; the rest is empty.
const IDLE_TRAVEL_SHARE: f64 = 0.8;
/// Streaming sends this many packets down each pipe at once, and they lap it this
/// many times per loop.
const STREAMING_PACKETS_PER_EDGE: u32 = 3;
const STREAMING_LAPS_PER_LOOP: f64 = 2.0;
const STREAMING_FLASHES_PER_LOOP: f64 = 4.0;
/// Ingest sends this many packets out of each source at once, this many times per loop.
const INGEST_PACKETS_PER_EDGE: u32 = 2;
const INGEST_BURSTS_PER_LOOP: f64 = 3.0;
const PAUSED_BLINKS_PER_LOOP: f64 = 4.0;
/// Spreads each pipe's packets out of step with its neighbors.
const GOLDEN_RATIO_FRACTION: f64 = 0.618_033_988_749_895;

/// One packet on one pipe.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Packet {
    /// The pipe, as an index into the layout's edges.
    pub edge_index: usize,
    /// How far along the pipe, from zero at the parent up to one at the child.
    pub progress: f64,
}

/// What moves and glows in one frame.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowState {
    /// Every packet in flight.
    pub packets: Vec<Packet>,
    /// How brightly the sources pulse, from zero through one.
    pub source_pulse: f64,
    /// How brightly the marts flash, from zero through one.
    pub mart_flash: f64,
    /// How far the graph is dimmed toward grey, from zero through one.
    pub dim: f64,
    /// Whether the paused cursor shows beside the first source.
    pub cursor_visible: bool,
}

/// Frames in a behavior's loop at FRAMES_PER_SECOND.
pub fn frame_count(behavior: Behavior) -> u32 {
    match behavior {
        Behavior::Batch => BATCH_FRAME_COUNT,
        Behavior::Streaming => STREAMING_FRAME_COUNT,
        Behavior::Ingest => INGEST_FRAME_COUNT,
        Behavior::Idle => IDLE_FRAME_COUNT,
        Behavior::Paused => PAUSED_FRAME_COUNT,
    }
}

/// The still image's flow: one packet halfway along every pipe, nothing pulsing,
/// flashing, or dimmed.
pub fn still_flow(layout: &DagLayout) -> FlowState {
    FlowState {
        packets: (0..layout.edges.len())
            .map(|edge_index| Packet {
                edge_index,
                progress: 0.5,
            })
            .collect(),
        ..quiet_flow()
    }
}

/// The flow for a frame of a behavior's loop; frame numbers wrap around the loop, so
/// frame `frame_count(behavior)` is frame zero again.
///
/// A batch sends one packet down every pipe of a gap at a time, gap after gap, then
/// flashes the marts as the wave lands. Streaming keeps several packets lapping every
/// pipe, out of step with each other, while the marts flash. Ingest bursts packets out
/// of pulsing sources and lets single packets drift through the rest. Idle sends one
/// packet along a single path from the first source to a mart. Paused moves nothing,
/// dims the graph, and blinks a cursor. Every periodic value completes whole cycles per
/// loop, so loops close seamlessly.
pub fn calculate_flow(behavior: Behavior, layout: &DagLayout, frame_index: u32) -> FlowState {
    let loop_frame_count = frame_count(behavior);
    let loop_phase = f64::from(frame_index % loop_frame_count) / f64::from(loop_frame_count);
    let gap_count = layout.gap_count() as f64;
    let edge_offset = |edge_index: usize| (edge_index as f64 * GOLDEN_RATIO_FRACTION).fract();
    match behavior {
        Behavior::Batch => {
            let wave_position = loop_phase / BATCH_TRAVEL_SHARE * gap_count;
            let packets = if loop_phase < BATCH_TRAVEL_SHARE {
                let wave_gap = wave_position.floor() as usize;
                packets_where(layout, |_, gap| {
                    (gap == wave_gap)
                        .then_some(wave_position.fract())
                        .into_iter()
                        .collect()
                })
            } else {
                Vec::new()
            };
            let mart_flash = if loop_phase < BATCH_TRAVEL_SHARE {
                0.0
            } else {
                1.0 - (loop_phase - BATCH_TRAVEL_SHARE) / (1.0 - BATCH_TRAVEL_SHARE)
            };
            FlowState {
                packets,
                mart_flash,
                ..quiet_flow()
            }
        }
        Behavior::Streaming => FlowState {
            packets: packets_where(layout, |edge_index, _| {
                (0..STREAMING_PACKETS_PER_EDGE)
                    .map(|packet_index| {
                        (STREAMING_LAPS_PER_LOOP * loop_phase
                            + f64::from(packet_index) / f64::from(STREAMING_PACKETS_PER_EDGE)
                            + edge_offset(edge_index))
                        .fract()
                    })
                    .collect()
            }),
            mart_flash: (TAU * STREAMING_FLASHES_PER_LOOP * loop_phase)
                .cos()
                .max(0.0),
            ..quiet_flow()
        },
        Behavior::Ingest => FlowState {
            packets: packets_where(layout, |edge_index, gap| {
                if gap == 0 {
                    (0..INGEST_PACKETS_PER_EDGE)
                        .map(|packet_index| {
                            (INGEST_BURSTS_PER_LOOP * loop_phase
                                + f64::from(packet_index) / f64::from(INGEST_PACKETS_PER_EDGE)
                                + edge_offset(edge_index))
                            .fract()
                        })
                        .collect()
                } else {
                    vec![(loop_phase + edge_offset(edge_index)).fract()]
                }
            }),
            source_pulse: 0.5 - 0.5 * (TAU * INGEST_BURSTS_PER_LOOP * loop_phase).cos(),
            ..quiet_flow()
        },
        Behavior::Idle => {
            let trickle_path = trace_trickle_path(layout);
            let packets = if loop_phase < IDLE_TRAVEL_SHARE {
                let path_position = loop_phase / IDLE_TRAVEL_SHARE * trickle_path.len() as f64;
                vec![Packet {
                    edge_index: trickle_path[path_position.floor() as usize],
                    progress: path_position.fract(),
                }]
            } else {
                Vec::new()
            };
            FlowState {
                packets,
                ..quiet_flow()
            }
        }
        Behavior::Paused => FlowState {
            dim: 1.0,
            cursor_visible: (PAUSED_BLINKS_PER_LOOP * loop_phase).fract() < 0.5,
            ..quiet_flow()
        },
    }
}

fn quiet_flow() -> FlowState {
    FlowState {
        packets: Vec::new(),
        source_pulse: 0.0,
        mart_flash: 0.0,
        dim: 0.0,
        cursor_visible: false,
    }
}

/// Packets for every edge, at the progresses the rule gives each edge and its gap.
fn packets_where(
    layout: &DagLayout,
    progresses_for: impl Fn(usize, usize) -> Vec<f64>,
) -> Vec<Packet> {
    layout
        .edges
        .iter()
        .enumerate()
        .flat_map(|(edge_index, edge)| {
            progresses_for(edge_index, edge.gap)
                .into_iter()
                .map(move |progress| Packet {
                    edge_index,
                    progress,
                })
        })
        .collect()
}

/// One edge per gap, from the first source's first pipe onward through each child's
/// first outgoing pipe. Every node before the marts feeds a child, so the path reaches
/// a mart.
fn trace_trickle_path(layout: &DagLayout) -> Vec<usize> {
    let mut path = Vec::new();
    let mut current_node = 0;
    for gap in 0..layout.gap_count() {
        let Some(edge_index) = layout
            .edges
            .iter()
            .position(|edge| edge.gap == gap && edge.from == current_node)
        else {
            break;
        };
        path.push(edge_index);
        current_node = layout.edges[edge_index].to;
    }
    path
}
