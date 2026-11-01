//! Draw a living pixel-art dbt lineage graph from a GitHub account's public activity.
//!
//! The pipeline runs one way, and each module owns one stage of it:
//!
//! 1. [`collect`] gathers an [`history::AccountSnapshot`] through the GitHub CLI.
//! 2. [`history`] reads and writes snapshots and derives language shares and weekly
//!    participation from them.
//! 3. [`development`] turns a snapshot into a [`development::PipelineDevelopment`]:
//!    identity, language colors, growth, streaks, and recent activity.
//! 4. [`layout`] places the graph's sources, models, and pipes from that development.
//! 5. [`behavior`] chooses the day's [`behavior::Behavior`] from recent activity, and
//!    [`flow`] gives the packets for each frame of the behavior's loop.
//! 6. [`badges`] measures how strongly the streaks show, and [`palette`] holds the
//!    theme and colors the sources from their languages.
//! 7. [`render`] draws a [`render::Frame`], still or running, and [`output`] encodes it
//!    as PNG, SVG, or a looping GIF, with a preview page.
//!
//! Everything after collection is deterministic: it reads no clock, performs no I/O, and
//! mutates no input. Callers own files and publication. The `git-lineage` command
//! composes these stages; the repository's getting-started guide covers the command and
//! the GitHub Action. Breaking changes to this API bump the major version.
#![warn(missing_docs)]

pub mod badges;
pub mod behavior;
pub mod collect;
pub mod development;
pub mod flow;
pub mod history;
pub mod layout;
pub mod output;
pub mod palette;
pub mod render;
