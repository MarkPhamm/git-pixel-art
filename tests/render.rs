use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, Days};
use git_lineage::behavior::Behavior;
use git_lineage::development::{PipelineDevelopment, RecentStreak, derive_pipeline_development};
use git_lineage::flow::frame_count;
use git_lineage::history::{AccountSnapshot, ContributionWeek, LanguageColor};
use git_lineage::palette::{BACKGROUND, CURSOR, TEST_PASS, settle_into_night};
use git_lineage::render::{
    FRAME_HEIGHT_PIXELS, FRAME_WIDTH_PIXELS, Frame, render_flow_frame, render_still,
};

const PYTHON_BLUE: [u8; 3] = [0x35, 0x72, 0xa5];
const BEHAVIORS: [Behavior; 5] = [
    Behavior::Batch,
    Behavior::Streaming,
    Behavior::Ingest,
    Behavior::Idle,
    Behavior::Paused,
];

#[test]
fn every_frame_fills_the_banner_canvas_opaquely() {
    let development = develop(20_000, 4, None);
    for frame in [
        render_still(&development),
        render_flow_frame(&development, Behavior::Streaming, 17),
    ] {
        assert_eq!(
            frame.as_rgba_bytes().len(),
            (FRAME_WIDTH_PIXELS * FRAME_HEIGHT_PIXELS * 4) as usize
        );
        assert!(pixels_of(&frame).all(|pixel| pixel[3] == 255));
        assert_eq!(pixels_of(&frame).next().unwrap()[..3], BACKGROUND);
    }
    assert_eq!(FRAME_WIDTH_PIXELS, 2 * FRAME_HEIGHT_PIXELS);
}

#[test]
fn rendering_is_reproducible_and_loops_wrap() {
    let development = develop(20_000, 4, None);
    assert_eq!(render_still(&development), render_still(&development));
    for behavior in BEHAVIORS {
        assert_eq!(
            render_flow_frame(&development, behavior, 3),
            render_flow_frame(&development, behavior, 3 + frame_count(behavior))
        );
    }
}

#[test]
fn sources_wear_their_language_colors_until_the_graph_pauses() {
    let development = develop(20_000, 4, None);
    let night_blue = settle_into_night(PYTHON_BLUE);
    assert_ne!(night_blue, PYTHON_BLUE);
    assert!(has_color(&render_still(&development), night_blue));
    assert!(has_color(
        &render_flow_frame(&development, Behavior::Batch, 0),
        night_blue
    ));
    assert!(!has_color(&render_still(&development), PYTHON_BLUE));
    let paused_frames: Vec<Frame> = (0..frame_count(Behavior::Paused))
        .map(|frame_index| render_flow_frame(&development, Behavior::Paused, frame_index))
        .collect();
    assert!(
        paused_frames
            .iter()
            .all(|frame| !has_color(frame, PYTHON_BLUE) && !has_color(frame, night_blue))
    );
    assert!(paused_frames.iter().any(|frame| has_color(frame, CURSOR)));
    assert!(paused_frames.iter().any(|frame| !has_color(frame, CURSOR)));
}

#[test]
fn packets_move_between_frames_of_a_running_graph() {
    let development = develop(20_000, 4, None);
    for behavior in [
        Behavior::Batch,
        Behavior::Streaming,
        Behavior::Ingest,
        Behavior::Idle,
    ] {
        assert_ne!(
            render_flow_frame(&development, behavior, 0),
            render_flow_frame(&development, behavior, 5),
            "{behavior:?}"
        );
    }
}

#[test]
fn a_current_streak_turns_the_marts_tests_green_and_they_fade_after_it_ends() {
    let no_streak = render_still(&develop(20_000, 13, None));
    assert!(!has_color(&no_streak, TEST_PASS));
    let full_streak = render_still(&develop(20_000, 13, Some(None)));
    assert!(has_color(&full_streak, TEST_PASS));
    let long_ended = render_still(&develop(20_000, 13, Some(Some(60))));
    assert!(!has_color(&long_ended, TEST_PASS));
}

#[test]
fn every_frame_fits_one_gif_palette() {
    for total_contributions in [0, 300, 3_000, 100_000] {
        let development = develop(total_contributions, 208, Some(None));
        for behavior in BEHAVIORS {
            for frame_index in (0..frame_count(behavior)).step_by(7) {
                let frame = render_flow_frame(&development, behavior, frame_index);
                let colors: BTreeSet<[u8; 4]> = pixels_of(&frame).copied().collect();
                assert!(
                    colors.len() <= 255,
                    "{behavior:?} has {} colors",
                    colors.len()
                );
            }
        }
    }
}

#[test]
fn growth_adds_models_to_the_picture() {
    let distinct_pixels = |total_contributions| {
        let frame = render_still(&develop(total_contributions, 4, None));
        pixels_of(&frame)
            .filter(|pixel| pixel[..3] != BACKGROUND)
            .count()
    };
    assert!(distinct_pixels(100_000) > distinct_pixels(3_000));
    assert!(distinct_pixels(3_000) > distinct_pixels(10));
}

fn pixels_of(frame: &Frame) -> impl Iterator<Item = &[u8; 4]> {
    frame.as_rgba_bytes().as_chunks::<4>().0.iter()
}

fn has_color(frame: &Frame, color: [u8; 3]) -> bool {
    pixels_of(frame).any(|pixel| pixel[..3] == color)
}

/// A two-language development with the given total and longest streak. The recent
/// streak is the longest one, running when `Some(None)` or ended that many days
/// before the observation when `Some(Some(days))`.
fn develop(
    total_contributions: u64,
    longest_weeks: usize,
    recent_streak: Option<Option<u64>>,
) -> PipelineDevelopment {
    let account_snapshot: AccountSnapshot =
        serde_json::from_str(include_str!("fixtures/account-with-history.json")).unwrap();
    let mut development = derive_pipeline_development(&account_snapshot).unwrap();
    development.language_shares =
        BTreeMap::from([("Python".to_owned(), 0.6), ("Rust".to_owned(), 0.4)]);
    development.language_colors = BTreeMap::from([
        ("Python".to_owned(), Some(LanguageColor(PYTHON_BLUE))),
        ("Rust".to_owned(), Some(LanguageColor([0xde, 0xa5, 0x84]))),
    ]);
    development.total_contributions = total_contributions;
    let weeks: Vec<ContributionWeek> = (0..longest_weeks)
        .map(|week_index| ContributionWeek {
            iso_week: (development.observed_on - Days::new(7 * week_index as u64)).iso_week(),
            active_day_count: 7,
            observed_day_count: 7,
        })
        .rev()
        .collect();
    development.streaks.longest = weeks.clone();
    development.streaks.recent = recent_streak.map(|days_since_end| match days_since_end {
        None => RecentStreak::Current(weeks.clone()),
        Some(days_since_end) => RecentStreak::Ended {
            weeks: weeks.clone(),
            ended_on: development.observed_on - Days::new(days_since_end),
        },
    });
    development
}
