# GeodeTD

First Bevy prototype for a Gem TD-style tower defense.

## Run

```powershell
cargo run
```

RustRover can open `C:\dev\bevy\GeodeTD` directly and run the `geode_td` binary.

This project uses `stable-x86_64-pc-windows-gnu` through `rust-toolchain.toml` and a local w64devkit toolchain at `C:\dev\bevy\.toolchains\w64devkit`.

## Controls

- The game opens to a home screen. Click `Play`, then choose `Standard` for fixed route points or `Random` for randomized route points.
- Click `How to Play` on the home screen for the current rules summary.
- Press `Esc` during play to toggle the information menu.
- Press `1` through `5` to select one of the offered chipped gems.
- Left-click a grid cell to place the selected gem. You place one offered gem per round.
- The wave starts 3 seconds after placing your gem.
- Left-click an existing tower to open its menu, click `Upgrade`, then click a matching duplicate tower to sacrifice it.
- Placements that block the only path from start to finish are rejected.

## CI/CD

Two GitHub Actions workflows live in `.github/workflows/`:

- **CI** (`ci.yml`) — runs on pull requests and non-`main` pushes: `cargo fmt
  --check`, `cargo clippy -D warnings`, and a release build.
- **Release** (`release.yml`) — runs on push to `main`:
  1. Derives the next semantic version from the commits since the last `v*` tag.
  2. Generates release notes / `CHANGELOG.md` with [git-cliff](https://git-cliff.org/).
  3. Builds a WASM/HTML5 release and bundles it (`web/index.html` + wasm-bindgen glue).
  4. Publishes a GitHub Release with the zipped web build attached.
  5. Deploys the playable build to itch.io with [Butler](https://github.com/itchio/butler)
     to the `html` channel.
  6. Commits the regenerated `CHANGELOG.md` back to `main` (`[skip ci]`).

> Both workflows delete the committed `rust-toolchain.toml` and `.cargo/config.toml`
> **on the runner only** — those pin a local Windows w64devkit toolchain that does
> not exist on CI. Your local checkout is unaffected.

### Commit conventions

Versioning and the changelog are driven by [Conventional Commits](https://www.conventionalcommits.org/).
Only three types are recognized (anything else is omitted from the changelog and
does not trigger a release):

| Type | Changelog section | Version bump |
|------|-------------------|--------------|
| `feat:` | Features | minor |
| `fix:` | Bug Fixes | patch |
| `balance:` | Balance | patch |

A `!` after the type (e.g. `feat!:`) or a `BREAKING CHANGE` body forces a major
bump. Scopes are optional: `balance(ruby): lower base damage 26 -> 22`.

### Required repository configuration

Set these under **Settings → Secrets and variables → Actions** for itch deployment
(the deploy step is skipped automatically if they are absent):

- Secret `BUTLER_API_KEY` — an itch.io API key (itch.io → Settings → API keys).
- Variable `ITCH_USER` — your itch.io username.
- Variable `ITCH_GAME` — the game's project slug on itch.io.

The build pushes to the itch channel `${ITCH_USER}/${ITCH_GAME}:html`.

## Current Scope

- Enemies path from the start cell through four numbered checkpoints, then to the finish cell.
- Standard mode uses a fixed spoke route that returns through the center several times; Random mode chooses new checkpoints each run.
- Tower placement changes the route using grid pathfinding.
- Each enemy killed awards one coin. Coins are tracked but do not have a use yet.
- Ruby, Sapphire, Topaz, Emerald, Amethyst, and Diamond have different chipped stats.
- Upgrade grades run Chipped, Flawed, Regular, Cut, and Perfect.
