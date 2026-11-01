use std::collections::BTreeMap;

use git_lineage::behavior::Behavior;
use git_lineage::development::{PipelineDevelopment, derive_pipeline_development};
use git_lineage::flow::{FRAMES_PER_SECOND, calculate_flow, frame_count, still_flow};
use git_lineage::history::{AccountSnapshot, LanguageColor};
use git_lineage::layout::{DagLayout, layout_dag};

const BEHAVIORS: [Behavior; 5] = [
    Behavior::Batch,
    Behavior::Streaming,
    Behavior::Ingest,
    Behavior::Idle,
    Behavior::Paused,
];

#[test]
fn loops_last_six_or_eight_seconds_and_wrap_seamlessly() {
    let layout = grown_layout();
    for behavior in BEHAVIORS {
        let loop_frame_count = frame_count(behavior);
        assert!([6, 8].contains(&(loop_frame_count / FRAMES_PER_SECOND)));
        for frame_index in [0, 1, loop_frame_count / 2] {
            assert_eq!(
                calculate_flow(behavior, &layout, frame_index + loop_frame_count),
                calculate_flow(behavior, &layout, frame_index),
                "{behavior:?} frame {frame_index}"
            );
        }
    }
}

#[test]
fn packets_ride_real_pipes_within_their_length() {
    let layout = grown_layout();
    for behavior in BEHAVIORS {
        for frame_index in 0..frame_count(behavior) {
            for packet in calculate_flow(behavior, &layout, frame_index).packets {
                assert!(packet.edge_index < layout.edges.len());
                assert!((0.0..1.0).contains(&packet.progress), "{behavior:?}");
            }
        }
    }
}

#[test]
fn a_batch_crosses_one_gap_at_a_time_then_lands_on_the_marts() {
    let layout = grown_layout();
    let mut visited_gaps = Vec::new();
    let mut flashed = false;
    for frame_index in 0..frame_count(Behavior::Batch) {
        let flow = calculate_flow(Behavior::Batch, &layout, frame_index);
        let mut gaps: Vec<usize> = flow
            .packets
            .iter()
            .map(|packet| layout.edges[packet.edge_index].gap)
            .collect();
        gaps.dedup();
        assert!(gaps.len() <= 1);
        if let Some(&gap) = gaps.first() {
            assert!(flow.mart_flash == 0.0);
            assert_eq!(
                flow.packets.len(),
                layout.edges.iter().filter(|edge| edge.gap == gap).count()
            );
            if visited_gaps.last() != Some(&gap) {
                visited_gaps.push(gap);
            }
        } else {
            flashed |= flow.mart_flash > 0.0;
        }
    }
    assert_eq!(visited_gaps, (0..layout.gap_count()).collect::<Vec<_>>());
    assert!(flashed);
}

#[test]
fn streaming_keeps_three_packets_on_every_pipe_and_flashes_the_marts() {
    let layout = grown_layout();
    let mut flashes = Vec::new();
    for frame_index in 0..frame_count(Behavior::Streaming) {
        let flow = calculate_flow(Behavior::Streaming, &layout, frame_index);
        assert_eq!(flow.packets.len(), 3 * layout.edges.len());
        flashes.push(flow.mart_flash);
    }
    assert!(flashes.iter().any(|&flash| flash > 0.9));
    assert!(flashes.contains(&0.0));
}

#[test]
fn ingest_bursts_out_of_pulsing_sources() {
    let layout = grown_layout();
    let first_gap_edges = layout.edges.iter().filter(|edge| edge.gap == 0).count();
    let mut pulses = Vec::new();
    for frame_index in 0..frame_count(Behavior::Ingest) {
        let flow = calculate_flow(Behavior::Ingest, &layout, frame_index);
        let first_gap_packets = flow
            .packets
            .iter()
            .filter(|packet| layout.edges[packet.edge_index].gap == 0)
            .count();
        assert_eq!(first_gap_packets, 2 * first_gap_edges);
        pulses.push(flow.source_pulse);
    }
    assert!(pulses.iter().any(|&pulse| pulse < 0.05));
    assert!(pulses.iter().any(|&pulse| pulse > 0.95));
}

#[test]
fn idle_sends_one_packet_along_one_path_to_a_mart() {
    let layout = grown_layout();
    let mut path = Vec::new();
    for frame_index in 0..frame_count(Behavior::Idle) {
        let packets = calculate_flow(Behavior::Idle, &layout, frame_index).packets;
        assert!(packets.len() <= 1);
        if let Some(packet) = packets.first()
            && path.last() != Some(&packet.edge_index)
        {
            path.push(packet.edge_index);
        }
    }
    assert_eq!(path.len(), layout.gap_count());
    assert_eq!(layout.edges[path[0]].from, 0);
    for pair in path.windows(2) {
        assert_eq!(layout.edges[pair[0]].to, layout.edges[pair[1]].from);
    }
}

#[test]
fn paused_moves_nothing_dims_everything_and_blinks_a_cursor() {
    let layout = grown_layout();
    let mut cursor_states = Vec::new();
    for frame_index in 0..frame_count(Behavior::Paused) {
        let flow = calculate_flow(Behavior::Paused, &layout, frame_index);
        assert!(flow.packets.is_empty());
        assert_eq!(flow.dim, 1.0);
        cursor_states.push(flow.cursor_visible);
    }
    assert!(cursor_states.contains(&true) && cursor_states.contains(&false));
    for behavior in [
        Behavior::Batch,
        Behavior::Streaming,
        Behavior::Ingest,
        Behavior::Idle,
    ] {
        let flow = calculate_flow(behavior, &layout, 0);
        assert_eq!(flow.dim, 0.0);
        assert!(!flow.cursor_visible);
    }
}

#[test]
fn the_still_holds_one_packet_halfway_along_every_pipe() {
    let layout = grown_layout();
    let flow = still_flow(&layout);
    assert_eq!(flow.packets.len(), layout.edges.len());
    assert!(flow.packets.iter().all(|packet| packet.progress == 0.5));
    assert_eq!(
        (flow.source_pulse, flow.mart_flash, flow.dim),
        (0.0, 0.0, 0.0)
    );
}

fn grown_layout() -> DagLayout {
    layout_dag(&grown_development())
}

fn grown_development() -> PipelineDevelopment {
    let account_snapshot: AccountSnapshot =
        serde_json::from_str(include_str!("fixtures/account-with-history.json")).unwrap();
    let mut development = derive_pipeline_development(&account_snapshot).unwrap();
    development.language_shares =
        BTreeMap::from([("Python".to_owned(), 0.6), ("Rust".to_owned(), 0.4)]);
    development.language_colors = BTreeMap::from([
        ("Python".to_owned(), Some(LanguageColor([0x35, 0x72, 0xa5]))),
        ("Rust".to_owned(), Some(LanguageColor([0xde, 0xa5, 0x84]))),
    ]);
    development.total_contributions = 20_000;
    development
}
