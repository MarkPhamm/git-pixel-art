# Changelog

Notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- The lineage graph uses a darker night palette. Language sources keep their hue
  and drop to the brightness of the room, and the canvas, pipes, and badges
  follow.

## [2.0.0] - 2026-11-01

### Changed

- The koi is replaced by a living dbt lineage graph, and the crate, command, and
  Action are renamed `git-lineage`, `git-lineage`, and `git-profile-lineage`.
  Languages become source nodes, contributions grow staging, intermediate, and
  mart layers, and the day's activity sends packets through the pipes: batch,
  streaming, ingest, idle, or paused.
- Streaks show as "tests passed" badges and a green glow on the marts instead of
  koi markings.
- `render` writes `lineage.gif`, `lineage.png`, `lineage.svg`, and
  `preview.html` on a 384 by 192 banner canvas, and no longer takes `--body`.
- GIF frames after the first store only what changed since the frame before,
  which keeps an opaque canvas small.

### Removed

- Koi bodies, coats, markings, and swimming motion, and the koi library example,
  which the lineage gallery replaces.

## [1.0.2] - 2026-10-31

### Fixed

- `git-koi collect` requests again any contribution days GitHub leaves out of an
  answer, instead of failing with "missing contribution date". GitHub left out
  days for some accounts whose history crosses a February 29 (#52).

## [1.0.1] - 2026-10-28

### Changed

- The Action is listed as `git-profile-koi`, since a GitHub user already holds
  the name `git-koi`, and its description fits the Marketplace limit.

## [1.0.0] - 2026-10-28

### Added

- `git-koi collect`, which gathers an account's public snapshot through the
  GitHub CLI, and `--help` and `--version` on the command.
- `git-koi render`, which draws the day's koi as a looping GIF, a still PNG, and
  a preview page, in six bodies.
- A GitHub Action that downloads the release binary and runs both inside a
  profile repository's workflow.
- The `git-koi` library: account history, koi development, behavior, motion,
  markings, rendering, and encoding.

[Unreleased]: https://github.com/MarkPhamm/git-pixel-art/compare/v2.0.0...HEAD
[2.0.0]: https://github.com/MarkPhamm/git-pixel-art/compare/v1.0.2...v2.0.0
[1.0.2]: https://github.com/yalongwastaken/git-koi/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/yalongwastaken/git-koi/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/yalongwastaken/git-koi/releases/tag/v1.0.0
