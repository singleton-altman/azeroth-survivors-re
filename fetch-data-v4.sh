#!/usr/bin/env bash
# Segmented IPv4-forced downloader for the game data — drop-in replacement for
# scripts/fetch-data.sh on networks where IPv6 blackholes and archive.org
# throttles per-connection (~90 KB/s each; N parallel range requests scale linearly).
# Stall-resistant: a transfer slower than 20 KB/s for 60 s is aborted, and failed
# parts are retried in later passes — safe to re-run at any time; it resumes.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATA_DIR="${WOW_DATA_DIR:-$ROOT/wow-data}"
DATA="$DATA_DIR/Data"
URL="${WOW_DATA_URL:-https://archive.org/download/WoW_1.12.1-5875/1.12.1-5875_enUS.zip}"
ZIP="$DATA_DIR/$(basename "${URL%%\?*}")"
PARTS_DIR="$DATA_DIR/.parts"
SEGMENTS="${SEGMENTS:-24}"
PART_SIZE=67108864  # 64 MB — do not change: on-disk parts are indexed by this size
MAX_PASSES="${MAX_PASSES:-100}"

if [ -f "$DATA/terrain.MPQ" ] && [ -f "$DATA/patch.MPQ" ]; then
    echo "game data already present at $DATA"
    exit 0
fi

mkdir -p "$DATA_DIR" "$PARTS_DIR"

part_path() { printf '%s/%s.part-%05d' "$PARTS_DIR" "$(basename "$ZIP")" "$1"; }
part_want() {
    local start=$(( $1 * PART_SIZE ))
    local end=$(( start + PART_SIZE - 1 ))
    [ "$end" -ge "$SIZE" ] && end=$(( SIZE - 1 ))
    echo $(( end - start + 1 ))
}
part_ok() {
    local out want
    out="$(part_path "$1")"; want="$(part_want "$1")"
    [ -f "$out" ] && [ "$(stat -f%z "$out" 2>/dev/null || stat -c%s "$out")" = "$want" ]
}

# Resolve the redirect chain (archive.org -> dnXXXX.ca.archive.org) with IPv4 forced.
FINAL_URL="$(curl -4 -sIL --max-time 60 -o /dev/null -w '%{url_effective}' "$URL")"
SIZE="$(curl -4 -sIL --max-time 60 "$FINAL_URL" | tr -d '\r' | awk 'tolower($1)=="content-length:"{s=$2} END{print s}')"
[ -n "$SIZE" ] && [ "$SIZE" -gt 0 ] 2>/dev/null || { echo "could not get content-length from $FINAL_URL" >&2; exit 1; }
NPARTS=$(( (SIZE + PART_SIZE - 1) / PART_SIZE ))
echo "target: ~$((SIZE / 1024 / 1024)) MB in $NPARTS parts from:"
echo "  $FINAL_URL"

# curl: --speed-* only sees the body phase (kills true dead body streams; a
# 25 KB/s evening trickle is real progress and survives), --connect-timeout and
# --max-time bound the two phases --speed-* cannot see: connect and the wait
# for a response (a middlebox can hang those indefinitely on this network).
# No curl-internal retries: a zombie part must die fast — the watchdog below
# resets the whole script when nothing progresses, and the outer retry loop
# (or a re-run) respawns everything missing on fresh connections.
CURL_STALL=(-S --speed-limit 20480 --speed-time 90 --connect-timeout 15 --max-time 1800)

# Watchdog: if the parts directory grows <10 MB over any 4-minute window, the
# network is clamped/hung — kill this script (curls die with it) and let the
# caller's retry loop start over with fresh connections.
(
    last=0
    while :; do
        sleep 120
        cur=$(du -sk "$PARTS_DIR" 2>/dev/null | awk '{print $1}')
        [ -z "$cur" ] && cur=0
        if [ "$last" -gt 0 ] && [ $(( cur - last )) -lt 10240 ]; then
            echo "[watchdog] no progress in 4 min — restarting with fresh connections" >&2
            pkill -P "$$" curl 2>/dev/null
            kill -9 "$$" 2>/dev/null
        fi
        last=$cur
    done
) &
watchdog_pid=$!

resolve_url() {
    curl -4 -sIL --max-time 60 -o /dev/null -w '%{url_effective}' "$URL"
}

pass=0
finish() { kill "$watchdog_pid" 2>/dev/null || true; }
trap finish EXIT

while :; do
    pass=$(( pass + 1 ))
    # Wave parallelism: launch up to SEGMENTS missing parts, wait for the wave,
    # repeat — no job-table counting, waves just drain whatever is missing.
    for wave_start in $(seq 0 "$SEGMENTS" $((NPARTS - 1))); do
        # Fresh redirect per wave: archive.org rotates across data nodes
        # (dnNNNNN.ca / iaNNNNNN.us) and a node can turn bad mid-run —
        # the next wave then draws a different one automatically.
        FINAL_URL="$(resolve_url)" || { echo "[pass $pass] resolve failed, next wave retries" >&2; sleep 30; continue; }
        wave_end=$(( wave_start + SEGMENTS - 1 ))
        [ "$wave_end" -ge "$NPARTS" ] && wave_end=$(( NPARTS - 1 ))
        launched=0
        pids=()
        for i in $(seq "$wave_start" "$wave_end"); do
            part_ok "$i" && continue
            start=$(( i * PART_SIZE ))
            end=$(( start + PART_SIZE - 1 ))
            [ "$end" -ge "$SIZE" ] && end=$(( SIZE - 1 ))
            echo "[pass $pass] part $((i + 1))/$NPARTS …"
            curl -4 -s "${CURL_STALL[@]}" -r "$start-$end" -o "$(part_path "$i")" "$FINAL_URL" \
                || { echo "[pass $pass] part $((i + 1)) failed" >&2; rm -f "$(part_path "$i")"; } &
            pids+=("$!")
            launched=$(( launched + 1 ))
        done
        # wait on the wave's PIDs only — a bare `wait` would also block on the
        # never-exiting watchdog subshell.
        [ "$launched" -gt 0 ] && wait "${pids[@]}"
    done
    left=0
    for i in $(seq 0 $((NPARTS - 1))); do part_ok "$i" || left=$(( left + 1 )); done
    echo "[pass $pass] $((NPARTS - left))/$NPARTS parts complete"
    [ "$left" -eq 0 ] && break
    if [ "$pass" -ge "$MAX_PASSES" ]; then
        echo "giving up after $pass passes with $left parts missing" >&2
        exit 1
    fi
    sleep 30
done

echo "assembling parts…"
: > "$ZIP"
for i in $(seq 0 $((NPARTS - 1))); do
    cat "$(part_path "$i")" >> "$ZIP"
done
GOT="$(stat -f%z "$ZIP" 2>/dev/null || stat -c%s "$ZIP")"
[ "$GOT" = "$SIZE" ] || { echo "size mismatch: got $GOT want $SIZE" >&2; exit 1; }
rm -rf "$PARTS_DIR"

echo "extracting MPQs into ${DATA}…"
mkdir -p "$DATA"
unzip -o "$ZIP" 'Data/*.MPQ' -d "$DATA_DIR"
rm -f "$ZIP"

echo "game data ready at $DATA"
