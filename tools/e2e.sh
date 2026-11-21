#!/usr/bin/env bash
# In-game check for Integrated Dynamics slice 2: a Redstone Reader on a Logic
# Cable reads the redstone level of the block it targets, a **World Reader** reads
# the light level, the weather or the day cycle of the block it targets, the
# connected cable network carries the strongest reading, and every Redstone Writer
# on that network emits it on its own face.
#
# The game is the launcher's native build of this package (the content ships inside
# the `.hmod`), so no `--dev-mod` is passed; `launcher-game.sh build` exports
# GAME/CTL/RES. The same Nightly **without** the package (CONTROL_GAME) is the
# control leg: run this script with `GAME=$CONTROL_GAME` and it must FAIL (the ids
# and the command are unknown), which is what makes the leg non-vacuous.
#
# Every mod command is typed with `ctl chat "/cmd"` because `ctl command` runs the
# console path and bypasses the native command hook.
#
# A–G: the End, in the void at x=200 (away from the central island, its dragon egg
# and the ambient item entities of a generated level), a stone floor at y=100.
# H–P: the Overworld, in a roofed stone chamber at x=200/z=200, so the sky light at
# the test block is 0 and the light aspect has a clean differential.
#
#  A  a reader-fed cable network lights a lamp three cables away: value 15,
#     lamp lit, cable status active;
#  B  the same network with its input removed is value 0 and the lamp dark;
#  B2 a redstone wire line gives an analog level 14 (not just on/off);
#  C  a redstone torch beside the reader is another 15 source;
#  D  two writers on one network both light (the value reaches every sink);
#  E  a disconnected cable with a writer and no reader stays dark;
#  F  removing the reader part drops the channel to 0 and both lamps go dark;
#  G  right-clicking a cable face with a Redstone Reader places the part;
#  H  a World Reader's light aspect reads the glowstone above it as 15;
#  I  the same aspect reads 0 after the glowstone becomes stone (the differential);
#  J  weather_raining: clear 0, rain 15;
#  K  weather_clear: clear 15, rain 0;
#  L  weather_thunder: thunder 15, clear 0;
#  M  isday: noon 15, midnight 0;
#  N  isnight: midnight 15, noon 0;
#  O  right-clicking a cable face with the World Reader item places it (default
#     aspect light, reported by `info`);
#  P  an unknown aspect and a face without a World Reader are refused.
#
# SELFTEST=1 flips phase H's expected light level (the World Reader gives 15, the
# run demands 14): the new family's assertion must FAIL. SELFTEST=A does the same
# for phase A's redstone value.
set -Eeuo pipefail
export LC_ALL=C
[ -z "${ID_DEBUG:-}" ] || set -x

SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/integrateddynamics
PIN_BIN=$CLONE/ports/_shared/bin/horizon-0a875da
CTL=${CTL:-$PIN_BIN/horizon-ctl}
BIN_ROOT=${HORIZON_TARGET_DIR:-$PORT/target/horizon-cargo}
GAME=${GAME:-$BIN_ROOT/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/integrateddynamics-e2e.sock}
WORLD=${WORLD:-IntegratedDynamicsE2E}
# Where the End arena stands: the End's void, far from the central island.
X0=200
Y=101
# The Overworld chamber and its rig.
X1=200
YW=102
Z1=200
EXPECT_A=${EXPECT_A:-15}
EXPECT_WORLD_LIGHT=${EXPECT_WORLD_LIGHT:-15}
case "$SELFTEST" in
    1) EXPECT_WORLD_LIGHT=14 ;;   # the World Reader reads 15; H must fail
    A|a) EXPECT_A=14 ;;           # the Redstone Reader reads 15; A must fail
esac

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL (run launcher-game.sh env <mod>)" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/integrateddynamics-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
# The newest chat line matching a pattern. `ctl messages` lists chat newest
# first, so `head -1` is the reply the command just produced; a `tail` would pick
# the oldest match in the window, i.e. setup noise from an earlier phase (the
# false-proof hazard the parity ruling names).
newest() { ctl messages 12 | jq -r '.chat[]' | grep -E "$1" | head -1 || true; }
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

launch() {
    rm -f "$SOCK"
    cd "$SCRATCH"
    "$CTL" --socket "$SOCK" launch --game "$GAME" --world "$WORLD" --seed 20261105 --creative \
        --headless --wait --resources "$RES" >/dev/null
    ctl wait-ready 120
    ctl wait 40
    # `/integrateddynamics part|aspect` are operator-only since 0.2.1: they configure a
    # cable at any loaded coordinates without its screen or its item, so an ordinary
    # player uses the part items (the `use_item_on` path phases G and O check). The
    # harness runs them as an operator, exactly as a server admin would.
    ctl command "op @a" >/dev/null 2>&1 || true
    ctl wait 4
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

# A roofed stone chamber in the Overworld: the sky lights nothing inside, so the
# light aspect is 0 until a block emits, and the weather/day phases cannot leak in.
world_arena() {
    ctl command "execute in minecraft:overworld run tp @s $X1.5 101 $Z1.5" >/dev/null
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(ctl block "$X1" 100 "$Z1" | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the Overworld chamber chunk never loaded"
    # No hostile spawns in the dark chamber at midnight; the player is in creative.
    ctl command "difficulty peaceful" >/dev/null
    ctl command "weather clear" >/dev/null
    ctl command "time set noon" >/dev/null
    ctl command "fill $((X1 - 5)) 100 $((Z1 - 5)) $((X1 + 5)) 104 $((Z1 + 5)) minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "fill $((X1 - 4)) 101 $((Z1 - 4)) $((X1 + 4)) 103 $((Z1 + 4)) minecraft:air" >/dev/null
    ctl wait 12 >/dev/null
    [ "$(block_name "$X1" 100 "$Z1")" = "minecraft:stone" ] || fail "the chamber floor is not stone"
    [ "$(block_name "$X1" $YW "$Z1")" = "minecraft:air" ] || fail "the chamber interior is not air"
}

set_block_in() { # set_block_in DIMENSION X Y Z STATE
    ctl command "execute in $1 run setblock $2 $3 $4 $5" >/dev/null
    ctl wait 5 >/dev/null
}
set_block() { set_block_in minecraft:the_end "$@"; }

# `/integrateddynamics info X Y Z`: parse the value and status out of the reply.
info() { # info X Y Z
    say "integrateddynamics info $1 $2 $3"
    ctl wait 12 >/dev/null
    INFO_LINE=$(newest 'value [0-9-]+;')
    [ -n "$INFO_LINE" ] || fail "no /integrateddynamics info reply"
    VALUE=$(sed -n 's/.*value \([0-9-]*\);.*/\1/p' <<<"$INFO_LINE")
    STATUS=$(sed -n 's/.*status \([a-z]*\).*/\1/p' <<<"$INFO_LINE")
    [ -n "$VALUE" ] || fail "the info line did not parse: $INFO_LINE"
    echo "  $INFO_LINE" >&2
}

# The aspects line of `/integrateddynamics info`, e.g. `up=isday`.
aspects_reply() { # aspects_reply X Y Z
    say "integrateddynamics info $1 $2 $3"
    ctl wait 12 >/dev/null
    newest 'aspects '
}

probe() { # probe X Y Z: the engine's strongest signal at a position
    say "integrateddynamics probe $1 $2 $3"
    ctl wait 12 >/dev/null
    PROBE_LINE=$(newest 'neighbour signal')
    echo "  ${PROBE_LINE:-no probe reply}" >&2
    case "$PROBE_LINE" in
        *"neighbour signal "[0-9]*) ;;
        *) fail "the probe readout did not parse: ${PROBE_LINE:-no reply}" ;;
    esac
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

# The World Reader rig: a cable with a world reader on its up face and a writer on
# its east face, and a lamp east of it. The up face targets (X1,103,Z1).
world_scene() {
    set_block_in minecraft:overworld "$X1" $YW "$Z1" \
        "integrateddynamics:cable[part_up=world_reader,part_east=writer]"
    set_block_in minecraft:overworld $((X1 + 1)) $YW "$Z1" "minecraft:redstone_lamp"
    ctl wait 10 >/dev/null
}

aspect() { # aspect FACE NAME
    say "integrateddynamics aspect $X1 $YW $Z1 $1 $2"
    ctl wait 8 >/dev/null
    local reply
    reply=$(newest '\): aspect ')
    echo "  ${reply:-no aspect reply}" >&2
    case "$reply" in
        *"aspect $2"*) ;;
        *) fail "the aspect command did not report $2: ${reply:-no reply}" ;;
    esac
}

expect_aspect() { # expect_aspect FACE EXPECTED LABEL
    local line
    line=$(aspects_reply "$X1" $YW "$Z1")
    [ -n "$line" ] || fail "$3: no aspects line in the info reply"
    echo "  $line" >&2
    case "$line" in
        *"$1=$2"*) ;;
        *) fail "$3: aspects is '$line', expected $1=$2" ;;
    esac
}

# `/weather` sets the target; the level's rain/thunder levels then move 0.01 per
# tick (vanilla `advanceWeatherCycle`). `Level.isRaining` is a rain level above
# 0.2 (about 20 ticks) and `isThundering` a thunder level above 0.9 (about 90),
# and `sky_darken` carries the rain (+3) and thunder (+10) darkening until they
# fall again, which `isday` reads. So a weather change is waited out.
weather() { # weather "rain 1000000" | clear | "thunder 1000000"
    ctl command "weather $1" >/dev/null
    case "$1" in
        clear) ctl wait 140 >/dev/null ;;
        thunder*) ctl wait 140 >/dev/null ;;
        *) ctl wait 60 >/dev/null ;;
    esac
}
time_set() { ctl command "time set $1" >/dev/null; ctl wait 10 >/dev/null; }

part_cmd() { # part_cmd X Y Z FACE PART: the harness path, reply parsed
    say "integrateddynamics part $1 $2 $3 $4 $5"
    ctl wait 8 >/dev/null
    local reply
    reply=$(newest ' on the ')
    echo "  ${reply:-no part reply}" >&2
    case "$reply" in
        *"$5 on the $4 side"*) ;;
        *) fail "the part command did not report $5 on the $4 side: ${reply:-no reply}" ;;
    esac
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
    part_cmd $((X0 + 1)) $Y 0 north writer
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
    part_cmd $X0 $Y 0 west none
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
    local reply aim parts
    aim=$(ctl look-at-block $((X0 + 6)) $((Y + 1)) 0 west 2>&1) \
        || fail "cannot aim at the cable's west face: $(tr -d '\n' <<<"$aim" | cut -c1-300)"
    reply=$(ctl interact $((X0 + 6)) $((Y + 1)) 0 west 2>&1) \
        || fail "ctl interact on the cable's west face failed: $(tr -d '\n' <<<"$reply" | cut -c1-300)"
    ctl wait 8 >/dev/null
    say "integrateddynamics info $((X0 + 6)) $((Y + 1)) 0"
    ctl wait 12 >/dev/null
    parts=$(newest 'parts ')
    echo "  $parts" >&2
    case "$parts" in
        *west=reader*) ;;
        *) fail "the right click did not place the reader on the west face: ${parts:-no reply}" ;;
    esac

    echo "== slice 2: the World Reader, in a roofed Overworld chamber =="
    world_arena
    world_scene

    echo "== H: the light aspect reads the glowstone above it as 15 =="
    aspect up light
    set_block_in minecraft:overworld "$X1" 103 "$Z1" "minecraft:glowstone"
    ctl wait 10 >/dev/null
    info "$X1" $YW "$Z1"
    expect_value "$EXPECT_WORLD_LIGHT" active "H"
    expect_aspect up light "H"
    lamp_lit $((X1 + 1)) $YW "$Z1" "H"

    echo "== I: the same aspect reads 0 once the glowstone becomes stone =="
    set_block_in minecraft:overworld "$X1" 103 "$Z1" "minecraft:stone"
    ctl wait 10 >/dev/null
    info "$X1" $YW "$Z1"
    expect_value 0 inactive "I"
    lamp_dark $((X1 + 1)) $YW "$Z1" "I"

    echo "== J: weather_raining: clear is 0, rain is 15 =="
    aspect up weather_raining
    weather clear
    info "$X1" $YW "$Z1"
    expect_value 0 inactive "J (clear)"
    lamp_dark $((X1 + 1)) $YW "$Z1" "J (clear)"
    weather "rain 1000000"
    info "$X1" $YW "$Z1"
    expect_value 15 active "J (rain)"
    lamp_lit $((X1 + 1)) $YW "$Z1" "J (rain)"

    echo "== K: weather_clear: clear is 15, rain is 0 =="
    aspect up weather_clear
    weather clear
    info "$X1" $YW "$Z1"
    expect_value 15 active "K (clear)"
    lamp_lit $((X1 + 1)) $YW "$Z1" "K (clear)"
    weather "rain 1000000"
    info "$X1" $YW "$Z1"
    expect_value 0 inactive "K (rain)"
    lamp_dark $((X1 + 1)) $YW "$Z1" "K (rain)"

    echo "== L: weather_thunder: thunder is 15, clear is 0 =="
    aspect up weather_thunder
    weather "thunder 1000000"
    info "$X1" $YW "$Z1"
    expect_value 15 active "L (thunder)"
    lamp_lit $((X1 + 1)) $YW "$Z1" "L (thunder)"
    weather clear
    info "$X1" $YW "$Z1"
    expect_value 0 inactive "L (clear)"
    lamp_dark $((X1 + 1)) $YW "$Z1" "L (clear)"

    echo "== M: isday: noon is 15, midnight is 0 =="
    aspect up isday
    weather clear
    time_set noon
    info "$X1" $YW "$Z1"
    expect_value 15 active "M (noon)"
    lamp_lit $((X1 + 1)) $YW "$Z1" "M (noon)"
    time_set midnight
    info "$X1" $YW "$Z1"
    expect_value 0 inactive "M (midnight)"
    lamp_dark $((X1 + 1)) $YW "$Z1" "M (midnight)"

    echo "== N: isnight: midnight is 15, noon is 0 =="
    aspect up isnight
    time_set midnight
    info "$X1" $YW "$Z1"
    expect_value 15 active "N (midnight)"
    lamp_lit $((X1 + 1)) $YW "$Z1" "N (midnight)"
    time_set noon
    info "$X1" $YW "$Z1"
    expect_value 0 inactive "N (noon)"
    lamp_dark $((X1 + 1)) $YW "$Z1" "N (noon)"

    echo "== O: right-clicking a cable face with the World Reader places it =="
    set_block_in minecraft:overworld "$X1" $YW $((Z1 + 3)) "integrateddynamics:cable"
    ctl command "item replace entity @s hotbar.0 with integrateddynamics:part_world_reader 2" >/dev/null
    ctl select 1 >/dev/null
    ctl wait 4 >/dev/null
    ctl command "tp @s $((X1 - 1)).5 $Y $((Z1 + 3)).5" >/dev/null
    ctl wait 8 >/dev/null
    aim=$(ctl look-at-block "$X1" $YW $((Z1 + 3)) west 2>&1) \
        || fail "cannot aim at the second cable's west face: $(tr -d '\n' <<<"$aim" | cut -c1-300)"
    reply=$(ctl interact "$X1" $YW $((Z1 + 3)) west 2>&1) \
        || fail "ctl interact on the second cable's west face failed: $(tr -d '\n' <<<"$reply" | cut -c1-300)"
    ctl wait 8 >/dev/null
    say "integrateddynamics info $X1 $YW $((Z1 + 3))"
    ctl wait 12 >/dev/null
    parts=$(newest 'parts ')
    echo "  $parts" >&2
    case "$parts" in
        *west=world_reader*) ;;
        *) fail "the right click did not place the world reader on the west face: ${parts:-no reply}" ;;
    esac
    local aspect_line
    aspect_line=$(newest 'aspects ')
    echo "  ${aspect_line:-no aspects line}" >&2
    case "$aspect_line" in
        *west=light*) ;;
        *) fail "a placed world reader did not report its default light aspect: ${aspect_line:-no reply}" ;;
    esac

    echo "== P: an unknown aspect and a face without a World Reader are refused =="
    say "integrateddynamics aspect $X1 $YW $((Z1 + 3)) west teleport"
    ctl wait 12 >/dev/null
    local refusal
    refusal=$(newest 'unknown aspect')
    case "$refusal" in
        *unknown\ aspect*) echo "  $refusal" >&2 ;;
        *) fail "an unknown aspect was not refused: ${refusal:-no reply}" ;;
    esac
    say "integrateddynamics aspect $X1 $YW $((Z1 + 3)) east light"
    ctl wait 12 >/dev/null
    refusal=$(newest 'no World Reader')
    case "$refusal" in
        *"no World Reader on the east side"*) echo "  $refusal" >&2 ;;
        *) fail "an aspect on a face without a world reader was not refused: ${refusal:-no reply}" ;;
    esac
}

launch
run

echo "PASS: a Redstone Reader and a World Reader fed a Logic Cable network and a Redstone Writer emitted the strongest reading; a Redstone Reader fed a cable network lit a lamp at level ${EXPECT_A}; the World Reader read light ${EXPECT_WORLD_LIGHT}, rain/clear/thunder, day and night, a placed world-reader item and its default aspect, and the two refusals"
