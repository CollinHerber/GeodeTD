# AGENTS.md

Guidance for AI agents (and humans) working in **GeodeTD** — a Bevy prototype of a
Gem TD-style tower defense (inspired by the Warcraft 3 custom game). Built one gem
each round; towers can't block the route but can bend it; matching duplicates fuse
into higher grades.

## Build & Run

```powershell
cargo run            # debug run
cargo build          # compile only
cargo check          # fast type-check, prefer this while iterating
cargo clippy         # lints (code is written to be clippy-clean)
cargo fmt            # format before committing
```

- **Toolchain is pinned and non-standard.** `rust-toolchain.toml` selects
  `stable-x86_64-pc-windows-gnu`. `.cargo/config.toml` points the linker at a local
  w64devkit GCC at `C:\dev\bevy\.toolchains\w64devkit` and uses `lld`. If a build
  fails with linker/`gcc not found` errors, that toolchain directory is the cause —
  don't "fix" it by switching to MSVC.
- Rust **edition 2024**. Single dependency: `bevy = "0.18"`.
- Platform is Windows; the shell is PowerShell. Use PowerShell syntax in commands.

## Architecture

Everything is a flat module tree under `src/`, wired together in
[main.rs](src/main.rs). There is **no plugin split** — all systems are registered in
one `.add_systems(Update, (...).chain())` call, so **system order is explicit and
deterministic** (the `.chain()`). If you add a system, place it deliberately in that
tuple.

State is held in two `Resource`s rather than Bevy `States`:

- `Game` ([game.rs](src/game.rs)) — the whole game/session state machine:
  `AppScreen` (Home / ModeSelect / HowToPlay / Settings / Playing), `GameMode`
  (Standard / Random), `Phase` (Build → Countdown → Wave), round, lives, coins,
  the five gem `offers`, timers, RNG, and current selection/upgrade. Most systems
  early-return unless `screen == Playing` (and often a specific `Phase`).
- `Board` ([board.rs](src/board.rs)) — the grid world: `towers` map
  (`GridPos -> Entity`), the current `path` (`Vec<GridPos>`), and `checkpoints`.
  Owns **all pathfinding**.

Module map:

| File | Responsibility |
|------|----------------|
| [main.rs](src/main.rs) | App setup, window, system registration & order |
| [constants.rs](src/constants.rs) | Grid dims (29×17), cell size (40), offer count, countdown |
| [grid.rs](src/grid.rs) | `GridPos`, start/finish, grid↔world coordinate conversion |
| [board.rs](src/board.rs) | `Board` resource, checkpoints, **pathfinding** |
| [game.rs](src/game.rs) | `Game` resource, screens/phases/mode, round flow |
| [components.rs](src/components.rs) | ECS components & marker structs |
| [gem.rs](src/gem.rs) | `GemKind`, `GemGrade`, per-gem stats, grade multipliers |
| [wave.rs](src/wave.rs) | Enemy spawning + **enemy movement along the path** |
| [combat.rs](src/combat.rs) | Tower targeting/attack, shot effects, enemy health visuals |
| [input.rs](src/input.rs) | Mouse/keyboard: select offer, place tower, select/upgrade |
| [ui.rs](src/ui.rs) | All screens, HUD, board tiles, markers, menus (largest file) |
| [rng.rs](src/rng.rs) | Tiny xorshift `OfferRng` (deterministic from a seed) |

## How core systems fit together

- **Coordinates:** the board is centered at world origin with a small Y offset.
  Use `grid_to_world` / `world_to_grid` ([grid.rs](src/grid.rs)) — never hand-roll
  the math. `GridPos` is `{ col, row }` with `(0,0)` at the bottom-left.
- **Pathfinding** lives entirely in [board.rs](src/board.rs).
  `find_complete_path(blocked, checkpoints)` stitches per-segment searches:
  `start → checkpoint[0] → … → finish`. The same function is used both to **route
  enemies** and to **validate tower placement** (a placement is rejected if it would
  leave no complete path). Keep those two uses consistent — change the search once,
  both behaviors update together.
- **Enemy movement** ([wave.rs](src/wave.rs) `move_enemies`): each enemy walks toward
  `path[next_path_index]` in world space, snapping and advancing the index when it
  reaches a waypoint. Movement is already straight-line interpolation between
  waypoints, so the path's shape (orthogonal vs. diagonal) determines how enemies
  look on screen.
- **Tower placement / upgrade** ([input.rs](src/input.rs)): one gem placed per Build
  phase; placement recomputes the path and refreshes path markers. Upgrades work by
  selecting a tower then clicking a matching `(GemKind, GemGrade)` duplicate to
  sacrifice it, advancing the source up the `GRADE_LADDER`.
- **UI is immediate-ish, sprite-based.** Menus/buttons are plain sprites + `Text2d`;
  clicks are hit-tested with `point_in_rect` against stored centers/sizes. Screens
  are tagged with marker components (`HomeScreen`, `GameWorld`, etc.) and torn down
  with `despawn_all`. There is no `bevy_ui` node tree.

## Conventions & gotchas

- Prefer `cargo check` / `cargo clippy` over full runs while iterating — Bevy debug
  builds are slow to link.
- Add new tunable numbers to [constants.rs](src/constants.rs) or the relevant
  `impl` (e.g. gem stats in [gem.rs](src/gem.rs)) rather than scattering literals.
- Systems must guard on `game.screen` / `game.phase` to avoid running on menus.
- `GridPos` derives `Hash`/`Eq` and is used as a `HashMap`/`HashSet` key; keep those
  derives if you extend it.
- RNG is intentionally tiny and deterministic; don't pull in `rand` for gameplay
  randomness — reuse `OfferRng`.
- Commit messages: this repo has no history yet; only commit when asked.
