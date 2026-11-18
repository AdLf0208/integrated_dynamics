# Integrated Dynamics → Horizon (slice 1)

A Horizon port of **Integrated Dynamics 1.38.2** (`CyclopsMC/IntegratedDynamics`,
**MIT**), Minecraft 1.21.1 NeoForge, from the exact ATM10 pack jar.

**Slice 1 is the redstone network:** a **Logic Cable** with a **Redstone Reader**
and a **Redstone Writer** part. The reader samples the redstone level of the
block on its face into the cable network's channel, and every writer on the
connected network emits that level on its own face. It is the mod's
`redstone_transmission` advancement without the variable-card layer.

What is not here: the variable store, logic programmer, materializer, proxy,
delay, every other part, the machines, the menril resource chain beyond the
crystallized chunk the cable recipe needs, screens, energy consumption and the
multipart cable renderer. See `notes.md` for the full slice boundary and gaps.

## The package

One flat `.hmod` (Horizon mod API 0.2), content and code together:

```text
integrateddynamics.hmod
  horizon-mod.toml   the manifest and the only identity (0.2.0)
  Cargo.toml         horizon-mod-api from .horizon/sdk/ (fetched, never shipped)
  src/lib.rs         the one #[mod_api::horizon_mod] impl Hooks + the cable's Parts
  src/runtime.rs     the pure cable rules (byte-identical to the 0.1.0 file)
  src/support.rs     the glue: network recompute, block entity value, commands
  assets/integrateddynamics/   models, textures, lang
  data/integrateddynamics/     block/item definitions, recipe, loot, tag
  LICENSE, LICENSE-NOTICE.md
```

The pre-migration `.hnmod` and its `mixins/` tree are kept, not shipped, under
`ports/_shared/evidence/integrateddynamics/retired-hnmod-0.1.0/`.

## Building and checking

```sh
BUILD_LOCAL=1 ports/tools/build-slot.sh run --job sdk-id --port ports/integrateddynamics \
    --step test -- env -C ports/integrateddynamics CARGO_INCREMENTAL=0 cargo test
rm -f ports/integrateddynamics/Cargo.lock
BUILD_LOCAL=1 ports/tools/build-slot.sh run --job sdk-id --port ports/integrateddynamics \
    --step build -- env -C ports/integrateddynamics CARGO_INCREMENTAL=0 cargo horizon-mod build
cp ports/integrateddynamics/target/horizon-mod/integrateddynamics.hmod ports/integrateddynamics/
cargo horizon-mod check ports/integrateddynamics/integrateddynamics.hmod
cargo horizon-mod fits ports/integrateddynamics/integrateddynamics.hmod --build nightly
```

The in-game scenario (`tools/e2e.sh`) needs a **launcher-built** game:
`horizon-launcher native-build ports/integrateddynamics --instance <i> --trust-native-code`,
then `GAME=<instance horizon-client> CTL=… tools/e2e.sh`.

The jar is evidence at
`ports/_shared/carry-work/integrateddynamics/integrateddynamics-1.21.1-neoforge-1.38.2.jar`
(SHA-1 `1a77cf34…c600`, the pack record's hash).

**Local-only** until the maintainer approves publishing (ARCHITECTURE.md §8).
