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

## Current Scope

- Enemies path from the start cell through four numbered checkpoints, then to the finish cell.
- Standard mode uses a fixed spoke route that returns through the center several times; Random mode chooses new checkpoints each run.
- Tower placement changes the route using grid pathfinding.
- Each enemy killed awards one coin. Coins are tracked but do not have a use yet.
- Ruby, Sapphire, Topaz, Emerald, Amethyst, and Diamond have different chipped stats.
- Upgrade grades run Chipped, Flawed, Regular, Cut, and Perfect.
