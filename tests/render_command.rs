use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use chrono::Days;
use git_lineage::behavior::{Behavior, select_behavior};
use git_lineage::development::derive_pipeline_development;
use git_lineage::flow::frame_count;
use git_lineage::history::{AccountSnapshot, ContributionDay};
use git_lineage::output::{encode_gif, encode_png, encode_svg};
use git_lineage::render::{Frame, render_flow_frame, render_still};

const HISTORY_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/account-with-history.json"
);
const OUTPUT_FILES: [&str; 4] = ["lineage.gif", "lineage.png", "lineage.svg", "preview.html"];

#[test]
fn render_writes_the_days_loop_stills_and_preview_as_the_renderer_draws_them() {
    // Two quiet weeks make the graph idle, so the loop can't pass as the fallback batch.
    let mut account_snapshot = read_fixture_snapshot();
    let contribution_history = account_snapshot.contribution_history.as_mut().unwrap();
    for day_offset in 1..=14 {
        contribution_history.days.push(ContributionDay {
            date: contribution_history.observed_on + Days::new(day_offset),
            contribution_count: 0,
        });
    }
    contribution_history.observed_on = contribution_history.days.last().unwrap().date;
    let scratch_directory = fresh_directory("render-default");
    let snapshot_path = scratch_directory.join("snapshot.json");
    fs::write(
        &snapshot_path,
        serde_json::to_string(&account_snapshot).unwrap(),
    )
    .unwrap();
    let output_directory = scratch_directory.join("lineage");
    let command_output = run_git_lineage(&[
        "render",
        snapshot_path.to_str().unwrap(),
        "--output-dir",
        output_directory.to_str().unwrap(),
    ]);
    assert_eq!(
        command_output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&command_output.stderr)
    );

    let development = derive_pipeline_development(&account_snapshot).unwrap();
    let behavior = select_behavior(&development);
    assert_eq!(behavior, Behavior::Idle);
    let flow_frames: Vec<Frame> = (0..frame_count(behavior))
        .map(|frame_index| render_flow_frame(&development, behavior, frame_index))
        .collect();
    assert_eq!(
        fs::read(output_directory.join("lineage.gif")).unwrap(),
        encode_gif(&flow_frames).unwrap()
    );
    let still_frame = render_still(&development);
    assert_eq!(
        fs::read(output_directory.join("lineage.png")).unwrap(),
        encode_png(&still_frame).unwrap()
    );
    assert_eq!(
        fs::read_to_string(output_directory.join("lineage.svg")).unwrap(),
        encode_svg(&still_frame)
    );
    let preview_html = fs::read_to_string(output_directory.join("preview.html")).unwrap();
    assert!(preview_html.contains(r#"src="lineage.gif""#));
    assert!(preview_html.contains(r#"src="lineage.png""#));
    assert!(!preview_html.contains("<script"));
    let summary_line = String::from_utf8_lossy(&command_output.stdout);
    let observation_date = development.observed_on.to_string();
    assert!(summary_line.starts_with(&format!("idle for {observation_date}")));
    assert!(preview_html.contains(&observation_date));
}

#[test]
fn a_failed_render_exits_one_and_keeps_the_previous_files() {
    let output_directory = fresh_directory("render-failure");
    for file_name in OUTPUT_FILES {
        fs::write(
            output_directory.join(file_name),
            format!("previous {file_name}"),
        )
        .unwrap();
    }
    let command_output = run_git_lineage(&[
        "render",
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/account.json"),
        "--output-dir",
        output_directory.to_str().unwrap(),
    ]);
    assert_eq!(command_output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&command_output.stderr).starts_with("git-lineage: "));
    for file_name in OUTPUT_FILES {
        assert_eq!(
            fs::read_to_string(output_directory.join(file_name)).unwrap(),
            format!("previous {file_name}")
        );
    }
    assert_eq!(
        fs::read_dir(&output_directory).unwrap().count(),
        OUTPUT_FILES.len()
    );
}

#[test]
fn render_usage_errors_exit_two_and_help_exits_zero() {
    for invalid_arguments in [
        &["render", HISTORY_FIXTURE][..],
        &["render", "--output-dir", "lineage"],
        &[
            "render",
            HISTORY_FIXTURE,
            "--output-dir",
            "lineage",
            "--body",
            "white",
        ],
    ] {
        let command_output = run_git_lineage(invalid_arguments);
        assert_eq!(
            command_output.status.code(),
            Some(2),
            "{invalid_arguments:?}"
        );
        assert!(String::from_utf8_lossy(&command_output.stderr).starts_with("Usage:"));
    }
    let help_output = run_git_lineage(&["render", "--help"]);
    assert_eq!(help_output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&help_output.stdout).contains("lineage.gif"));
}

#[test]
fn version_prints_the_manifest_version_and_exits_zero() {
    let version_output = run_git_lineage(&["--version"]);
    assert_eq!(version_output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&version_output.stdout),
        format!("git-lineage {}\n", env!("CARGO_PKG_VERSION"))
    );
}

fn read_fixture_snapshot() -> AccountSnapshot {
    serde_json::from_str(&fs::read_to_string(HISTORY_FIXTURE).unwrap()).unwrap()
}

/// An empty directory of the given name under Cargo's scratch space for tests.
fn fresh_directory(directory_name: &str) -> PathBuf {
    let directory_path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(directory_name);
    let _ = fs::remove_dir_all(&directory_path);
    fs::create_dir_all(&directory_path).unwrap();
    directory_path
}

fn run_git_lineage(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_git-lineage"))
        .args(arguments)
        .output()
        .unwrap()
}
