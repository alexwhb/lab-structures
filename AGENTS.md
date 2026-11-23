# Structures Playground

Structural-engineering playground + truss netlist workbench for macOS, written in Rust ("SPICE
for things that stand up"). The fifth bench in the lab series (Breadbox, Chem Playground, Bio
Playground, Automata Playground, Structures Playground). Playground mode (Bridge · build a truss
and break it, Tower · skyscraper in a storm, Ground · soils and foundations, Materials ·
tensile test) and Advanced mode (the bridge as a truss netlist: stiffness results, influence
line, checks). Drivable over MCP. Educational, not for real structural design.

**Read before working:**

| | |
|---|---|
| [`SPEC.md`](SPEC.md) | the definition of done — every requirement has an ID |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | crate map and contracts between crates |
| [`docs/design/`](docs/design) | the mockup: `mockup/*.dc.html` (spec: markup + CSS + the `class Component` logic), `screenshots/*-1440.png` / `*-390.png` (rendered references), `bundles/*.html` (runnable in Chrome) |
| [`PROGRESS.md`](PROGRESS.md) | who owns what, status, open issues, requests |
| [labkit](https://github.com/alexwhb/labkit) (checked out at `../labkit`) | shared lab-series crates (`lab-theme`, `lab-ui`, `lab-mcp`), a git dependency pinned to a commit in `Cargo.toml` — use them, never edit them from here (see "Working on labkit") |
| [`../automata-playground`](../automata-playground) | the newest sibling and **our template**: app shell, widgets, egui_kittest harness, MCP bridge, packaging scripts. Fork its code freely (same owner, MIT); never edit it |
| [`../breadbox`](../breadbox), [`../chem-playground`](../chem-playground) | older siblings — read for patterns; never edit them |

## Rules

- **Toolchain**: `export PATH="$HOME/.cargo/bin:$PATH"` (Homebrew's older rustc must not be used).
- **Parallel agents share this checkout.** Use your own target dir
  (`export CARGO_TARGET_DIR=target/<your-agent-name>`), build/test with `-p <crate>` for what
  you own, and never edit files owned by another active agent (see `PROGRESS.md`). Need a
  change elsewhere? Write it under "Requests" in `PROGRESS.md`.
- **labkit and the sister apps are other people's live work.** Never edit anything outside this
  repo. If labkit fails to compile, it's probably mid-edit: wait a minute and retry. Anything
  that should move into labkit goes under "Candidates for labkit" in `PROGRESS.md`.
- **Working on labkit**: this repo is public (github.com/alexwhb/lab-structures) and builds
  labkit from GitHub at the pinned `rev`. To try it against the local `../labkit` checkout,
  create `.cargo/config.toml` (gitignored) and delete it when done:
  ```toml
  [patch."https://github.com/alexwhb/labkit"]
  lab-theme = { path = "../labkit/crates/lab-theme" }
  lab-ui = { path = "../labkit/crates/lab-ui" }
  lab-mcp = { path = "../labkit/crates/lab-mcp" }
  ```
  To move to a newer labkit: it must be pushed first; bump all three `rev`s together, then run
  `scripts/test-all.sh`.
- **Commits**: commit only your own paths (`git add <paths> && git commit -m "..." -- <paths>`);
  retry if `.git/index.lock` is busy. Small, frequent commits with clear messages. End commit
  messages with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Root `Cargo.toml` / `Cargo.lock`: adding your crate to `members` or a dependency is allowed for
  anyone as a one-line change; re-read the file just before editing.
- Keep `docs/ARCHITECTURE.md` in sync when you change a cross-crate contract (each owner edits
  their own section).
- No panics on user input; libraries use `thiserror`, the app `anyhow`.
- Colours, fonts, radii and sizes come from `sp_theme` (which builds on `lab_theme`) — never
  hard-code a hex in a screen. Screen scene colours go in your screen's module in `sp-theme`
  (`crates/sp-theme/src/<screen>.rs`), each with its mockup CSS source.
- Every feature lands with tests. Real behaviour beats mocks: drive the GUI with `egui_kittest`
  through AccessKit labels, run the real engine, spawn the real MCP server.
- **Every number shown comes from the engine** (SPEC DONE-2). No mockup numbers in screens.
- **No sparkle / magic-wand / four-point-star icons, ever** (no ✨ ✦ ✧). Pick an icon that says
  what the feature does.
- `cargo fmt --check` (not `--all`: that also checks `../labkit`), `cargo clippy --workspace
  --all-targets -- -D warnings` and `cargo test --workspace` must stay green
  (`scripts/test-all.sh` once it exists). Tests must not take more than a few seconds each in
  the default (opt-level 2) test profile.
