# Integrated Dynamics — jar inventory and slice 1 findings

Ported from **Integrated Dynamics 1.38.2** (`CyclopsMC/IntegratedDynamics`, MIT),
Minecraft 1.21.1 NeoForge, from the exact ATM10 pack jar.

## Provenance (step 1 — the jar is verified against the pack record)

Pack record (`ports/_shared/licence-cache/atm10-modlist.json`, row
`integrateddynamics-1.21.1-neoforge-1.38.2.jar`):

| field | value |
|---|---|
| `modrinthHash` (SHA-1) | `1a77cf34678dcdb3a11ab9695af6ce588e6fc600` |
| `curseForgeHash` | `2217547851` |
| version | `1.38.2` |

The pack file is the **CurseForge** build (its SHA-1 is not on Modrinth). Resolved
from CurseForge file id `9385193` and verified:

| field | value |
|---|---|
| file | `ports/_shared/carry-work/integrateddynamics/integrateddynamics-1.21.1-neoforge-1.38.2.jar` |
| size | **4,501,170 B** |
| SHA-1 | `1a77cf34678dcdb3a11ab9695af6ce588e6fc600` — **matches the pack record** |
| SHA-256 | `7dee30453f1e8972962108175cf46c09c7531ab273279888bf2162198d241198` |
| licence (jar `META-INF/neoforge.mods.toml`) | `license="MIT"`, `modId="integrateddynamics"`, `version="1.38.2"` |

The jar is kept as licence/provenance evidence. **Cache discrepancy recorded:**
`ports/_shared/licence-cache/integrateddynamics-1.21.1-neoforge-1.38.2.jar`
(SHA-1 `369ea904…`, the Modrinth `1.21.1-1.38.2-2422` CI build renamed to the
pack filename) does **not** match the pack record — `audit_licences.py`'s
project-version fallback picked the newest version containing `1.38.2`. This
slice uses the verified pack jar from `carry-work/`, never the cache copy. The
cache is read-only to this job (not in its write ownership); the orchestrator
may want to replace it with the CurseForge file.

## Inventory (step 2 — written before coding)

3,755 entries: 66 blockstates, 85 item models, 300 recipes, 39 block loot
tables, 43 advancements, 495 textures, 31 lang files, 6 worldgen files,
60×`org/cyclops/integrateddynamics/**/*.class`.

Block families in `assets/integrateddynamics/blockstates/`:

- **Network/logic**: `cable`, `variablestore`, `logic_programmer`,
  `logic_programmer` items, `proxy`, `materializer`, `delay`, `labeller`.
- **Parts** (18 models, `facing` six-way): `part_redstone_reader`,
  `part_redstone_writer`, `part_block_reader`, `part_entity_reader/writer`,
  `part_inventory_reader/writer`, `part_fluid_reader`, `part_machine_reader/writer`,
  `part_audio_reader/writer`, `part_network_reader`, `part_world_reader`,
  `part_extradimensional_reader`, `part_effect_writer`,
  `part_display_panel`, `part_static_light_panel`, `part_dynamic_light_panel`,
  `part_connector_mono_directional`, `part_connector_omni_directional`.
- **Machines**: `coal_generator`, `energy_battery`, `energy_battery_creative`,
  `drying_basin`, `mechanical_drying_basin`, `mechanical_squeezer`, `squeezer`.
- **Menril resources**: log/wood/planks/leaves/sapling/berries/door/fence/gate/
  torch (stone/wood), liquid chorus, menril resin, crystalized chorus/menril
  blocks, slabs, stairs, bricks.
- **Items**: `variable`, `variable_transformer_input/output`, `wrench*`,
  `enhancement_offset`, `facade`, `on_the_dynamics_of_integration`, `portable_logic_programmer`,
  `menril_berries`, `crystalized_menril_chunk`, `crystalized_chorus_chunk`,
  buckets, `invisible_light`.

There is **no** standalone `part_*` block registration in the Java sources:
parts are data inside a cable's `BlockEntityMultipartTicking`
(`ItemPart.useOn` → `PartHelpers.addPart`); the `part_*` blockstates are model
lookups only. One cable block carries up to six parts, each with `facing`.

## Slice 1 chosen: the redstone reader → cable network → redstone writer

The smallest honest **network** behaviour in the jar, and the mod's own
tutorial's first "network foundations" step: a **Redstone Reader** part on a
Logic Cable reads the redstone level of the block it targets, the cable network
carries the value, and a **Redstone Writer** on the same network writes it out
as a redstone level on its target face. It is the mod's `redstone_transmission`
advancement without the variable-card layer.

What the slice ships:

- `integrateddynamics:cable` — a non-occluding 6 px core, six
  `part_<face>` enum state properties (`none|reader|writer`), a `status`
  property (`inactive|active`), and a block entity holding the network channel
  value (`value`, 0..15).
- `integrateddynamics:part_redstone_reader` and
  `integrateddynamics:part_redstone_writer` **items**, placed on a clicked
  cable face (`right_click_block`), consumed outside creative; parts live in
  the cable's state and block entity, as upstream's multipart data lives in the
  cable.
- Reader: samples the redstone level reaching the cable and stores it in the
  network channel; Writer: emits the network channel value as a weak redstone
  signal on its own face (`native_block_signal` + `is_signal_source`).
- The network is the connected cable set (6-way adjacency); every cable in it
  stores and exposes the same channel value.
- `/integrateddynamics info <x> <y> <z>` and
  `/integrateddynamics part <x> <y> <z> <face> <none|reader|writer>` for the
  harness (upstream's screen is a gap).

Not in this slice (recorded gaps): the variable card system (variable store,
logic programmer, materializer, proxy, delay), every other part, the machines,
the menril resource chain (only the chunk item is carried for the cable
recipe), `faceless`/mono-directional cable connectors, screens, power/energy
consumption, and the collision/render shapes of upstream's multipart cable.

## Upstream behaviour extracted (analysis aid, not copied)

Repository <https://github.com/CyclopsMC/IntegratedDynamics>, tag
`1.21.1-1.38.2`, commit `807508c12286c26dd7d97b059bdb450f18af6ca8`
(shallow clone in `ports/_shared/upstream/integrateddynamics`, deleted after
these notes). Files read:

- `src/main/java/org/cyclops/integrateddynamics/core/item/ItemPart.java` —
  placement: click a cable face with a part item → `PartHelpers.addPart` on
  that face; otherwise place a *fake cable* in the adjacent block (not ported).
- `part/PartTypeRedstoneReader.java`, `part/PartTypeRedstoneWriter.java` —
  the aspect lists (reader: low/nonlow/high/clock/integer value/comparator;
  writer: boolean/integer and their pulse variants).
- `part/aspect/read/AspectReadBuilders.java` (`Redstone.PROP_GET`) — the
  reader reads `level.getSignal(targetPos, targetSide)`, i.e. the signal the
  **target block** gives toward the part; a redstone wire's `power` is read
  directly when `getSignal` is 0. `PROP_GET_CLOCK` is a game-time modulo.
- `part/aspect/write/AspectWriteBuilders.java` (`Redstone`) — the writer sets
  a level (boolean = 15/0) on its `IDynamicRedstone` capability; pulse variants
  schedule a one-tick pulse.
- `capability/dynamicredstone/DynamicRedstoneHolderGlobal.java`,
  `DynamicRedstoneDefault.java`, `PartTarget.java` — the writer's default
  target is the block it faces; `setRedstoneLevel(level, direct)` writes the
  level into the target side and calls `neighborChanged` (and
  `updateNeighborsAt` for strong power).
- `core/part/write/PartTypeWriteBase.java` — a writer only writes through its
  **active aspect**, which needs a variable card in its slot; the part status
  is active/inactive/error.
- Part storage: `RegistryEntries.BLOCK_ENTITY_MULTIPART_TICKING` — one cable
  block entity holds the parts of its six faces.

## Behaviours carried into the rewrite (and where it differs)

| Upstream | Here | Why |
|---|---|---|
| Parts are multipart data in the cable BE, target = block on the clicked face | same shape: six `part_<face>` states + cable BE, target = the neighbour on that face | faithful |
| Reader reads `getSignal(targetPos, targetSide)` | `native_neighbor_signal(cable_pos)` (API 96), the strongest signal reaching the cable | the pin has no per-face signal read for mods (ARCHITECTURE §3 gap); with one reader and inert other faces the answers agree; recorded |
| Writer emits on the target block through `IDynamicRedstone`, per-aspect level | the cable answers `native_block_signal` (API 38) on the writer face, weak only | the writer is a part of the cable block here; weak is the aspect default (`direct=false`) |
| Writer value = the active aspect's variable-card stream | writer value = the network channel: the **maximum** reader level on the connected cables | one channel per network stands in for the variable system; recorded |
| Reader/writer need a variable card and an aspect selection | always on once placed; `status=active` when the channel is > 0 | the card/aspect UI is out of slice |
| Cable network forms/splits on cable add/remove with a network id | connected cable set recomputed each level tick from the cable's own tick | the pin's BE writes inside a tick are reverted; recompute is safe at a handful of cables; a per-network dirty cache is future work |
| Cable renders a multipart core + arms + part models | content multipart: a 6 px core + the carried part models on the six faces | content blocks cannot draw a BE model; the carried part textures are real |
| Cable and parts drop themselves on break | self-drop loot tables (26.3 singular `condition`) | no loot tables exist for them in the jar (the drop is code) |
| Cable recipe: 3 per craft from crystalized menril chunk, stick, redstone dust | carried, with a `crystalized_menril_chunk` item carried from the jar | the chunk is a plain item in this slice |
| Part recipes need `variable_transformer_input/output` | **not carried** | those belong to the unported variable system; the harness gives the parts; recorded gap |

No Java was copied or translated, and the jar was not decompiled.
