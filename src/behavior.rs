//! The day's flow through the pipeline, chosen from recent activity by one ordered set
//! of rules.

use crate::development::{PipelineDevelopment, RecentActivity};

/// One flow per run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Behavior {
    /// The whole graph dims and greys; nothing moves but a blinking cursor.
    Paused,
    /// A single packet trickles through the graph.
    Idle,
    /// A burst of packets enters from pulsing sources.
    Ingest,
    /// Dense, fast packets on every pipe, with the marts flashing.
    Streaming,
    /// A steady wave of packets moves layer by layer; the everyday run.
    Batch,
}

const PAUSED_QUIET_DAYS: u32 = 60;
const IDLE_QUIET_DAYS: u32 = 14;
/// Totals whose crossing earns a week of streaming.
const STREAMING_MILESTONES: [u64; 6] = [1_000, 5_000, 10_000, 25_000, 50_000, 100_000];
/// A busy week is compared with at most this many of the latest complete weeks.
const STREAMING_COMPARED_WEEKS: usize = 52;
/// A top tenth means little with fewer complete weeks than this.
const STREAMING_MIN_COMPLETE_WEEKS: usize = 10;
/// A busy week is matched or beaten by at most one in this many compared weeks.
const STREAMING_WEEK_SHARE_DENOMINATOR: usize = 10;
const DAYS_PER_WEEK: usize = 7;

/// Choose the flow for the day after the observation date. A nightly run judges the
/// day that just ended, and the pipeline shows the result all of the next day. The
/// first matching rule wins:
///
/// | Behavior | When the observation date |
/// | --- | --- |
/// | Paused | ends PAUSED_QUIET_DAYS or more quiet days |
/// | Idle | ends IDLE_QUIET_DAYS or more quiet days |
/// | Ingest | is the first day with contributions in its Monday-Sunday week |
/// | Streaming | falls within seven days of the total crossing a milestone |
/// | Streaming | is a Sunday that finishes a busy week |
/// | Batch | is anything else |
///
/// A finished week is busy when at most one in ten of the latest
/// STREAMING_COMPARED_WEEKS complete weeks before it match or beat its total, and at
/// least STREAMING_MIN_COMPLETE_WEEKS weeks are compared. So a steady account runs
/// batches, and the pipeline streams at most one day a week beyond milestones. This
/// reads no clock and changes no growth, colors, or badges.
pub fn select_behavior(development: &PipelineDevelopment) -> Behavior {
    let recent_activity = &development.recent_activity;
    if recent_activity.quiet_day_count >= PAUSED_QUIET_DAYS {
        Behavior::Paused
    } else if recent_activity.quiet_day_count >= IDLE_QUIET_DAYS {
        Behavior::Idle
    } else if is_first_contribution_of_week(recent_activity) {
        Behavior::Ingest
    } else if has_crossed_milestone(development.total_contributions, recent_activity)
        || is_finished_busy_week(recent_activity)
    {
        Behavior::Streaming
    } else {
        Behavior::Batch
    }
}

fn is_first_contribution_of_week(recent_activity: &RecentActivity) -> bool {
    matches!(
        recent_activity.observation_week_contributions.split_last(),
        Some((&observed_day, earlier_days))
            if observed_day > 0 && earlier_days.iter().all(|&day_contributions| day_contributions == 0)
    )
}

fn has_crossed_milestone(total_contributions: u64, recent_activity: &RecentActivity) -> bool {
    let earlier_total = total_contributions.saturating_sub(recent_activity.recent_contributions);
    STREAMING_MILESTONES
        .iter()
        .any(|&milestone| earlier_total < milestone && milestone <= total_contributions)
}

fn is_finished_busy_week(recent_activity: &RecentActivity) -> bool {
    let finished_week = &recent_activity.observation_week_contributions;
    let complete_weeks = &recent_activity.complete_week_contributions;
    let compared_weeks = &complete_weeks[complete_weeks
        .len()
        .saturating_sub(STREAMING_COMPARED_WEEKS)..];
    let week_total: u64 = finished_week.iter().sum();
    let matching_week_count = compared_weeks
        .iter()
        .filter(|&&week_contributions| week_contributions >= week_total)
        .count();
    finished_week.len() == DAYS_PER_WEEK
        && compared_weeks.len() >= STREAMING_MIN_COMPLETE_WEEKS
        && matching_week_count * STREAMING_WEEK_SHARE_DENOMINATOR <= compared_weeks.len()
}
