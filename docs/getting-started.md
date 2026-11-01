# Getting Started

Three ways in: a lineage graph on your profile README, the command line, and the
library. Each stands alone.

## On Your Profile README

GitHub shows the README of the repository named after your username, such as
`octocat/octocat`, on your profile page. To give it a lineage graph that redraws
every night:

1. Copy [examples/profile-workflow.yml](../examples/profile-workflow.yml) into
   that repository as `.github/workflows/lineage.yml`.
2. Embed the graph in its `README.md`:

   ```html
   <img src="lineage/lineage.gif" width="768" alt="My data pipeline" />
   ```

   The GIF is drawn at exactly twice the graph's pixel size, so `width="768"`
   shows it at that size without blurring.

3. Run the workflow once from the repository's Actions tab, so the first graph
   appears without waiting for midnight.

The workflow runs just after midnight UTC, so each graph shows the day that just
ended, and again whenever the workflow file changes. Its `git-lineage` step is
this repository's Action. It collects your public activity with the workflow's
built-in token and renders `lineage.gif`, `lineage.png`, `lineage.svg`, and
`preview.html` into `lineage/`. The Action never commits. The workflow's last
step commits `lineage/lineage.gif` only when it changed, so an unchanged graph
adds no commit.

| Input        | Default              | Meaning                                     |
| ------------ | -------------------- | ------------------------------------------- |
| `login`      | The repository owner | Account whose lineage graph to draw         |
| `output-dir` | `lineage`            | Folder that receives the rendered files     |
| `token`      | The built-in token   | Token for reading the account's public data |

If you change `output-dir`, change the commit step's path and the README image
to match. The example uses `MarkPhamm/git-pixel-art@v2`, which moves to each
compatible release; pin a full release tag to choose when you update.

GitHub pauses scheduled workflows in a public repository after 60 days without
activity such as a push. A graph that stops changing during a long quiet stretch
adds no commits, so its schedule can pause and does not resume on its own.
Re-enable it from the Actions tab or with `gh workflow enable lineage.yml`.

## From the Command Line

Install one of two ways. With Rust at or above the `rust-version` in
[Cargo.toml](../Cargo.toml), from a checkout of this repository:

```sh
cargo install --path . --locked
```

Or download the archive for your platform from the
[releases page](https://github.com/MarkPhamm/git-pixel-art/releases), a
`.tar.gz` for Linux and macOS or a `.zip` for Windows. Each archive has a
`.sha256` file beside it; check the download with `shasum -a 256 -c` on macOS,
`sha256sum -c` on Linux, or `certutil -hashfile` on Windows, then put
`git-lineage` on your `PATH`. macOS may refuse to open a binary it downloaded;
`xattr -d com.apple.quarantine git-lineage` clears that flag.

Collection goes through the [GitHub CLI](https://cli.github.com), so install it
and sign in once with `gh auth login`. Then:

```sh
git-lineage collect octocat --output snapshot.json
git-lineage render snapshot.json --output-dir lineage
```

Open `lineage/preview.html` in a browser to see the GIF and the still on
GitHub's light and dark page colors.

`collect` reads only public data: the account's public repositories, the public
repositories it has contributed to, GitHub's color for each language, and its
daily contributions from the account's creation through the last complete UTC
day. It writes the snapshot to standard output, or with `--output` replaces the
file only after collection succeeds. In GitHub Actions, `GH_TOKEN` set to the
workflow's token is enough. `render` draws everything in memory first and
replaces each file through a temporary file, so a failed run keeps the previous
files. Both print errors to standard error and exit 1; invalid usage exits 2.
`git-lineage --help` prints the full usage and `git-lineage --version` the
version.

## As a Library

The same crate is a library. Add it beside `serde_json`, which reads the
snapshot:

```sh
cargo add git-lineage --git https://github.com/MarkPhamm/git-pixel-art
cargo add serde_json
```

The pipeline runs one way. Read or collect an `AccountSnapshot`, derive the
pipeline's development, choose the day's behavior, render frames, and encode
them:

```rust
use git_lineage::behavior::select_behavior;
use git_lineage::development::derive_pipeline_development;
use git_lineage::flow::frame_count;
use git_lineage::history::AccountSnapshot;
use git_lineage::output::{encode_gif, encode_png};
use git_lineage::render::{render_flow_frame, render_still};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let snapshot_json = std::fs::read_to_string("snapshot.json")?;
    let account_snapshot: AccountSnapshot = serde_json::from_str(&snapshot_json)?;
    let development = derive_pipeline_development(&account_snapshot)?;
    let behavior = select_behavior(&development);
    let flow_frames: Vec<_> = (0..frame_count(behavior))
        .map(|frame_index| render_flow_frame(&development, behavior, frame_index))
        .collect();
    std::fs::write("lineage.gif", encode_gif(&flow_frames)?)?;
    std::fs::write("lineage.png", encode_png(&render_still(&development))?)?;
    Ok(())
}
```

To collect in-process instead of reading a file, call
`collect::collect_account_snapshot` with `collect::GhCli` and today's UTC date.
Everything after collection is deterministic and performs no I/O, so the same
snapshot always draws the same graph. Breaking changes to the API bump the major
version.
