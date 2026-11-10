#!/usr/bin/env bash
# Two launches of the same binary to isolate the embedded-content failure:
#  1. no --dev-mod  (the bundle's own content, the shipping route)
#  2. --dev-mod content/ (the known-good route the earlier e2e used)
# For each: the log lines about content, the mods page, a setblock with the
# bundle's id and the native command over chat. Prints both, side by side.
#
#   build-slot.sh run --job id-probe --port ports/integrateddynamics --step ctl -- \
#       tools/probe-embedded-load.sh
set -uo pipefail
PORT=$(cd "$(dirname "$0")/.." && pwd)
RES=${RES:-${HORIZON_RES:-${HORIZON_RES:-$HOME/.local/horizon-client/26.3}}}
CTL=${CTL:-${CAMPAIGN_ROOT:-$(cd "$(dirname "$0")/../../.." && pwd)}/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}
BIN_ROOT=${HORIZON_TARGET_DIR:-$PORT/target/horizon-cargo}
GAME=${GAME:-$BIN_ROOT/fast/horizon-client}
SOCK=${SOCK:-/tmp/opencode/integrateddynamics-probe.sock}
OUT=${OUT:-${CAMPAIGN_ROOT:-$(cd "$(dirname "$0")/../../.." && pwd)}/ports/_shared/evidence/integrateddynamics}

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/id-probe.XXXXXX)
export HORIZON_HOME=$SCRATCH/home HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data XDG_CONFIG_HOME=$SCRATCH/config HORIZON_NO_AUDIO=1
mkdir -p "$HORIZON_HOME" "$XDG_CONFIG_HOME"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "${KEEP:-1}" = 1 ]; then echo "kept scratch: $SCRATCH" >&2; else rm -rf "$SCRATCH"; fi
    echo "EXIT:$status"
}
trap cleanup EXIT

leg() { # leg LABEL [--dev-mod PATH]
    local label=$1; shift
    echo "===== $label ====="
    rm -f "$SOCK"
    cd "$SCRATCH"
    local args=(--game "$GAME" --world "IDProbe${label//[^a-zA-Z0-9]/}" --seed 20261105
                --creative --headless --wait --resources "$RES")
    [ $# -eq 0 ] || args+=(--dev-mod "$1")
    "$CTL" --socket "$SOCK" launch "${args[@]}" >/dev/null
    ctl wait-ready 180
    ctl wait 40
    echo "-- cache the launch left behind --"
    ls -la .horizon-cache 2>&1 | head -6
    echo "-- content lines --"
    ctl logs --lines 400 2>&1 | grep -i "content\|not loaded\|integrateddynamics\|mods page" | head -12
    echo "-- setblock integrateddynamics:cable --"
    ctl command "setblock 0 100 0 integrateddynamics:cable" 2>&1
    ctl wait 4 >/dev/null
    ctl messages 3 2>&1
    echo "-- chat /integrateddynamics info --"
    ctl chat "/integrateddynamics info 0 100 0" 2>&1
    ctl wait 10 >/dev/null
    ctl messages 4 2>&1
    echo "-- held item + interact registration --"
    ctl command "give @s integrateddynamics:cable 1" 2>&1
    ctl wait 4 >/dev/null
    ctl messages 3 2>&1
    ctl quit >/dev/null 2>&1 || true
    sleep 2
}

leg embedded
leg devmod "$PORT/content"

cp "$HORIZON_HOME/logs/latest.log" "$OUT/probe-modload-latest.log" 2>/dev/null \
    || echo "no latest.log under $HORIZON_HOME" >&2
echo "PASS: probe finished (see the two legs above)"
