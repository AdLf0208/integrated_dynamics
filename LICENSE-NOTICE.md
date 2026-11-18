# Licence notice

This port is a clean-room Rust rewrite of **Integrated Dynamics 1.38.2** by
rubensworks (CyclopsMC), distributed under the **MIT License**.

- Upstream: <https://github.com/CyclopsMC/IntegratedDynamics>, tag
  `1.21.1-1.38.2` (commit `807508c12286c26dd7d97b059bdb450f18af6ca8`).
- Licence read from the pack jar's `META-INF/neoforge.mods.toml`
  (`license="MIT"`, `modId="integrateddynamics"`) and from the upstream
  `LICENSE.txt` (Copyright (c) 2015 Cyclops), copied verbatim into `LICENSE`.
- Pack jar: `integrateddynamics-1.21.1-neoforge-1.38.2.jar`, SHA-1
  `1a77cf34678dcdb3a11ab9695af6ce588e6fc600` (the ATM10 pack record's
  `modrinthHash`), kept at
  `ports/_shared/carry-work/integrateddynamics/integrateddynamics-1.21.1-neoforge-1.38.2.jar`.
- Carried assets (models, textures, lang values, the cable recipe and the
  crystalized menril chunk icon) are the upstream project's MIT-licensed
  resources; the cable's own block model and blockstate are authored here
  because upstream draws the cable with a block-entity renderer.

The port is **local-only** until the maintainer explicitly approves publishing
(campaign ARCHITECTURE.md §8). A published copy must keep the MIT notice above.
