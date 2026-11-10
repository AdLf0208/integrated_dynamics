#!/usr/bin/env bash
# In-game check for Integrated Dynamics slice 1: a Redstone Reader on a Logic
# Cable reads the redstone level of the block it targets, the connected cable
# network carries the value, and every Redstone Writer on that network emits it
# on its own face.
#
# Every mod command is typed with `ctl chat "/cmd"` because `ctl command` runs
# the console path and bypasses the native `perform_command` hook.
#
# The End, in the void at x=200 (away from the central island, its dragon egg
# and the ambient item entities of a generated level), a stone floor at y=100.
#
# Default run (EMBEDDED=0, the content loaded with `--dev-mod content/`):
#  A  a reader-fed cable network lights a lamp three cables away: value 15,
#     lamp lit, cable status active;
#  B  the same network with its input removed is value 0 and the lamp dark;
#  B2 a redstone wire line gives an analog level 14 (not just on/off);
#  C  a redstone torch beside the reader is another 15 source;
#  D  two writers on one network both light (the value reaches every sink);
#  E  a disconnected cable with a writer and no reader stays dark (the value is
#     the network's, not a global);
#  F  removing the reader part drops the channel to 0 and both lamps go dark;
#  G  right-clicking a cable face with a Redstone Reader places the part
#     (upstream's `ItemPart.useOn`).
#
# EMBEDDED=1 launches the same binary with only the native bundle's embedded
# content (no `--dev-mod`); CONTROL=1 (default with EMBEDDED) first launches it
# with `.horizon-cache` unusable, where the ids must be unknown.
#
# SELFTEST=1 flips phase A's expected value (it must FAIL), proving the
# scenario can fail.
set -Eeuo pipefail
export LC_ALL=C
[ -z "${ID_DEBUG:-}" ] || set -x

EMBEDDED=${EMBEDDED:-0}
CONTROL=${CONTROL:-$EMBEDDED}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}

CLONE=${CAMPAIGN_ROOT:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CLONE/ports/integrateddynamics
PIN_BIN=$CLONE/ports/_shared/bin/horizon-1d63e0a5
CTL=${CTL:-$PIN_BIN/horizon-ctl}
BIN_ROOT=${HORIZON_TARGET_DIR:-$PORT/target/horizon-cargo}
GAME=${GAME:-$BIN_ROOT/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-${HORIZON_RES:-$HOME/.local/horizon-client/26.3}}}
SOCK=${SOCK:-/tmp/opencode/integrateddynamics-e2e.sock}
WORLD=IntegratedDynamicsE2E
CONTROL_WORLD=IntegratedDynamicsControl
# Where the arena stands: the End's void, far from the central island.
X0=200
Y=101
EXPECT_A=${EXPECT_A:-15}
# The selftest asks for a level the reader cannot produce, so phase A must fail.
if [ "$SELFTEST" = 1 ]; then EXPECT_A=14; fi

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/integrateddynamics-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
# The game folder is where the game starts (no `--game-dir` is passed), and the
# bundle's `content_root` helper writes `<game folder>/.horizon-cache`. `launch`
# changes into `$SCRATCH`, so that is the folder the control leg must block; if a
# `--game-dir` is ever added, set GAME_DIR to the same path.
GAME_DIR=${GAME_DIR:-$SCRATCH}
CACHE=$GAME_DIR/.horizon-cache
EMBEDDED_FILE=$CACHE/integrateddynamics-embedded-content.hmod
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() { echo "FAIL: $*" >&2; ctl logs --lines 60 >&2 || true; exit 1; }
on_error() { echo "FAIL: line $1: '$2' exited $?" >&2; ctl logs --lines 40 >&2 || true; }
trap 'on_error "$LINENO" "$BASH_COMMAND"' ERR

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
    # Last line of every run: `check-e2e-logs.py` reads it as the verdict.
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() { # launch [WORLD]
    local world=${1:-$WORLD}
    rm -f "$SOCK"
    cd "$SCRATCH"
    local args=(--game "$GAME" --world "$world" --seed 20261105 --creative --headless
                --wait --resources "$RES")
    if [ "$EMBEDDED" = 1 ]; then
        echo "  launch: no --dev-mod; the bundle carries its own content" >&2
    else
        args+=(--dev-mod "$PORT/content")
    fi
    "$CTL" --socket "$SOCK" launch "${args[@]}" >/dev/null
    ctl wait-ready 120
    ctl wait 40
}

block_state() { ctl block "$1" "$2" "$3" | jq -r '.state // empty'; }
block_name() { ctl block "$1" "$2" "$3" | jq -r '.block // empty'; }

# The End's void at X0: a stone floor and clear air above it. The player is
# teleported first, so the chunks the fills touch are loaded.
arena() {
    ctl command "execute in minecraft:the_end run tp @s $X0.5 $Y 0.5" >/dev/null
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(ctl block "$X0" 100 0 | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the End arena chunk never loaded"
    ctl command "execute in minecraft:the_end run fill $((X0 - 8)) 100 -14 $((X0 + 12)) 100 14 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run fill $((X0 - 8)) $Y -14 $((X0 + 12)) 108 14 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run kill @e[type=!player]" >/dev/null
    ctl command "execute in minecraft:the_end run tp @s $X0.5 $Y 0.5" >/dev/null
    ctl wait 20
    [ "$(block_name "$X0" 100 0)" = "minecraft:stone" ] || fail "the arena floor is not stone"
    [ "$(block_name "$X0" $Y 0)" = "minecraft:air" ] || fail "the arena air above the floor is not air"
}

set_block() { # set_block X Y Z STATE
    ctl command "execute in minecraft:the_end run setblock $1 $2 $3 $4" >/dev/null
    ctl wait 5 >/dev/null
}

# `/integrateddynamics info X Y Z`: parse the value and status out of the reply.
info() { # info X Y Z
    say "integrateddynamics info $1 $2 $3"
    ctl wait 12 >/dev/null
    INFO_LINE=$(ctl messages 12 | jq -r '.chat[]' | grep -m1 'value' || true)
    [ -n "$INFO_LINE" ] || fail "no /integrateddynamics info reply"
    VALUE=$(sed -n 's/.*value \([0-9-]*\);.*/\1/p' <<<"$INFO_LINE")
    STATUS=$(sed -n 's/.*status \([a-z]*\).*/\1/p' <<<"$INFO_LINE")
    [ -n "$VALUE" ] || fail "the info line did not parse: $INFO_LINE"
    echo "  $INFO_LINE" >&2
}

probe() { # probe X Y Z: the engine's strongest signal at a position
    say "integrateddynamics probe $1 $2 $3"
    ctl wait 12 >/dev/null
    PROBE_LINE=$(ctl messages 12 | jq -r '.chat[]' | grep -m1 'neighbour signal' || true)
    echo "  ${PROBE_LINE:-no probe reply}" >&2
}

expect_value() { # expect_value VALUE STATUS LABEL
    [ "$VALUE" = "$1" ] || fail "$3: the cable's value is $VALUE, expected $1"
    [ "${STATUS:-}" = "$2" ] || fail "$3: the cable's status is '${STATUS:-}', expected $2"
}

lamp_lit() { # lamp_lit X Y Z LABEL
    local state
    state=$(block_state "$1" "$2" "$3")
    case "$state" in
        *lit=true*) echo "  $4: $state" >&2 ;;
        *) fail "$4: the lamp is not lit ($state)" ;;
    esac
}

lamp_dark() { # lamp_dark X Y Z LABEL
    local state
    state=$(block_state "$1" "$2" "$3")
    case "$state" in
        *lit=false*) echo "  $4: $state" >&2 ;;
        *) fail "$4: the lamp is lit ($state)" ;;
    esac
}

# The slice-1 scene: reader cable at (X0,Y,0) fed from the west, two more
# cables, a writer on the east face of the last one, and a lamp beyond it.
scene() {
    set_block $((X0 - 1)) $Y 0 "minecraft:redstone_block"
    set_block "$X0" $Y 0 "integrateddynamics:cable[part_west=reader]"
    set_block $((X0 + 1)) $Y 0 "integrateddynamics:cable"
    set_block $((X0 + 2)) $Y 0 "integrateddynamics:cable[part_east=writer]"
    set_block $((X0 + 3)) $Y 0 "minecraft:redstone_lamp"
    ctl wait 10 >/dev/null
}

run() {
    arena

    echo "== A: a reader-fed network lights a lamp three cables away =="
    scene
    info "$X0" $Y 0
    expect_value "$EXPECT_A" active "A"
    echo "  cable state: $(block_state "$X0" $Y 0)"
    info $((X0 + 2)) $Y 0
    echo "  writer cable state: $(block_state $((X0 + 2)) $Y 0)"
    probe $((X0 + 3)) $Y 0
    lamp_lit $((X0 + 3)) $Y 0 "A"

    echo "== B: the same network with its input removed is dark =="
    set_block $((X0 - 1)) $Y 0 "minecraft:air"
    ctl wait 10 >/dev/null
    info "$X0" $Y 0
    expect_value 0 inactive "B"
    lamp_dark $((X0 + 3)) $Y 0 "B"

    echo "== B2: a redstone wire line gives an analog level (14) =="
    set_block $((X0 - 3)) $Y 0 "minecraft:redstone_block"
    set_block $((X0 - 2)) $Y 0 "minecraft:redstone_wire"
    set_block $((X0 - 1)) $Y 0 "minecraft:redstone_wire"
    ctl wait 10 >/dev/null
    info "$X0" $Y 0
    expect_value 14 active "B2"
    lamp_lit $((X0 + 3)) $Y 0 "B2 (any non-zero level lights a redstone lamp)"

    echo "== C: a redstone torch beside the reader is another 15 source =="
    set_block $((X0 - 3)) $Y 0 "minecraft:air"
    set_block $((X0 - 2)) $Y 0 "minecraft:air"
    set_block $((X0 - 1)) $Y 0 "minecraft:redstone_torch"
    ctl wait 10 >/dev/null
    info "$X0" $Y 0
    expect_value "$EXPECT_A" active "C"
    lamp_lit $((X0 + 3)) $Y 0 "C"

    echo "== D: two writers on one network both light =="
    set_block $((X0 + 1)) $Y -1 "minecraft:redstone_lamp"
    say "integrateddynamics part $((X0 + 1)) $Y 0 north writer"
    ctl wait 10 >/dev/null
    info $((X0 + 1)) $Y 0
    lamp_lit $((X0 + 1)) $Y -1 "D"

    echo "== E: a disconnected cable with a writer and no reader stays dark =="
    set_block $((X0 + 6)) $Y 4 "integrateddynamics:cable[part_east=writer]"
    set_block $((X0 + 7)) $Y 4 "minecraft:redstone_lamp"
    ctl wait 10 >/dev/null
    info $((X0 + 6)) $Y 4
    expect_value 0 inactive "E (the disconnected cable's own channel)"
    lamp_dark $((X0 + 7)) $Y 4 "E"

    echo "== F: removing the reader part drops the channel to 0 =="
    say "integrateddynamics part $X0 $Y 0 west none"
    ctl wait 10 >/dev/null
    info "$X0" $Y 0
    expect_value 0 inactive "F"
    lamp_dark $((X0 + 3)) $Y 0 "F"
    lamp_dark $((X0 + 1)) $Y -1 "F"

    echo "== G: right-clicking a cable face with the reader places the part =="
    # One block above the floor: the eye (feet + 1.62) then looks through the
    # 4 px core's west face, which a ray at floor level would pass under.
    set_block $((X0 + 6)) $((Y + 1)) 0 "integrateddynamics:cable"
    ctl command "item replace entity @s hotbar.0 with integrateddynamics:part_redstone_reader 2" >/dev/null
    ctl select 1 >/dev/null
    ctl wait 4 >/dev/null
    ctl command "execute in minecraft:the_end run tp @s $((X0 + 5)).5 $Y 0.5" >/dev/null
    ctl wait 8 >/dev/null
    local reply aim
    aim=$(ctl look-at-block $((X0 + 6)) $((Y + 1)) 0 west 2>&1) \
        || fail "cannot aim at the cable's west face: $(tr -d '\n' <<<"$aim" | cut -c1-300)"
    reply=$(ctl interact $((X0 + 6)) $((Y + 1)) 0 west 2>&1) \
        || fail "ctl interact on the cable's west face failed: $(tr -d '\n' <<<"$reply" | cut -c1-300)"
    ctl wait 8 >/dev/null
    say "integrateddynamics info $((X0 + 6)) $((Y + 1)) 0"
    ctl wait 12 >/dev/null
    local parts
    parts=$(ctl messages 12 | jq -r '.chat[]' | grep -m1 'parts' || true)
    echo "  $parts" >&2
    case "$parts" in
        *west=reader*) ;;
        *) fail "the right click did not place the reader on the west face: ${parts:-no reply}" ;;
    esac
}

control_leg() {
    echo "== control: the same binary with an unusable .horizon-cache =="
    # A regular file where the directory must be: the bundle cannot write its
    # content, so the ids it declares must be unknown.
    rm -rf "$CACHE"
    : > "$CACHE"
    launch "$CONTROL_WORLD"
    ctl command "execute in minecraft:the_end run tp @s 0.5 101 0.5" >/dev/null || true
    ctl wait 10 >/dev/null
    ctl command "setblock 0 100 0 integrateddynamics:cable" >/dev/null 2>&1 || true
    ctl wait 6 >/dev/null
    local block
    block=$(block_name 0 100 0)
    echo "  block: $block"
    [ "$block" != "integrateddynamics:cable" ] \
        || fail "the control placed a logic cable, so the embedded file proves nothing"
    ctl quit >/dev/null 2>&1 || true
    # Unblock: the next launch re-creates the cache directory itself.
    rm -f "$CACHE"
    sleep 2
}

# The embedded leg's own proof: the bundle wrote *its own* content where the
# control leg blocks it, so the control is not vacuous. Byte-identity with the
# built content/ mod is the claim: the bundle's data/ copy is what `native_data!`
# compiles in, and this is what the game then registered from the cache.
require_embedded_cache() {
    local built=$PORT/content/integrateddynamics.hmod
    [ -f "$EMBEDDED_FILE" ] \
        || fail "the bundle did not write $EMBEDDED_FILE, so the control leg would prove nothing"
    [ -f "$built" ] || fail "missing the built content mod: $built"
    local written_sha built_sha
    written_sha=$(sha256sum "$EMBEDDED_FILE" | cut -d' ' -f1)
    built_sha=$(sha256sum "$built" | cut -d' ' -f1)
    echo "  embedded content written: $EMBEDDED_FILE ($(stat -c %s "$EMBEDDED_FILE") B) sha256=$written_sha" >&2
    echo "  built content/          : $built ($(stat -c %s "$built") B) sha256=$built_sha" >&2
    cmp -s "$EMBEDDED_FILE" "$built" \
        || fail "the written embedded content is not byte-identical to $built"
    echo "  embedded content is byte-identical to the built content/ mod" >&2
}

launch
run
if [ "$EMBEDDED" = 1 ]; then
    require_embedded_cache
fi
if [ "$EMBEDDED" = 1 ] && [ "$CONTROL" = 1 ]; then
    control_leg
fi
echo "PASS: a Redstone Reader fed a Logic Cable network and a Redstone Writer three cables away lit a lamp at level ${EXPECT_A}; removing the input, a wire line at 14, a torch, a second writer, a disconnected network, a removed part and a right-click placement all matched"
