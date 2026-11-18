# Working on this Horizon mod (SDK layout, mod API 0.2)

> **Build slots:** every build/test/e2e goes through the campaign runner
> (`ports/tools/build-slot.sh` from the campaign root). It **blocks in the
> kernel** until a slot is free (no polling), sets `CARGO_BUILD_JOBS` per step,
> journals every lease, and exits **75** on a wait timeout. Read
> `../_shared/build-slots-usage.md` before running anything; observe with
> `status`/`watch`/`events`/`stats`. Never poll, never background a build,
> never pass `--target-dir`.
>
> The runner executes the command in the **campaign root**, so name the port:
> `env -C ports/integrateddynamics cargo test`. Add `BUILD_LOCAL=1`: the remote
> runner's `cd` failed here once and still reported exit 0.

This port is **one flat `.hmod`** (Horizon retired `.hnmod` on 2026-11-17):
`horizon-mod.toml` + `Cargo.toml` + `src/lib.rs` (the one
`#[mod_api::horizon_mod] impl Hooks`, plus the cable's `BlockPart` +
`BlockEntityPart`) + `src/runtime.rs` (pure, unit-tested, **byte-identical** to
its 0.1.0 copy) + `src/support.rs` (glue) + `assets/` + `data/`.

Read, in this order: `README.md`, `notes.md` (§Migration to the SDK format is the
recipe and the trap list), `../_shared/AGENT_MOD_STRUCTURE.md` **§0**, and the
SDK's own surface at `.horizon/sdk/api/crates/horizon-mod-vocab/generated/`
(the generated `Hooks`/`BlockPart`/`BlockEntityPart` traits and every event) and
`.../src/host/` (host methods). Use `cargo horizon-mod hooks <text>` instead of
guessing a hook name.

Rules that bite here:

- a moment that belongs to one block is a **Part**, not a `Hooks` method; the
  Part must be attached in `Hooks::register`;
- `block_signal`/`is_signal_source` get a read-only `LevelHost`: no block-entity
  read and no state-property read, so the channel is mirrored in memory and the
  parts come from a memoized `state_string` parse;
- level reads are handle methods (`level.block(host, pos)`), not host methods;
- `runtime.rs` stays byte-identical: put new logic in `support.rs` with tests, or
  justify a new slice;
- validate with `cargo test`, `cargo clippy --all-targets -- -D warnings`,
  `cargo fmt --check` (never `--all`), then `rm -f Cargo.lock` and
  `cargo horizon-mod build` + `check` + `fits`; the `.hmod` lives in this folder,
  never in `target/`;
- the in-game e2e needs a **launcher-built** game
  (`horizon-launcher native-build . --instance <i> --trust-native-code`); without
  it the port stays `ported-partial` and the notes say so.

Keep validation claims and the pinned revisions in `horizon-mod.toml` honest.
Never edit the pinned Horizon checkouts or player resources; do not commit
binaries, exported sources or Minecraft assets.
