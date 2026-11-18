# Integrated Dynamics → Horizon (slice 1)

Port of **Integrated Dynamics 1.38.2** (`CyclopsMC/IntegratedDynamics`, **MIT**
per the jar's `META-INF/neoforge.mods.toml`), Minecraft 1.21.1 NeoForge, from
the exact ATM10 jar: `integrateddynamics-1.21.1-neoforge-1.38.2.jar`,
**4,501,170 B**, SHA-1 `1a77cf34678dcdb3a11ab9695af6ce588e6fc600` (the pack
record's `modrinthHash`), SHA-256
`7dee30453f1e8972962108175cf46c09c7531ab273279888bf2162198d241198`.
Provenance, the jar inventory and the upstream behaviour:
[`FINDINGS.md`](FINDINGS.md). The jar is kept at
`ports/_shared/carry-work/integrateddynamics/`.

**Package version 0.2.0** — the SDK repackage of 0.1.0, which was submitted as
PR **#852** and closed in Horizon's format purge. `src/runtime.rs` is
**byte-identical** to the pre-migration `mixins/integrateddynamics/runtime.rs`
(sha256 `91de3aa07721f843a962d1f1c3271dc59d79cccd7437899b3291302fd4bdd692`,
`cmp` clean, and the same bytes inside the built package). The glue, the
manifest and the packaging are new; [the migration section](#migration-to-the-sdk-format)
is the recipe.

**Class + tier:** `content` / **`deep`** (ledger row; provenance
`ports/_shared/source-map/atm10-sources.csv`). Approach per Horizon's
`docs/modpacks.md`: a deep mod is entities/renderers/dimensions/new hooks; this
slice takes the mod's *network* shape — a cable block entity holding parts —
which is the smallest honest deep behaviour. `precheck-mod.py
integrateddynamics` reports `content · deep · core · P0`, licence `MIT` rank 0.

**Licence:** MIT → **rank 0** (permissive). Local-only until the maintainer
approves publishing (ARCHITECTURE.md §8); the package carries the Cyclops
copyright and the MIT text (`LICENSE`, `LICENSE-NOTICE.md` are inside the
`.hmod`).

**Why now:** licence-first queue; the commit's `deep`/core mechanics row.
More important for the queue: `integrateddynamics` is the **parent of the
`integrated*` family** (`integratedtunnels`, `integratedcrafting`,
`integratedterminals`, `integratedscripting`), so this migration fixes the
shape its children copy — see [the family section](#what-is-family-wide-glue-and-what-is-this-mod-only).
`cyclopscore` (the library) is **mapped, not required**; this slice needs no
library API.

## Slice 1: the redstone network

A Logic Cable, a **Redstone Reader** part and a **Redstone Writer** part. The
reader samples the redstone level of the block on its face into the cable
network's channel; every writer on the connected network emits that level on its
own face. It is upstream's `redstone_transmission` advancement without the
variable-card layer.

### What the slice does

- **`integrateddynamics:cable`** — a non-occluding 4 px core
  (`CableModel.MIN..MAX`, hardness 3.0F), six `part_<face>` enum properties
  (`none|reader|writer`), a `status` property (`inactive|active`), a block
  entity holding the network channel value (`value`, 0..15), and a self-drop
  loot table. Upstream's `waterlogged`/`ticking` states and its multipart
  arms/facades are not carried.
- **Parts** — `integrateddynamics:part_redstone_reader` and
  `..._redstone_writer` **items**, placed on a clicked cable face by the cable's
  `use_item_on` (upstream's `ItemPart.useOn`), consumed outside creative; parts
  live in the cable's state (and its persisted block entity) as upstream's
  multipart data lives in the cable. A face that already holds a part is refused
  (the unported screen).
- **Reader** — `LevelRef::signal(pos, direction)` (the pin's per-face
  `SignalGetter.getSignal`) reads the block on the reader's own face, exactly
  as upstream's `getSignal(targetPos, targetSide)` does; the strongest reading
  on the network is the channel.
- **Writer** — the cable's `block_signal` part answers the channel on the
  writer's face as a **weak** signal (`direct = false`, upstream's aspect
  default), and `is_signal_source` makes the cable a source so wire beside it
  connects.
- **Network** — the connected cable set (6-way adjacency, bounded to 1,024
  cables); the channel is the **maximum** reader reading on the network, and
  every cable in the network stores it. A value change (or a live writer)
  calls `LevelRef::update_neighbors(.., Neighbours)` so the lamp re-reads.
- **Cadence** — a cable's block-entity tick only **queues** it; the network is
  recomputed at `level_tick_post`, where a block entity is back in the world and
  its saved compound can be written. Cables with no reader or writer still
  re-evaluate once per tick in this slice.
- **Harness readout** — `/integrateddynamics info <x> <y> <z>` prints the six
  parts, the channel value and the status; `/integrateddynamics part <x> <y>
  <z> <face> <none|reader|writer>` places/removes a part (the path a click
  cannot reach); `/integrateddynamics probe <x> <y> <z>` prints the strongest
  signal reaching a position.

### Deliberate deviations

| Upstream | Here | Why |
|---|---|---|
| ~~Reader reads the target *face*~~ **— withdrawn 2026-11-18** | `LevelRef::signal(pos, direction)` **is** the target-face read | the SDK has the per-face read the `.hnmod` pin lacked; the 0.1.0 deviation ("strongest of the six neighbours") is gone and the reader is upstream-faithful |
| A writer writes the active aspect's variable-card value | the writer writes the network channel: **max** reader level on the connected cables | one integer channel per network stands in for the variable system; the card slot and aspect selection are out of slice |
| Parts require a variable card and an aspect, and a writer is active only with one | always on once placed; `status=active` when the channel is > 0 | no card/inventory UI in this slice |
| Cable network id is maintained incrementally on add/remove | connected set recomputed each level tick | safe and simple at a handful of cables; a per-network dirty cache is future work |
| Cable draws multipart core + arms + part models from its block entity | content multipart: 4 px core + the carried part models per face/status | content blocks cannot draw a block-entity model |
| Parts can sit on a *fake cable* in an otherwise empty block | not carried: a part is only placed on a real cable | the fake-cable network is a separate feature |
| Cable and parts drop themselves (code path) | self-drop loot tables, 26.3 singular `condition` | the jar has no loot tables for them |
| Cable recipe: 3 per craft | carried with a `crystalized_menril_chunk` item from the jar; part recipes **not** carried | the part recipes need `variable_transformer_input/output`, which belong to the variable system |
| A writer emits strong power when its aspect says so | weak only (`direct = false`) | the aspect layer is out of slice; a `direct` face answers 0 as upstream's default does |

### Library APIs the slice shrank around (`cyclopscore`, mapped)

`cyclopscore` is a Horizon *mapped* library (not ported). This slice needed none
of it; what it would have provided and how the slice avoids it:

| `cyclopscore` API | What it gave upstream | Here |
|---|---|---|
| `PartHelpers.addPart` / `IPartContainer` | the cable's six-slot part container | the cable's six `part_<face>` state properties + block entity |
| `PartTarget` / `PartPos` | centre + target pos/side pairs | the face property and `face_normal` in the pure runtime |
| `EnumFacingMap`, `DimPos` | face- and dimension-keyed maps | plain arrays and `[i32; 3]` positions |
| `INetwork` / `NetworkHelpers` | the incremental network graph | `collect_network` over the connected cable set |
| `DynamicRedstone` capability | writer output through a block capability | `block_signal` + `is_signal_source` on the cable Part |
| `Helpers`, `BlockEntityHelpers` | misc world/level helpers | the SDK's `LevelRef`/`Host` helpers |
| `IValuedCyclicItemWrapper` (config) | the config file | none: the slice has no config, upstream defaults are baked |

### Gaps (next slices)

1. the **variable system**: variable store, logic programmer, materializer,
   proxy, delay, variable cards/transformers — the whole typed value layer;
2. every other part (inventory/fluid/energy/entity/machine/world/audio/
   network/effect/panel/connector) and the **facade**/fake-cable placement;
3. the **screens** (part settings, writer aspect card slot, display panel);
4. **energy consumption** of parts and the network's power (upstream's
   `getConsumptionRate`);
5. the **menril resource chain** (log/wood/chunk smelting/squeezer/basin) beyond
   the crystallized chunk the cable recipe needs;
6. the **machines** (coal generator, energy battery, squeezer/drying basin);
7. the multipart cable **renderer** (arms, facades, lit tint, part animations);
8. per-face channels (upstream one variable per reader) — this slice is one
   channel per network, the maximum reading.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,systematic-debugging,test-driven-development,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

## Migration to the SDK format

*Written 2026-11-18 for this port, the third large one after `imfast` (and the
`twilightforest` reference). Pins: nightly
**`0b14f8acfd97fa2c750e42795249dcc981178ca4`**, mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, launcher 0.2.78,
`cargo-horizon-mod` from `horizon-rust/horizon@main`.*

**Why.** Horizon's mod-API merge (#16941, launcher ≥ 0.2.70, PR #17992) retired
`.hnmod`; the registry closed every `.hnmod` submission on 2026-11-18. The new
unit is **one flat `.hmod`**: content, code, or both, in one package. A port
with carried content needs **no embedded-content trick any more** — the AE2
`content_root` pattern is dead with the container that needed it.

### 0. Read the surface before writing glue

```sh
cargo horizon-mod new /tmp/opencode/<id>-new --template native   # fetches .horizon/sdk/
cargo horizon-mod hooks <text>                                   # search the catalog offline
```

The declaration the generator reads is
`.horizon/sdk/api/crates/horizon-mod-vocab/src/hooks/<name>.rs`; the **generated**
`Hooks`/`BlockPart`/`BlockEntityPart` traits, every event struct and every
dispatch function are in
`.horizon/sdk/api/crates/horizon-mod-vocab/generated/mod_api_generated.rs`. Host
methods are one file per area in `.../src/host/`, and the **handle** methods
(what a mod actually calls for levels and block entities) are in
`generated/mod_api_generated_host.rs`. Read those three, do not guess.

### 1. Files created

| Path | What it is |
|---|---|
| `horizon-mod.toml` | the only manifest and the only identity |
| `Cargo.toml` | one crate; its only dependency is `horizon-mod-api` from `.horizon/sdk/` |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks`, the cable's `BlockPart` + `BlockEntityPart`, and `pub mod runtime; mod support;` |
| `src/runtime.rs` | the port's pure logic, **copied byte-for-byte** (`cp`, then `cmp`, then the packaged copy compared too) |
| `src/support.rs` | the glue: network recompute, block-entity value, the part placement, the commands, and 6 tests of the state parser |
| `assets/integrateddynamics/`, `data/integrateddynamics/`, `data/c/` | the carried content, at the package root (moved from `content/`) |
| `LICENSE`, `LICENSE-NOTICE.md` | the licence travels inside the package |
| `.gitignore` | `/target`, `/.horizon`, `/Cargo.lock` |

### 2. Files deleted (and where the evidence went)

| Deleted | Why | Kept as evidence |
|---|---|---|
| `mixins/integrateddynamics/` (`horizon-mixin.toml`, `runtime.rs`, `support.rs`, one helper per hook, `data/`) | the container is retired; `runtime.rs` moved to `src/` | `ports/_shared/evidence/integrateddynamics/retired-hnmod-0.1.0/` |
| `integrateddynamics.hnmod` (25,152 B, `b9fb347e…`) | unloadable on launcher ≥ 0.2.70 | same folder + `SHA256SUMS` |
| `content/` (its `.hmod` 13,865 B `dc8f63ec…`, manifest) | one package now; the content moved to `assets/` + `data/` | same folder |
| `compatibility.json`, `rust-toolchain.toml` | `[native]` in the manifest is the pin; the launcher decides the toolchain | same folder |
| `tools/horizon.py`, `package.py`, `package.sh`, `assemble_content.py`, `audit/`, `test_template.py`, `test_stage.py`, `probe-embedded-load.sh`, `hmod-load.sh` | staging, `.hnmod` packaging and the embedded-content probe are gone; the content is checked by `cargo horizon-mod check` on the package | same folder |
| `.github/workflows/ci.yml` | the template's scaffold: it checked out `horizon-mixins` at `compatibility.json`'s revision and ran `tools/test_template.py`, neither of which exists any more — a red badge for a retired format | not kept (the SDK ladder is in §6 and the verification table) |
| `Cargo.lock` | packed otherwise (trap 5); no registry dependency of this mod's own | not kept |

**Kept:** `tools/e2e.sh` (its assertions are format-independent; it needs
`GAME=`), `tools/notice.py`, `notes.md`, `FINDINGS.md`, `VALIDATION.md`,
`README.md`.

### 3. Manifest mapping

| Old | New |
|---|---|
| `horizon-mixin.toml` `[mod] id` | `horizon-mod.toml` `id` (also the resource namespace) |
| `[mod] version = "0.1.0"` | `version = "0.2.0"` — a **new** version for the resubmission |
| `[mod] priority = 100` | gone; ordering is `#[listener(priority = …)]` |
| `[compatibility] horizon_revision = "1d63e0a5…"` | `[native] horizon = "0b14f8acfd97fa2c750e42795249dcc981178ca4"` |
| bundle's mixins revision `5a64af93…` | `[native] mixins = "e59da9094f0c2fdda6c627fe9eff22dec6a906f0"` — **it moved too** |
| — | `[native] game = "26.3"` |
| `[[module]]` runtime/support | **no plan section**: `src/runtime.rs` and `src/support.rs` are ordinary modules |
| 10 `[[injection]]`s + helper files + `bindings` | `Hooks`/`BlockPart`/`BlockEntityPart` methods (table below) |
| `horizon-native.json` | gone (the manifest is the only one) |
| content `horizon-mod.toml` inside `content/` | gone; one manifest, one package |
| — | `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]`, `license`, `homepage`, `issues`, `authors` |

`side = "both"` is the **installation** side (an integrated server belongs to the
client installation); `capabilities = ["registries"]` is the one value the
schema allows even for a mod that registers little.

### 4. Helper → hook table (every old helper, mapped)

| Old (`.hnmod`) | New (SDK) | Kept or gap |
|---|---|---|
| `block_entity.rs` — `native_block_entity` HEAD, `(id, Compound)` | `BlockEntityPart::block_entity(&self, LevelSide, &mut BlockEntityEvent) -> Option<Compound>` on the cable Part | **kept** (replacement contract; the declaration owns the type id) |
| `block_entity_ticks.rs` — `native_block_entity_ticks` | `BlockEntityPart::block_entity_ticks(&self, &dyn Host, &mut BlockEntityTicksEvent)` → `event.ticks = true` | **kept** |
| `block_entity_tick.rs` — `native_block_entity_tick` | `BlockEntityPart::block_entity_tick(&self, &mut dyn ServerHost, &mut BlockEntityTickEvent)` → queue `(host.dimension(), event.pos)` | **kept** (the event's `entity` view is unused — the writes happen at the level tick) |
| `level_tick.rs` — `native_level_tick_post` | `Hooks::level_tick_post(&self, HostSide, &mut LevelTickPostEvent)`; `event.level` is a `LevelRef` | **kept** |
| — (new) | `Hooks::level_unload(&self, &mut ServerHost, &mut LevelUnloadEvent)` → drop that level's queued ticks and channel mirror | **added** (the mirror is new state; this is its cleanup) |
| `block_signal.rs` — `native_block_signal` | `BlockPart::block_signal(&self, &dyn LevelHost, &mut BlockSignalEvent)`; `event.dimension/pos/state/face/direct/signal` | **kept** — and now richer: the event carries the dimension, so no string lookup |
| `is_signal_source.rs` — `native_is_signal_source` | `BlockPart::is_signal_source(&self, &dyn LevelHost, &mut IsSignalSourceEvent)` | **kept** |
| `right_click.rs` — `native_right_click_block` (global, `canceled`) | `BlockPart::use_item_on(&self, &mut ServerHost, &mut UseItemOnEvent) -> Option<InteractionResult>` (`event.click: &BlockClick`) | **kept, and scoped**: the moment now belongs to the cable block, and vanilla's own `useOn` ordering applies |
| `register_commands.rs` — `NativeCommand` strings | `Hooks::register_commands(&self, &mut ServerHost, &mut RegisterCommandsEvent)` pushing `mod_api::Command::new("… <pos:block_pos>")` | **kept** |
| `perform_command.rs` — `(player, command, ran)`, string split | `Hooks::run_mod_command(&self, &mut ServerHost, &mut RunModCommandEvent) -> Flow` with **typed** `CommandInvocation` (`literals`, `argument("pos") = BlockPos`) and `host.commands().feedback(Text)` | **kept, typed** |
| `support.rs` — the glue module | `src/support.rs` (same role, same split: state, network, commands) | **kept** |
| `runtime.rs` — pure rules | `src/runtime.rs`, **byte-identical** | **kept** |
| `embedded_content.rs` — `content_root` HEAD, wrote the built `.hmod` to `.horizon-cache/` | — | **gap closed, not needed**: content ships in the same package's `assets/` + `data/` |
| `game.native_neighbor_signal(dimension,pos)` (strongest of six) | `LevelRef::signal(host, pos, direction)` — the per-face read | **improved**: the 0.1.0 deviation is withdrawn |
| `game.native_block_property(pos, key)` | `LevelRef::block_property(host, pos, key)` | kept |
| `game.native_block(pos) == CABLE` | `LevelRef::block(host, pos)` then `BlockKey::id(host) == CABLE` | kept (a `String` per neighbour; the walk is ≤ 1024) |
| `native_set_block_property_with(pos, …, UPDATE_CLIENTS)` | `ServerHost::set_block_property(pos, key, value)` | kept (the no-flags form is a no-op when unchanged) |
| `native_update_signal_neighbors(dimension,pos)` | `LevelRef::update_neighbors(host, pos, source_state, NeighbourUpdate::Neighbours)` | kept |
| `native_take_items(&held, 1)` | `ItemPlace::PlayerHand { player, hand }.extract(host, tx, &ItemResource::of(stack), 1)` inside `host.transaction(..)` | kept (creative hands give without losing, as before) |
| `native_feedback` / `native_acting_player().game_mode` | `PlayerRef::tell(host, Text::literal(..))` / the engine's own extract rules | kept |
| `native_register_commands` at API 8 floors | none: the SDK is the floor (`[native]` pins it) | — |

### 5. What the new API changed in the behaviour

- **A block-scoped moment is a Part.** `block_signal`, `is_signal_source`,
  `use_item_on`, `block_entity*` have **no `Hooks` method** — they are
  `BlockPart`/`BlockEntityPart` methods, and the Part must be attached in
  `Hooks::register` (`block_key` → `attach_block_part`; `declare_block_entity_type`
  → `attach_block_entity_part`). This is why a naive port that writes
  `fn block_signal` on `Hooks` does not compile.
- **The reader is upstream-faithful again.**
- **Commands are typed**, so the harness no longer splits a string; the three
  usages are declared once and `argument("pos")` is a `[i32; 3]`.
- **Placement is `use_item_on`**, so the click arrives with the hand, the stack
  and the face instead of a boolean `canceled`.
- **Hot hooks must not read the world.** `block_signal` is asked "very often,
  from inside the level's update", so the channel value is mirrored in memory
  (written at `level_tick_post`) and the parts come from a memoized
  state-string parse rather than a block-entity read.

### 6. Build and check (the ladder for an SDK package)

`build-slot.sh` runs the command in the **campaign root**, not the port, so
every step names the port explicitly:

```sh
BUILD_LOCAL=1 ports/tools/build-slot.sh run --job sdk-id --port ports/integrateddynamics \
    --step test -- env -C ports/integrateddynamics CARGO_INCREMENTAL=0 cargo test
BUILD_LOCAL=1 ports/tools/build-slot.sh run --job sdk-id --port ports/integrateddynamics \
    --step lint -- env -C ports/integrateddynamics CARGO_INCREMENTAL=0 cargo clippy --all-targets -- -D warnings
BUILD_LOCAL=1 ports/tools/build-slot.sh run --job sdk-id --port ports/integrateddynamics \
    --step fmt -- env -C ports/integrateddynamics cargo fmt --check
rm -f ports/integrateddynamics/Cargo.lock            # trap 5
BUILD_LOCAL=1 ports/tools/build-slot.sh run --job sdk-id --port ports/integrateddynamics \
    --step build -- env -C ports/integrateddynamics CARGO_INCREMENTAL=0 cargo horizon-mod build
cp ports/integrateddynamics/target/horizon-mod/integrateddynamics.hmod ports/integrateddynamics/
BUILD_LOCAL=1 ports/tools/build-slot.sh run --job sdk-id --port ports/integrateddynamics \
    --step check -- env -C ports/integrateddynamics cargo horizon-mod check integrateddynamics.hmod
BUILD_LOCAL=1 ports/tools/build-slot.sh run --job sdk-id --port ports/integrateddynamics \
    --step check -- env -C ports/integrateddynamics cargo horizon-mod fits integrateddynamics.hmod --build nightly
```

`BUILD_LOCAL=1` is not optional here (trap 17). The gate order that makes this a
migration rather than a rewrite: **`cargo test` green with `src/runtime.rs`
byte-identical *before* the glue was written** — the first run of the new crate
compiled `runtime` alone (12 tests, `sdk-test-runtime-only.log`), and only then
were `lib.rs`/`support.rs` filled in.

### 7. Traps (all hit in this migration)

1. **`build-slot.sh` runs in the campaign root.** `cargo test` there answers
   `could not find Cargo.toml`; pass `env -C ports/<id> …` (or `cd` inside a
   `bash -c`). The old ladders' `python3 tools/horizon.py …` worked because
   that tool `chdir`s itself.
2. **The remote runner can report success for a run that never happened.**
   The first attempt was forwarded, its `cd /home/deepseek/atm10/integrateddynamics`
   failed, and the exit status was **0** with only a `bash: cd: …` line in the
   log. Use `BUILD_LOCAL=1` for SDK packages and read the log's tail — a build
   verdict is the `released slot` line plus the tool's own output.
3. **Level reads are handle methods, not host methods.** `level.block(host,pos)`,
   `level.block_state(host,pos)`, `level.block_property(…)`, `level.signal(…)`,
   `level.update_neighbors(…)`, `level.best_neighbor_signal(…)` live on
   `LevelRef` (the backend); `host.block_entity(level,pos)` is a host method.
   Mixing them fails with "no method named `level_block`".
4. **`LevelHost` (the block-signal hooks) can read no block entity and no state
   property.** It has `state_string`, `state_block`, `signal`, `block_state`.
   Hence the in-memory channel mirror and the memoized state→parts parse.
5. **`JavaString` is not exported.** `Compound` is `BTreeMap<JavaString, Tag>`;
   build a `Tag::Compound(..)` to read with `Tag::get(&str)`, and insert with
   `entries.insert(VALUE_TAG.into(), Tag::Int(..))` — the key type is inferred.
6. **`HashSet`/`HashMap` `new` is not const at this toolchain.** A static cache
   is `Mutex<Option<…>>` with `get_or_insert_with`, not `Mutex::new(HashMap::new())`.
7. **A `Part` cannot see the `Hooks` instance.** Cross-call state must be a
   `static` (here `Mutex<Option<State>>`); the mod struct is only for what the
   `Hooks` methods need.
8. **`Hooks` has no per-block methods** (see §5): a `fn block_signal` on `Hooks`
   is E0407. Attach the Part in `register`.
9. **`block_entity` returns `Option<Compound>` and the *declaration* owns the
   type id** — return `Some(Compound::new())` for the cable, and declare the
   type with `declare_block_entity_type("cable", vec!["integrateddynamics:cable".into()])`.
10. **`check --json` reports `capabilities: []`** while the manifest has
    `capabilities = ["registries"]` and `uses.json` records `capability:registries`.
    That is the report's summary, not a defect; do not "fix" the manifest.
11. **`uses.json` is text-based and over-approximates.** It lists
    `hook:command`/`method:Hooks::command` and `EntitiesHost::entity` because the
    source mentions `Command::new`/`entity`; treat it as a usage sketch, not as a
    subscription list.
12. **`cargo fmt --check`, never `--all`**: `.horizon/sdk` is a path dependency
    and the SDK's own files are not formatted for this crate.
13. **`Cargo.lock` changes the artifact.** `cargo test`/`clippy` write one; the
    packer then adds it and the sha changes. `rm -f Cargo.lock` right before
    `cargo horizon-mod build`, and `/Cargo.lock` in `.gitignore`. The delivered
    package has **no** `Cargo.lock` (40 entries).
14. **A stale package is easy to ship.** The first package here was built before
    a last `cargo fmt`/glue edit and its sha (`7a006271…`) no longer matched the
    sources; the rebuild gave `3c15622a…`, and a later `README.md` edit (it is
    **packed**) gave the delivered `4723b651…`. Rebuild after *any* edit — source
    **or** a packed root file — and compare `cmp`/sha, not timestamps.
15. **No `rust-toolchain.toml`, no `icon.png`.** The pinned commit decides the
    toolchain; the icon is optional and this port has none.
16. **The carried content must be at the package root** (`assets/<id>/`,
    `data/…`), not under `content/`; the packer refuses stray package paths and
    silently ships only what it knows.

### What is family-wide glue, and what is this mod only

For `integratedtunnels`, `integratedcrafting`, `integratedterminals` and
`integratedscripting`, which are built from this port as the parent pattern:

**Family-wide (copy it):**

1. the single-package layout and manifest (`api = "0.2"`, `side = "both"`,
   `capabilities = ["registries"]`, `[native]` with both 40-hex commits), the
   `pub mod runtime; mod support;` split, `.gitignore`, licence files inside the
   package, and the "`runtime.rs` byte-identical" rule;
2. the **Part declaration pattern**: one `BlockPart`/`BlockEntityPart` type per
   family block, attached in `Hooks::register` with `block_key` +
   `declare_block_entity_type` + `attach_block*_part`; a moment that belongs to
   that block is a Part method, never a `Hooks` method;
3. the **network-state pattern**: queue `(dimension, pos)` in
   `block_entity_tick` (the tick must not write), recompute at
   `level_tick_post`, read inputs with the per-face `LevelRef::signal`, write the
   saved compound once per change, notify with `LevelRef::update_neighbors`,
   mirror the hot answer for `block_signal`, and drop it in `level_unload`;
4. the **harness-command pattern**: typed `Command::new("id sub <arg:block_pos>")`
   usages, one `run_mod_command` dispatcher, `Commands::feedback` for replies,
   and a dimension guard before any `set_block_property`;
5. the **carry pattern**: content at the package root; no embedded-content
   injection any more;
6. the build/check ladder and the traps above.

**This mod only (re-derive per child):**

1. the cable's six `part_<face>` properties + `status` (a child block has its
   own state table, so its own `parts_from_state_string` shape);
2. the redstone channel semantics (`channel_value`, `signal_for_face`,
   `status_for`, the reader/writer parts) — the tunnels' items/fluids/energy
   channels are a different value model;
3. `use_item_on` placing a *part item* onto a cable face, and the
   `integrateddynamics:part_*` item ids;
4. the `/integrateddynamics …` subcommands and their exact wording (the e2e
   asserts them).

### What still needs an in-game run

The in-game ladder could **not** run here: a native SDK mod is compiled into the
game by the **launcher**, and `horizon-launcher` is not installed in this
environment (`cargo horizon-mod run` answers: *"code mods run through the
launcher's native build: horizon-launcher native-build . --trust-native-code"*).
`tools/e2e.sh` is kept unchanged (the assertions are the behaviour's proof) and
must be run with a launcher-built game before the ledger moves past
`ported-partial`:

```sh
horizon-launcher native-build ports/integrateddynamics --instance <instance> --trust-native-code
GAME=<that instance's horizon-client> CTL=ports/_shared/bin/horizon-1d63e0a5/horizon-ctl tools/e2e.sh
```

Two things that run only prove out there, and are named as risks rather than
claims:

- the cable gets its block entity from the **declared type** even though its
  content template is `minecraft:stone` (at the 0.1.0 pin the same
  hook-declared block entity worked in game; the SDK declares the type instead
  of answering a hook). If a launcher run shows no block entity, the one-line
  fix is a block-entity-capable template in
  `data/integrateddynamics/horizon/block/cable.json`;
- the e2e's `/integrateddynamics` readout now reaches the **typed** command
  path; its three usages and the harness phases are unchanged in wording.

## Upstream defects fixed

None found in this slice. The 0.1.0 record listed "the pin has no per-face
redstone read" as an **API** gap, not an upstream defect; the SDK closes it, so
the reader is now upstream's own target-face read and the deviation table above
records the withdrawal. Nothing was copied from a Java defect, and nothing was
"fixed" that upstream does on purpose.

## Verification

Every command ran through `ports/tools/build-slot.sh` with `BUILD_LOCAL=1`,
`CARGO_INCREMENTAL=0`, no bare `cargo` outside a lease, and no `--target-dir`.
Logs: `ports/_shared/evidence/integrateddynamics/`.

| Step | Command | Result |
|---|---|---|
| runtime only, **before the glue** | `cargo test` on the crate that had only `pub mod runtime;` | **12 passed**, 0 failed — `sdk-test-runtime-only.log` |
| runtime byte-identical | `sha256sum` + `cmp` against `mixins/integrateddynamics/runtime.rs` | `91de3aa0…bdd692`, `BYTE-IDENTICAL` |
| tests (after the glue) | `cargo test` | **18 passed**, 0 failed (12 runtime + 6 state-parser) — `sdk-test.log` |
| lints | `cargo clippy --all-targets -- -D warnings` | exit 0, **0 warnings** — `sdk-clippy.log` |
| formatting | `cargo fmt --check` | clean — `sdk-fmt.log` |
| package | `cargo horizon-mod build` | `target/horizon-mod/integrateddynamics.hmod` — `sdk-build.log` |
| reproducibility | build twice, `sha256sum` both | **both `4723b651…a7c72`** — `sdk-build-repro.log` |
| package check | `cargo horizon-mod check integrateddynamics.hmod` | exit 0, verbatim below — `sdk-check.log` |
| fits | `cargo horizon-mod fits integrateddynamics.hmod --build nightly` | exit 0, `0b14f8a  made for` — `sdk-fits.log` |
| packaged runtime | `cmp` the zip's `src/runtime.rs` against the retired one | **byte-identical** |

`check`, verbatim:

```
integrateddynamics 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  37709 bytes, SHA-256 4723b651f5b77b5fa61af1dc2f93f4a8dab828d3bc74b85f58605c4c3a0a7c72, unsigned
  native source: unsandboxed; not executed by the registry check. Compile and test against the exact pinned Horizon and Mixins revisions before review.
```

`fits`, verbatim:

```
Integrated Dynamics:
0b14f8a  made for
```

The 12 runtime tests include the 0.1.0 review repair's two guards
(`a_sparse_line_keeps_its_whole_cable_budget`, `a_block_that_is_not_a_cable_starts_no_network`),
so carrying `runtime.rs` byte-identically carries that fix too.

### The deliverable (one file)

`ports/integrateddynamics/integrateddynamics.hmod` is **the** deliverable:

- **37,709 B**, SHA-256
  `4723b651f5b77b5fa61af1dc2f93f4a8dab828d3bc74b85f58605c4c3a0a7c72`,
  **40 entries**: `horizon-mod.toml`, `Cargo.toml`, `LICENSE`,
  `LICENSE-NOTICE.md`, `README.md`, `src/lib.rs`, `src/runtime.rs` (byte-identical
  to 0.1.0's), `src/support.rs`, `uses.json` (329 entries, `made_for 0b14f8a`),
  and the 31 carried content files under `assets/integrateddynamics/` and `data/`.
- no `Cargo.lock` inside (a lock would change the sha and pin newer crates than
  the SDK's own).
- `cargo horizon-mod check` and `fits` accept it at `0b14f8a`.

### Not run here (honest gap)

- the in-game e2e (`tools/e2e.sh`) — needs `horizon-launcher`; see
  [What still needs an in-game run](#what-still-needs-an-in-game-run);
- therefore `ported-partial` stands: the code builds, the pure and parser tests
  pass, and the package is registry-valid, but the *behaviour* at this pin is
  unverified in a world.

## Known gaps

The slice boundary and gaps above stand (variable system, every other part, the
machines, the screens, energy, the menril chain beyond the chunk, the multipart
renderer, per-face channels). Recorded for this migration:

- the writer emits **weak** power only (`direct = false`, upstream's aspect
  default); a strong answer is not carried;
- the network is recomputed at `level_tick_post` from the cables that ticked,
  bounded to 1,024 cables; there is no incremental network graph;
- a cable with no reader or writer still re-evaluates every tick (one integer
  channel per network — the maximum reader level);
- part placement is `use_item_on` on a cable face with the part item (upstream's
  screen/card layer is out of slice), and the harness command
  `/integrateddynamics info|part|probe` stands in for the GUI;
- the channel mirror is per level and dropped on `level_unload`; a cable that
  never ticked answers 0 until its first `level_tick_post`;
- `/integrateddynamics part` refuses a dimension other than the one in place
  (the write helpers act there);
- `src/support.rs` carries 6 tests of the state-string parser; the rest of the
  glue is exercised only in game (the e2e), which is the gap above.

## Changelog for the resubmission

> Supersedes #852, closed because Horizon retired `.hnmod` (#17992). Changes:
> flat SDK package with `api = "0.2"` and `[native]` pinned to `0b14f8a`; hooks
> rewritten as `impl mod_api::Hooks` methods against typed hosts/events (12
> helpers mapped, 1 recorded as a gap — the embedded-content injection, which the
> single package makes unnecessary); `runtime.rs` and its tests unchanged.
