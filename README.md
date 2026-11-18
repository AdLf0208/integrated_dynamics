# Integrated Dynamics — a Horizon port

**Integrated Dynamics** is a Minecraft mod. A Minecraft mod to take full and automated control of your appliances. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [CyclopsMC/IntegratedDynamics](https://github.com/CyclopsMC/IntegratedDynamics) — licence **MIT**.

| | |
|---|---|
| Upstream mod | Integrated Dynamics ([source](https://github.com/CyclopsMC/IntegratedDynamics)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `integrateddynamics.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

- **`integrateddynamics:cable`** — a non-occluding 4 px core
- **Parts** — `integrateddynamics:part_redstone_reader` and
- **Reader** — `LevelRef::signal(pos, direction)` (the pin's per-face
- **Writer** — the cable's `block_signal` part answers the channel on the
- **Network** — the connected cable set (6-way adjacency, bounded to 1,024
- **Cadence** — a cable's block-entity tick only **queues** it; the network is
- **Harness readout** — `/integrateddynamics info <x> <y> <z>` prints the six
- **A block-scoped moment is a Part.** `block_signal`, `is_signal_source`,

## Not yet ported

- the **variable system**: variable store, logic programmer, materializer,
- every other part (inventory/fluid/energy/entity/machine/world/audio/
- the **screens** (part settings, writer aspect card slot, display panel);
- **energy consumption** of parts and the network's power (upstream's
- the **menril resource chain** (log/wood/chunk smelting/squeezer/basin) beyond
- the **machines** (coal generator, energy battery, squeezer/drying basin);
- the multipart cable **renderer** (arms, facades, lit tint, part animations);
- per-face channels (upstream one variable per reader) — this slice is one

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `integrateddynamics.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Integrated Dynamics was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/CyclopsMC/IntegratedDynamics

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run integrateddynamics` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/integrateddynamics/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
