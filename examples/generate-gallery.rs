//! Render a gallery of lineage graphs into a folder: every behavior's loop, a still at
//! each growth stage, and the streak badges from none to a long record.
//!
//! Run with `cargo run --example generate-gallery -- DIR`.

use std::collections::BTreeMap;
use std::{env, error::Error, fs, path::Path, process::ExitCode};

use chrono::{Datelike, Days, NaiveDate};
use git_lineage::behavior::Behavior;
use git_lineage::development::{
    INTERMEDIATE_LAYER_CONTRIBUTIONS, MAX_GROWTH_CONTRIBUTIONS, PipelineDevelopment,
    RecentActivity, RecentStreak, STAGING_LAYER_CONTRIBUTIONS, StreakDevelopment,
};
use git_lineage::flow::frame_count;
use git_lineage::history::{ContributionWeek, LanguageColor};
use git_lineage::output::{encode_gif, encode_png};
use git_lineage::render::{Frame, render_flow_frame, render_still};

/// A data engineer's language mix, with GitHub's colors and shares of projects.
const LANGUAGE_MIX: [(&str, [u8; 3], f64); 5] = [
    ("Python", [0x35, 0x72, 0xa5], 0.4),
    ("Jupyter Notebook", [0xda, 0x5b, 0x0b], 0.22),
    ("HTML", [0xe3, 0x4c, 0x26], 0.14),
    ("HCL", [0x84, 0x4f, 0xba], 0.12),
    ("TypeScript", [0x31, 0x78, 0xc6], 0.12),
];
const BEHAVIOR_SAMPLES: [(&str, Behavior); 5] = [
    ("batch", Behavior::Batch),
    ("streaming", Behavior::Streaming),
    ("ingest", Behavior::Ingest),
    ("idle", Behavior::Idle),
    ("paused", Behavior::Paused),
];
const GROWTH_SAMPLES: [(&str, u64); 5] = [
    ("first-commits", 40),
    ("staging", STAGING_LAYER_CONTRIBUTIONS),
    ("intermediate", INTERMEDIATE_LAYER_CONTRIBUTIONS),
    ("established", 20_000),
    ("full", MAX_GROWTH_CONTRIBUTIONS),
];
/// A recent streak's length in weeks, and the days since it ended; `None` means it is
/// still running.
type RecentSample = (usize, Option<u64>);
/// Streak samples: name, longest streak in weeks, and the recent streak.
const STREAK_SAMPLES: [(&str, usize, Option<RecentSample>); 5] = [
    ("no-streak", 0, None),
    ("current-4-weeks", 4, Some((4, None))),
    ("current-13-weeks", 13, Some((13, None))),
    ("ended-2-weeks-ago", 13, Some((13, Some(14)))),
    ("record-208-weeks", 208, Some((208, None))),
];
const OBSERVED_ON: &str = "2026-10-31";

fn main() -> ExitCode {
    let Some(output_directory) = env::args_os().nth(1) else {
        eprintln!("Usage: cargo run --example generate-gallery -- DIR");
        return ExitCode::from(2);
    };
    match generate(Path::new(&output_directory)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(generate_error) => {
            eprintln!("generate-gallery: {generate_error}");
            ExitCode::FAILURE
        }
    }
}

fn generate(output_directory: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(output_directory)?;
    let base_development = build_development(8_500, 13, Some((13, None)));
    for (behavior_name, behavior) in BEHAVIOR_SAMPLES {
        let flow_frames: Vec<Frame> = (0..frame_count(behavior))
            .map(|frame_index| render_flow_frame(&base_development, behavior, frame_index))
            .collect();
        fs::write(
            output_directory.join(format!("behavior-{behavior_name}.gif")),
            encode_gif(&flow_frames)?,
        )?;
    }
    for (growth_name, total_contributions) in GROWTH_SAMPLES {
        let development = build_development(total_contributions, 13, Some((13, None)));
        fs::write(
            output_directory.join(format!("growth-{growth_name}.png")),
            encode_png(&render_still(&development))?,
        )?;
    }
    for (streak_name, longest_weeks, recent_streak) in STREAK_SAMPLES {
        let development = build_development(8_500, longest_weeks, recent_streak);
        fs::write(
            output_directory.join(format!("streak-{streak_name}.png")),
            encode_png(&render_still(&development))?,
        )?;
    }
    Ok(())
}

fn build_development(
    total_contributions: u64,
    longest_weeks: usize,
    recent_streak: Option<RecentSample>,
) -> PipelineDevelopment {
    let observed_on: NaiveDate = OBSERVED_ON.parse().expect("the sample date parses");
    let recent = recent_streak.map(|(streak_weeks, days_since_end)| match days_since_end {
        None => RecentStreak::Current(build_weeks(observed_on, streak_weeks)),
        Some(days_since_end) => RecentStreak::Ended {
            weeks: build_weeks(observed_on, streak_weeks),
            ended_on: observed_on - Days::new(days_since_end),
        },
    });
    PipelineDevelopment {
        identity_seed: 0x4d61_726b,
        language_shares: LANGUAGE_MIX
            .iter()
            .map(|&(language_name, _, share)| (language_name.to_owned(), share))
            .collect(),
        language_colors: LANGUAGE_MIX
            .iter()
            .map(|&(language_name, channels, _)| {
                (language_name.to_owned(), Some(LanguageColor(channels)))
            })
            .collect::<BTreeMap<_, _>>(),
        total_contributions,
        observed_on,
        streaks: StreakDevelopment {
            longest: build_weeks(observed_on, longest_weeks),
            recent,
        },
        recent_activity: RecentActivity {
            quiet_day_count: 0,
            recent_contributions: 20,
            complete_week_contributions: vec![20; 52],
            observation_week_contributions: vec![3; 6],
        },
    }
}

/// Fully active weeks ending in the observation week; badges depend only on how many.
fn build_weeks(observed_on: NaiveDate, week_count: usize) -> Vec<ContributionWeek> {
    (0..week_count)
        .rev()
        .map(|weeks_back| ContributionWeek {
            iso_week: (observed_on - Days::new(7 * weeks_back as u64)).iso_week(),
            active_day_count: 7,
            observed_day_count: 7,
        })
        .collect()
}
