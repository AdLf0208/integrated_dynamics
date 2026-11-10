#!/usr/bin/env python3
"""Assemble slice 1's `content/` folder from the pack's Integrated Dynamics jar.

The jar is the ATM10 pack file
(`ports/_shared/carry-work/integrateddynamics/integrateddynamics-1.21.1-neoforge-1.38.2.jar`,
SHA-1 `1a77cf34…c600` — the pack record's `modrinthHash`), kept as the licence
and provenance evidence. Nothing is invented: the part models, textures, lang
lines, the cable recipe and the menril chunk item come out of the jar,
re-spelled only where 26.3 differs (`{"tag": T}` → `"#T"`, a loot `conditions`
list → one `condition`). The cable's own model, blockstate, block declaration,
item definitions, creative tab and self-drop loot table are written here,
because upstream's cable is a block-entity-rendered multipart block whose model
the jar does not contain as a block model.

Rerun it to rebuild `content/` (it deletes its own generated folders first):

    python3 tools/assemble_content.py
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import shutil
import zipfile

ROOT = Path(__file__).resolve().parents[1]
JAR = (
    ROOT.parent
    / "_shared/carry-work/integrateddynamics/integrateddynamics-1.21.1-neoforge-1.38.2.jar"
)
CONTENT = ROOT / "content"
MOD = "integrateddynamics"
# The pack record's SHA-1 for the jar (atm10-modlist.json, modrinthHash).
JAR_SHA1 = "1a77cf34678dcdb3a11ab9695af6ce588e6fc600"

# Carried byte-for-byte: the jar's own assets.
CARRIED_ASSETS = [
    "assets/integrateddynamics/models/block/part.json",
    "assets/integrateddynamics/models/block/part_reader.json",
    "assets/integrateddynamics/models/block/part_writer.json",
    "assets/integrateddynamics/models/block/part_redstone_reader.json",
    "assets/integrateddynamics/models/block/part_redstone_writer_active.json",
    "assets/integrateddynamics/models/block/part_redstone_writer_inactive.json",
    "assets/integrateddynamics/models/item/part_redstone_reader.json",
    "assets/integrateddynamics/models/item/part_redstone_writer.json",
    "assets/integrateddynamics/textures/block/cable.png",
    "assets/integrateddynamics/textures/part/redstone_reader_back.png",
    "assets/integrateddynamics/textures/part/redstone_reader_front.png",
    "assets/integrateddynamics/textures/part/redstone_writer_front_active.png",
    "assets/integrateddynamics/textures/part/redstone_writer_front_inactive.png",
    "assets/integrateddynamics/textures/item/crystalized_menril_chunk.png",
]

# The six face names, in Horizon's `Direction` order (the block's properties).
FACES = ["down", "up", "north", "south", "west", "east"]

# A part model on a face: upstream's own `facing` rotations, keyed by our
# `part_<face>` property value.
ROTATION = {
    "down": {"x": 270},
    "up": {"x": 90},
    "north": {"y": 180},
    "south": {},
    "west": {"y": 90},
    "east": {"y": 270},
}

# The writer's active/inactive textures pick the model for the cable's status.
WRITER_MODEL = {
    "inactive": "integrateddynamics:block/part_redstone_writer_inactive",
    "active": "integrateddynamics:block/part_redstone_writer_active",
}


def read_jar() -> zipfile.ZipFile:
    data = JAR.read_bytes()
    got = hashlib.sha1(data).hexdigest()
    if got != JAR_SHA1:
        raise SystemExit(f"{JAR} has SHA-1 {got}, the pack record names {JAR_SHA1}")
    return zipfile.ZipFile(JAR)


def write(relative: str, value: object) -> None:
    path = CONTENT / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    if isinstance(value, bytes):
        path.write_bytes(value)
    else:
        path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def block_def() -> dict:
    """The cable block: a 4 px core (upstream's `CableModel.MIN..MAX`), six part
    properties and the writer's status. Hardness 3.0F is `BlockCable.BLOCK_HARDNESS`."""
    return {
        "template": "minecraft:stone",
        "properties": {
            "hardness": 3.0,
            "explosion_resistance": 3.0,
            "requires_tool": False,
            "sounds": "minecraft:stone",
            "occludes": False,
            "render_shape": "MODEL",
        },
        "state_properties": {
            **{f"part_{face}": {"type": "enum", "values": ["none", "reader", "writer"]} for face in FACES},
            "status": {"type": "enum", "values": ["inactive", "active"]},
        },
        "default_state": {
            **{f"part_{face}": "none" for face in FACES},
            "status": "inactive",
        },
        # Upstream's cable collision is the 4 px core plus an arm per connected
        # side; this slice's cable has no drawn connection arms, so the core is
        # the whole shape.
        "shapes": {"default": [[0.375, 0.375, 0.375, 0.625, 0.625, 0.625]]},
        "item": True,
    }


def blockstate() -> dict:
    multipart = [{"apply": {"model": "integrateddynamics:block/cable_core"}}]
    for face in FACES:
        part = f"part_{face}"
        multipart.append(
            {
                "when": {part: "reader"},
                "apply": {"model": "integrateddynamics:block/part_redstone_reader", **ROTATION[face]},
            }
        )
        for status, model in WRITER_MODEL.items():
            multipart.append(
                {
                    "when": {part: "writer", "status": status},
                    "apply": {"model": model, **ROTATION[face]},
                }
            )
    return {"multipart": multipart}


def cable_core() -> dict:
    """A cube from 6..10 px with the jar's cable texture (no block model exists
    in the jar: upstream draws the cable with a block-entity model)."""
    faces = {face: {"texture": "#all"} for face in FACES}
    return {
        "parent": "minecraft:block/block",
        "textures": {
            "particle": "integrateddynamics:block/cable",
            "all": "integrateddynamics:block/cable",
        },
        "elements": [{"from": [6, 6, 6], "to": [10, 10, 10], "faces": faces}],
    }


def item_model(texture: str) -> dict:
    return {
        "parent": "minecraft:item/generated",
        "textures": {"layer0": f"integrateddynamics:item/{texture}"},
    }


def item_definition(model: str) -> dict:
    return {"model": {"type": "minecraft:model", "model": f"integrateddynamics:item/{model}"}}


def cable_recipe() -> dict:
    """The jar's `data/integrateddynamics/recipe/crafting/cable.json`, in 26.3's
    string-ingredient shape (`{"tag": T}` → `"#T"`)."""
    return {
        "type": "minecraft:crafting_shaped",
        "category": "misc",
        "group": "integrateddynamics:cable",
        "pattern": ["CSC", "CRC", "CSC"],
        "key": {
            "C": "integrateddynamics:crystalized_menril_chunk",
            "S": "minecraft:stick",
            "R": "#c:dusts/redstone",
        },
        "result": {"id": "integrateddynamics:cable", "count": 3},
    }


def loot_table(item: str) -> dict:
    """26.3's singular typed condition (the 1.21.1 plural list reads false)."""
    return {
        "type": "minecraft:block",
        "pools": [
            {
                "bonus_rolls": 0.0,
                "rolls": 1.0,
                "condition": {"type": "minecraft:survives_explosion"},
                "entries": [{"type": "minecraft:item", "name": item}],
            }
        ],
        "random_sequence": f"integrateddynamics:blocks/{item.split(':')[1]}",
    }


def main() -> None:
    jar = read_jar()
    for folder in ("assets", "data"):
        shutil.rmtree(CONTENT / folder, ignore_errors=True)
    for relative in CARRIED_ASSETS:
        path = CONTENT / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(jar.read(relative))

    write("data/integrateddynamics/horizon/block/cable.json", block_def())
    write("assets/integrateddynamics/blockstates/cable.json", blockstate())
    write("assets/integrateddynamics/models/block/cable_core.json", cable_core())
    write("assets/integrateddynamics/models/item/cable.json", {"parent": "integrateddynamics:block/cable_core"})
    write("assets/integrateddynamics/models/item/crystalized_menril_chunk.json", item_model("crystalized_menril_chunk"))

    for name in ("cable", "crystalized_menril_chunk", "part_redstone_reader", "part_redstone_writer"):
        write(f"assets/integrateddynamics/items/{name}.json", item_definition(name))

    for name in ("crystalized_menril_chunk", "part_redstone_reader", "part_redstone_writer"):
        write(f"data/integrateddynamics/horizon/item/{name}.json", {"template": "minecraft:paper"})

    write("data/integrateddynamics/recipe/cable.json", cable_recipe())
    write(
        "data/c/tags/item/dusts/redstone.json",
        {"values": ["minecraft:redstone"]},
    )
    write("data/integrateddynamics/loot_table/blocks/cable.json", loot_table("integrateddynamics:cable"))
    write(
        "data/integrateddynamics/horizon/creative_tab/integrateddynamics.json",
        {
            "title": "Integrated Dynamics",
            "icon": "integrateddynamics:cable",
            "items": [
                "integrateddynamics:cable",
                "integrateddynamics:part_redstone_reader",
                "integrateddynamics:part_redstone_writer",
                "integrateddynamics:crystalized_menril_chunk",
            ],
        },
    )
    write(
        "assets/integrateddynamics/lang/en_us.json",
        {
            "block.integrateddynamics.cable": "Logic Cable",
            "item.integrateddynamics.part_redstone_reader": "Redstone Reader",
            "item.integrateddynamics.part_redstone_writer": "Redstone Writer",
            "item.integrateddynamics.crystalized_menril_chunk": "Crystallized Menril Chunk",
            "itemGroup.integrateddynamics.integrateddynamics": "Integrated Dynamics",
        },
    )
    # `horizon-mod.toml` is hand-written beside this script (`content/`), not
    # generated: the manifest is the port's, the assets and data are the jar's.


if __name__ == "__main__":
    main()
