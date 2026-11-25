#!/usr/bin/env bash
# Demo/replay for the `burn` block. Walks every state without waiting in real
# time, using an isolated cache (XDG_CACHE_HOME) and a scripted clock
# (CCBAR_NOW_SECS). Build first: cargo build
set -euo pipefail

BIN="${CCBAR_BIN:-./target/debug/ccbar}"
[ -x "$BIN" ] || { echo "build first: cargo build (or set CCBAR_BIN)"; exit 1; }

CACHE="$(mktemp -d)"; CONFIG="$(mktemp -d)"
trap 'rm -rf "$CACHE" "$CONFIG"' EXIT
mkdir -p "$CONFIG/ccbar"
cat > "$CONFIG/ccbar/config.toml" <<'EOF'
[[lines]]
blocks = ["burn"]
[blocks.burn]
window_secs = 300
unit = "tok/m"
scope = "account"
show_projection = true
EOF
export XDG_CACHE_HOME="$CACHE" XDG_CONFIG_HOME="$CONFIG"

# burn reads output tokens from the transcript, so the demo appends one
# assistant response per frame to a scratch transcript.
TRANSCRIPT="$CACHE/demo.jsonl"
: > "$TRANSCRIPT"
MSG=0

# args: now new_output_tokens cost used_pct resets_at
frame() {
  if [ "$2" -gt 0 ]; then
    MSG=$((MSG + 1))
    printf '{"type":"assistant","message":{"id":"msg_%d","usage":{"output_tokens":%d}}}\n' "$MSG" "$2" >> "$TRANSCRIPT"
  fi
  printf '{"session_id":"demo","transcript_path":"%s","workspace":{"current_dir":"/tmp","project_dir":"/tmp/demo"},"cost":{"total_cost_usd":%s},"rate_limits":{"five_hour":{"used_percentage":%s,"resets_at":%d}}}' \
    "$TRANSCRIPT" "$3" "$4" "$5" | CCBAR_NOW_SECS="$1" "$BIN"
}
step() {
  local label="$1"; shift
  # Not in a subshell: frame bumps MSG.
  frame "$@" > "$CACHE/out"
  local out; out="$(cat "$CACHE/out")"
  printf '%-34s%s\n' "$label" "${out:-[hidden]}"
}

echo "burn block demo (each line is one render):"
step "t=1000 warm-up (1 sample):"      1000 0     0.05 40 8000
step "t=1015 first rate (burst):"      1015 2500  0.45 47 8000
step "t=1100 rate continues:"          1100 4000  0.95 55 8000
step "t=1400 idle (no movement):"      1400 0     0.95 55 8000
step "t=1580 fresh burst + 5h risk:"   1580 6000  1.40 70 8000
echo
echo "account-wide debug view:"
CCBAR_NOW_SECS=1580 "$BIN" --burn
echo
echo "(a blank render is the block hiding during warm-up)"
