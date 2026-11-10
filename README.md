# integrateddynamics (Horizon port)

A Horizon port of [integrateddynamics](CyclopsMC/IntegratedDynamics) — MIT.

**Status: first slices.** What is below is what the port actually does today.

## Implemented so far

- A Logic Cable, a **Redstone Reader** part and a **Redstone Writer** part. The
- reader samples the redstone level of the block on its face into the cable
- network's channel; every writer on the connected network emits that level on its
- own face. It is upstream's `redstone_transmission` advancement without the
- variable-card layer.
- `integrateddynamics:cable`** — a non-occluding 4 px core
- (`CableModel.MIN..MAX`, hardness 3.0F), six `part_<face>` enum properties
- (`none|reader|writer`), a `status` property (`inactive|active`), a block

## Not yet ported

- the **variable system**: variable store, logic programmer, materializer,
- proxy, delay, variable cards/transformers — the whole typed value layer;
- every other part (inventory/fluid/energy/entity/machine/world/audio/
- network/effect/panel/connector) and the **facade**/fake-cable placement;
- the **screens** (part settings, writer aspect card slot, display panel);
- energy consumption** of parts and the network's power (upstream's
- `getConsumptionRate`);
- the **menril resource chain** (log/wood/chunk smelting/squeezer/basin) beyond

## Install

Build the bundle with a native-mod capable launcher, or add the file from the
registry once it is listed. Pinned to Horizon `1d63e0a` and horizon-mixins
`5a64af93` for game 26.3.

## Build from source

```sh
cargo test
python3 tools/horizon.py check --horizon <horizon checkout>
python3 tools/horizon.py build --horizon <horizon checkout>
python3 tools/package.py
```

## Credits and licence

Original mod: CyclopsMC/IntegratedDynamics. Port: AdLf0208. Licence: MIT — see [LICENSE](LICENSE);
the original notices are preserved.
