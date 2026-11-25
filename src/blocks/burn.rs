use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::clock::now_secs;
use crate::config::BlockConfig;
use crate::fsutil::{cache_dir, sanitize, write_atomic};
use crate::style::*;

use super::RenderContext;

// Renders happen ~1/s. Collapsing samples closer than this bounds the buffer
// and the per-render write without losing window resolution.
const MIN_SAMPLE_STEP: i64 = 5;
// Below this span a rate is dominated by sampling jitter, so hide instead.
const MIN_SPAN: i64 = 10;
const STALE_FILE_AGE: i64 = 24 * 60 * 60;
// Bounds `now - window` so a huge configured window can't overflow i64.
// Also bounds the buffer each render rewrites: ~1 sample per 5s.
const MAX_WINDOW: u64 = 60 * 60;
// A window shorter than a few MIN_SPANs can never hold a measurable rate.
const MIN_WINDOW: u64 = 3 * MIN_SPAN as u64;
// Open tabs render at least this often with refreshInterval or while working;
// a buffer untouched for longer belongs to a closed or idle tab.
const SESSION_GONE_AFTER: i64 = 30;

#[derive(Serialize, Deserialize, Clone)]
pub struct Sample {
    pub ts: i64,
    pub tokens: u64,
    pub cost: f64,
    pub pct5h: Option<f64>,
    pub resets_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Buffer {
    #[serde(default)]
    pub project: String,
    #[serde(default)]
    pub samples: Vec<Sample>,
    #[serde(default)]
    pub last_turn_tokens: u64,
    #[serde(default)]
    pub updated_at: i64,
    // Transcript the offset below belongs to; a different path restarts tailing.
    #[serde(default)]
    pub transcript_path: String,
    // Byte offset consumed in the transcript so far (None until first seen, then
    // primed to EOF so we measure burn forward instead of re-parsing history).
    #[serde(default)]
    pub transcript_offset: Option<u64>,
    #[serde(default)]
    pub transcript_tokens: u64,
    // Claude Code writes one transcript line per content block, each repeating
    // the response's full usage, so only the first line of a message id counts.
    // A few ids reappear after other lines, hence a short history, not one id.
    #[serde(default)]
    pub recent_message_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct NotifyState {
    #[serde(default)]
    pub last_notified_at: i64,
}

pub struct Projection {
    pub eta_to_100: i64,
    pub on_pace_to_exceed: bool,
}

// ── pure helpers (unit-tested, no IO) ───────────────────────────────────────

/// Token/cost velocity (per second) for one session's samples over the window.
/// Returns None on warm-up (<2 in-window samples, span < MIN_SPAN) or idle
/// (no positive movement), which is how the block hides itself.
pub fn session_rate(samples: &[Sample], now: i64, window: u64) -> Option<(f64, f64)> {
    let cutoff = now - window as i64;
    let win: Vec<&Sample> = samples.iter().filter(|s| s.ts >= cutoff).collect();
    if win.len() < 2 {
        return None;
    }
    let first = win.first()?;
    let last = win.last()?;
    let elapsed = last.ts - first.ts;
    if elapsed < MIN_SPAN {
        return None;
    }
    let d_tok = last.tokens.saturating_sub(first.tokens);
    let d_cost = (last.cost - first.cost).max(0.0);
    if d_tok == 0 && d_cost == 0.0 {
        return None;
    }
    Some((d_tok as f64 / elapsed as f64, d_cost / elapsed as f64))
}

/// 5h-bucket prediction from `used_percentage` velocity. Account-wide because
/// the percentage reflects the shared bucket, not this session's tokens.
pub fn project_pct(samples: &[Sample], now: i64, window: u64) -> Option<Projection> {
    let cutoff = now - window as i64;
    let win: Vec<&Sample> = samples
        .iter()
        .filter(|s| s.ts >= cutoff && s.pct5h.is_some())
        .collect();
    if win.len() < 2 {
        return None;
    }
    let first = win.first()?;
    let last = win.last()?;
    let elapsed = last.ts - first.ts;
    // used_percentage arrives in whole points, so over a short span one step
    // reads as a steep climb (1 point in 15s projects the cap in ~15 min).
    // Project only once the samples cover half the window.
    if elapsed < MIN_SPAN || elapsed < (window / 2) as i64 {
        return None;
    }
    let p0 = first.pct5h?;
    let p1 = last.pct5h?;
    let pct_rate = (p1 - p0) / elapsed as f64;
    if pct_rate <= 0.0 {
        return None;
    }
    let resets_at = last.resets_at?;
    let eta_to_100 = ((100.0 - p1) / pct_rate) as i64;
    let time_to_reset = resets_at.saturating_sub(now);
    Some(Projection {
        eta_to_100,
        on_pace_to_exceed: eta_to_100 < time_to_reset,
    })
}

/// At most one notification per cooldown across all tabs (the state file is
/// shared). Only an on-pace tab touches the state: tabs see different pace, and
/// letting an idle tab reset an "alerting" flag made the active tab re-fire on
/// every render.
pub fn should_notify(state: &NotifyState, on_pace: bool, now: i64, cooldown: u64) -> bool {
    on_pace && now.saturating_sub(state.last_notified_at) >= cooldown as i64
}

/// Keeps samples at least MIN_SAMPLE_STEP apart, plus a moving head. The head
/// is replaced only while it is within the step of the sample before it;
/// comparing against the head itself would replace it forever at ~1 render/s,
/// leaving one sample and no rate.
fn append_throttled(samples: &mut Vec<Sample>, s: Sample) {
    let n = samples.len();
    if n >= 2 && s.ts - samples[n - 2].ts < MIN_SAMPLE_STEP {
        samples[n - 1] = s;
        return;
    }
    samples.push(s);
}

/// Keep in-window samples plus the last one before the window. The rate math
/// ignores that one, but it keeps an idle session at two or more samples, so the
/// block shows a dimmed zero instead of hiding as if warming up.
fn prune(samples: &mut Vec<Sample>, now: i64, window: u64) {
    let cutoff = now - window as i64;
    if let Some(anchor) = samples.iter().rposition(|s| s.ts < cutoff)
        && anchor > 0
    {
        samples.drain(0..anchor);
    }
}

// ── cache IO ─────────────────────────────────────────────────────────────────

fn session_path(session_id: &str) -> PathBuf {
    cache_dir().join(format!("burn-{}.json", sanitize(session_id)))
}

fn notify_path() -> PathBuf {
    cache_dir().join("burn-notify.json")
}

fn load_buffer(path: &PathBuf) -> Option<Buffer> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

fn burn_files() -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(cache_dir()) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("burn-") && n.ends_with(".json") && n != "burn-notify.json")
                .unwrap_or(false)
        })
        .collect()
}

/// Bounds growth from one-off sessions. Called only when a new session file is
/// created, so it never runs on the hot per-render path.
fn sweep_stale(now: i64) {
    for path in burn_files() {
        // A file that no longer parses is never refreshed, so it goes too.
        let stale = load_buffer(&path).is_none_or(|buf| now - buf.updated_at > STALE_FILE_AGE);
        if stale {
            let _ = fs::remove_file(&path);
        }
    }
    sweep_temp_files();
}

/// Claude Code cancels an in-flight statusline run when a new update arrives;
/// one killed between write and rename leaves a pid-named temp file that no
/// later run reuses.
fn sweep_temp_files() {
    let Ok(entries) = fs::read_dir(cache_dir()) else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name();
        let name = name.to_string_lossy();
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age.as_secs() > 60);
        if name.starts_with('.') && name.ends_with(".tmp") && old {
            let _ = fs::remove_file(e.path());
        }
    }
}

// ── transcript tailing (Tier 1: main-thread tokens, live mid-turn) ───────────

/// Output tokens only. Cache-read is re-counted on every assistant message in a
/// turn (the same ~140k context re-read each iteration), inflating a total ~50×;
/// input_tokens spikes to full-context size on a cache-miss (TTL expiry). Output
/// is the monotonic generation signal that actually tracks burn.
fn usage_tokens(u: &serde_json::Value) -> u64 {
    u.get("output_tokens").and_then(|x| x.as_u64()).unwrap_or(0)
}

/// Totals from the complete (newline-terminated) lines in `data`, counting each
/// assistant message id once. A half-written trailing line is left unconsumed
/// so the next render re-reads it once complete. Works on bytes so a tail cut
/// inside a multibyte character never fails the read.
struct Tally {
    tokens: u64,
    /// Output of the newest response seen, for the per-response unit.
    last_response: Option<u64>,
    consumed: usize,
}

// Real transcripts showed ids reappearing within a few messages; this is ample.
const RECENT_IDS: usize = 32;

fn sum_complete_lines(data: &[u8], seen: &mut Vec<String>) -> Tally {
    let mut t = Tally {
        tokens: 0,
        last_response: None,
        consumed: 0,
    };
    let Some(end) = data.iter().rposition(|&b| b == b'\n') else {
        return t;
    };
    t.consumed = end + 1;
    for line in data[..end].split(|&b| b == b'\n') {
        let Ok(v) = serde_json::from_slice::<serde_json::Value>(line) else {
            continue;
        };
        if v.get("type").and_then(|x| x.as_str()) != Some("assistant") {
            continue;
        }
        let Some(msg) = v.get("message") else {
            continue;
        };
        if let Some(id) = msg.get("id").and_then(|x| x.as_str()) {
            if seen.iter().any(|s| s == id) {
                continue;
            }
            if seen.len() >= RECENT_IDS {
                seen.remove(0);
            }
            seen.push(id.to_string());
        }
        if let Some(usage) = msg.get("usage") {
            let out = usage_tokens(usage);
            t.tokens = t.tokens.saturating_add(out);
            t.last_response = Some(out);
        }
    }
    t
}

fn read_from(path: &str, start: u64) -> Option<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = fs::File::open(path).ok()?;
    f.seek(SeekFrom::Start(start)).ok()?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    Some(buf)
}

/// Cumulative main-thread output tokens from the transcript, tailing only new
/// bytes. Returns None when the transcript is absent or unreadable. First sight
/// of a transcript primes the offset to EOF, so a large existing transcript is
/// never fully parsed and history never shows up as a burst.
fn transcript_tokens(buf: &mut Buffer, path: Option<&str>) -> Option<u64> {
    let path = path?;
    let len = fs::metadata(path).ok()?.len();
    let start = match buf.transcript_offset {
        // A rewritten or different transcript: re-prime instead of re-reading
        // history, keeping the running total so the rate doesn't jump.
        Some(o) if o <= len && buf.transcript_path == path => o,
        _ => {
            buf.transcript_path = path.to_string();
            buf.transcript_offset = Some(len);
            return Some(buf.transcript_tokens);
        }
    };
    let data = read_from(path, start)?;
    let t = sum_complete_lines(&data, &mut buf.recent_message_ids);
    buf.transcript_tokens = buf.transcript_tokens.saturating_add(t.tokens);
    buf.transcript_offset = Some(start + t.consumed as u64);
    if let Some(out) = t.last_response {
        buf.last_turn_tokens = out;
    }
    Some(buf.transcript_tokens)
}

// ── notifications (macOS + Linux; other platforms color-only) ────────────────

fn fire_notification(title: &str, body: &str) {
    let mut cmd = if cfg!(target_os = "macos") {
        let mut c = std::process::Command::new("osascript");
        c.arg("-e").arg(format!(
            "display notification {body:?} with title {title:?}"
        ));
        c
    } else if cfg!(target_os = "linux") {
        let mut c = std::process::Command::new("notify-send");
        c.arg(title).arg(body);
        c
    } else {
        return;
    };
    let _ = cmd
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

// ── rate aggregation across scope ────────────────────────────────────────────

fn aggregate_rate(
    scope: &str,
    this_session: &str,
    project: &str,
    now: i64,
    window: u64,
) -> Option<(f64, f64, usize)> {
    let paths: Vec<PathBuf> = if scope == "session" {
        vec![session_path(this_session)]
    } else {
        burn_files()
    };

    let mut tok = 0.0;
    let mut cost = 0.0;
    let mut sessions = 0;
    for path in paths {
        let Some(buf) = load_buffer(&path) else {
            continue;
        };
        if scope == "project" && buf.project != project {
            continue;
        }
        // A closed or suspended tab stops adding samples, and its rate would
        // otherwise hold at its last value for the whole window.
        if now - buf.updated_at > SESSION_GONE_AFTER {
            continue;
        }
        if let Some((t, c)) = session_rate(&buf.samples, now, window) {
            tok += t;
            cost += c;
            sessions += 1;
        }
    }
    if sessions == 0 {
        None
    } else {
        Some((tok, cost, sessions))
    }
}

// ── formatting ───────────────────────────────────────────────────────────────

// From 58 real sessions, output tokens deduped per message: peak 5-minute
// rates had a median of ~3k/min and p90 of ~6.5k/min; per-response output had
// p90 ~1.5k and p99 ~3.7k. Warn at a typical peak, crit at a heavy one.
fn default_thresholds(unit: &str) -> (f64, f64) {
    match unit {
        "tok/s" => (50.0, 100.0),
        "cost" => (0.05, 0.15),
        "tok/turn" => (1_500.0, 4_000.0),
        _ => (3_000.0, 6_000.0), // tok/m
    }
}

/// The displayed magnitude in the configured unit, used for both rendering and
/// warn/crit coloring (thresholds are expressed in the same unit).
fn unit_value(unit: &str, tok_per_sec: f64, cost_per_sec: f64, last_turn_tokens: u64) -> f64 {
    match unit {
        "tok/s" => tok_per_sec,
        "cost" => cost_per_sec * 60.0,
        "tok/turn" => last_turn_tokens as f64,
        _ => tok_per_sec * 60.0, // tok/m
    }
}

fn format_value(unit: &str, value: f64) -> String {
    match unit {
        "tok/s" => format!("↑ {}/s", fmt_tokens(value as u64)),
        "cost" => format!("${value:.2}/m"),
        "tok/turn" => format!("Δ {}/turn", fmt_tokens(value as u64)),
        _ => format!("↑ {}/m", fmt_tokens(value as u64)),
    }
}

// ── render ───────────────────────────────────────────────────────────────────

pub fn render(ctx: &RenderContext, bc: &BlockConfig) -> Option<String> {
    let window = bc.window_secs.unwrap_or(300).clamp(MIN_WINDOW, MAX_WINDOW);
    let unit = bc.unit.as_deref().unwrap_or("tok/m");
    let scope = bc.scope.as_deref().unwrap_or("account");
    let show_projection = bc.show_projection.unwrap_or(true);
    let notify = bc.notify.unwrap_or(false);
    let cooldown = bc.notify_cooldown_secs.unwrap_or(900);
    let now = now_secs();

    let session_id = ctx.status.session_id.as_deref().unwrap_or("default");
    let project = ctx.status.workspace.project_dir.clone();
    let path = session_path(session_id);
    let is_new = !path.exists();

    // 1. update this session's buffer
    let mut buf = load_buffer(&path).unwrap_or_default();
    buf.project = project.clone();

    // Tokens come only from the transcript: stdin's total_output_tokens is the
    // last response's output, not a running total, so it can't give a rate.
    // Cost is a running total on stdin, so the cost unit works without it.
    let live_tokens = transcript_tokens(&mut buf, ctx.status.transcript_path.as_deref());
    let cur_tokens = live_tokens.unwrap_or(buf.transcript_tokens);
    let cur_cost = ctx.status.total_cost();

    if let Some(last) = buf.samples.last()
        && (cur_tokens < last.tokens || cur_cost + 1e-9 < last.cost)
    {
        // session reset / new session reusing an id: start clean
        buf.samples.clear();
        buf.last_turn_tokens = 0;
    }

    let five = ctx
        .status
        .rate_limits
        .as_ref()
        .and_then(|r| r.five_hour.as_ref());
    let pct5h = five.and_then(|f| f.used_percentage);
    let resets_at = five.and_then(|f| f.resets_at);
    append_throttled(
        &mut buf.samples,
        Sample {
            ts: now,
            tokens: cur_tokens,
            cost: cur_cost,
            pct5h,
            resets_at,
        },
    );
    prune(&mut buf.samples, now, window);
    buf.updated_at = now;
    let last_turn_tokens = buf.last_turn_tokens;
    write_atomic(&path, &buf);
    if is_new {
        sweep_stale(now);
    }

    // 2. rate over the chosen scope (reads from disk so the just-written buffer
    //    is included uniformly)
    let agg = aggregate_rate(scope, session_id, &project, now, window);

    // Need ≥2 samples in the buffer to produce anything (warm-up hide)
    if buf.samples.len() < 2 || (unit != "cost" && live_tokens.is_none()) {
        return None;
    }

    let mut parts: Vec<String> = Vec::new();
    if unit == "tok/turn" {
        if last_turn_tokens > 0 {
            let (warn, crit) = thresholds(bc, unit);
            let v = last_turn_tokens as f64;
            let color = color_for_cost(v, warn, crit);
            parts.push(format!("{color}{}{RST}", format_value(unit, v)));
        } else {
            parts.push(format!("{DIM}{}{RST}", format_value(unit, 0.0)));
        }
    } else if let Some((tok_s, cost_s, sessions)) = agg {
        let (warn, crit) = thresholds(bc, unit);
        let v = unit_value(unit, tok_s, cost_s, last_turn_tokens);
        let color = color_for_cost(v, warn, crit);
        let mut s = format!("{color}{}{RST}", format_value(unit, v));
        if scope == "account" && sessions > 1 {
            s.push_str(&format!(" {DIM}×{sessions}{RST}"));
        }
        parts.push(s);
    } else {
        // Idle: show dimmed zero so the block stays visible
        parts.push(format!("{DIM}{}{RST}", format_value(unit, 0.0)));
    }

    // 3. 5h-cap projection + opt-in notification. Notifications need the
    // projection even when it isn't displayed.
    let projection = if show_projection || notify {
        project_pct(&buf.samples, now, window)
    } else {
        None
    };
    let on_pace = projection.as_ref().is_some_and(|p| p.on_pace_to_exceed);
    let eta = projection.as_ref().map_or(0, |p| p.eta_to_100);
    if show_projection {
        if projection.is_some() {
            let cd = fmt_countdown(eta);
            if on_pace {
                parts.push(format!("{FG_BRED}5h cap ~{cd}{RST}"));
            } else {
                parts.push(format!("{DIM}5h cap ~{cd}{RST}"));
            }
        } else if let Some(resets_at) = resets_at {
            // Idle: pct not rising, but the reset clock still ticks
            let remaining = resets_at.saturating_sub(now);
            if remaining > 0 {
                let cd = fmt_countdown(remaining);
                let pct_label = pct5h.map(|p| format!(" {p:.0}%")).unwrap_or_default();
                parts.push(format!("{DIM}5h resets {cd}{pct_label}{RST}"));
            }
        }
    }
    if notify {
        handle_notify(on_pace, now, cooldown, eta);
    }

    Some(parts.join(&format!(" {DIM}·{RST} ")))
}

fn thresholds(bc: &BlockConfig, unit: &str) -> (f64, f64) {
    let (dw, dc) = default_thresholds(unit);
    (bc.warn.unwrap_or(dw), bc.crit.unwrap_or(dc))
}

fn handle_notify(on_pace: bool, now: i64, cooldown: u64, eta: i64) {
    let path = notify_path();
    let mut state = load_notify_state(&path);
    if should_notify(&state, on_pace, now, cooldown) {
        fire_notification(
            "ccbar — 5h limit",
            &format!(
                "On pace to hit the 5h limit in ~{} (before reset)",
                fmt_countdown(eta)
            ),
        );
        state.last_notified_at = now;
        write_atomic(&path, &state);
    }
}

fn load_notify_state(path: &PathBuf) -> NotifyState {
    load_buffer_as(path).unwrap_or_default()
}

fn load_buffer_as(path: &PathBuf) -> Option<NotifyState> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

/// `ccbar --burn`: print account-wide burn from the cache without stdin or
/// config edits, for ad-hoc debugging.
pub fn run_debug() {
    let cfg = crate::config::load();
    let bc = cfg.blocks.get("burn").cloned().unwrap_or_default();
    let window = bc.window_secs.unwrap_or(300).clamp(MIN_WINDOW, MAX_WINDOW);
    let now = now_secs();

    let Some((tok_s, cost_s, sessions)) = aggregate_rate("account", "", "", now, window) else {
        println!("burn: idle (no movement in the last {window}s across any session)");
        return;
    };
    let tok_m = fmt_tokens((tok_s * 60.0) as u64);
    let cost_m = cost_s * 60.0;
    let mut line = format!("account: {tok_m} tok/m · ${cost_m:.2}/m · {sessions} session(s)");

    // Every tab samples the shared bucket, but idle tabs see a frozen value
    // and project nothing, so take the soonest projection from any tab.
    let soonest = burn_files()
        .into_iter()
        .filter_map(|p| load_buffer(&p))
        .filter_map(|b| project_pct(&b.samples, now, window))
        .min_by_key(|p| p.eta_to_100);
    if let Some(p) = soonest {
        line.push_str(&format!(
            " · 5h cap ~{}{}",
            fmt_countdown(p.eta_to_100),
            if p.on_pace_to_exceed {
                " (before reset!)"
            } else {
                ""
            }
        ));
    }
    println!("{line}");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(ts: i64, tokens: u64, cost: f64) -> Sample {
        Sample {
            ts,
            tokens,
            cost,
            pct5h: None,
            resets_at: None,
        }
    }
    fn sp(ts: i64, pct: f64, resets_at: i64) -> Sample {
        Sample {
            ts,
            tokens: 0,
            cost: 0.0,
            pct5h: Some(pct),
            resets_at: Some(resets_at),
        }
    }

    #[test]
    fn rate_hides_below_two_samples() {
        assert!(session_rate(&[s(0, 100, 0.1)], 100, 300).is_none());
        assert!(session_rate(&[], 100, 300).is_none());
    }

    #[test]
    fn rate_hides_below_min_span() {
        let samples = vec![s(100, 100, 0.1), s(105, 999_999, 9.9)];
        assert!(session_rate(&samples, 105, 300).is_none());
    }

    #[test]
    fn rate_hides_when_idle() {
        let samples = vec![s(0, 5000, 0.5), s(150, 5000, 0.5), s(300, 5000, 0.5)];
        assert!(session_rate(&samples, 300, 300).is_none());
    }

    #[test]
    fn rate_computes_per_second() {
        // 6000 tokens, $0.60 over 300s -> 20 tok/s, $0.002/s
        let samples = vec![s(0, 1000, 0.1), s(300, 7000, 0.7)];
        let (tok_s, cost_s) = session_rate(&samples, 300, 300).unwrap();
        assert!((tok_s - 20.0).abs() < 1e-6);
        assert!((cost_s - 0.002).abs() < 1e-9);
    }

    #[test]
    fn projection_flags_when_eta_before_reset() {
        // 5%/min from 95% => cap in ~60s; reset ~59 min away => hit cap first
        let samples = vec![sp(0, 90.0, 3600), sp(60, 95.0, 3600)];
        let p = project_pct(&samples, 60, 120).unwrap();
        assert!(p.eta_to_100 < 3600 - 60);
        assert!(p.on_pace_to_exceed);
    }

    #[test]
    fn projection_safe_when_eta_after_reset() {
        // 1%/min from 11% => ~89 min to cap; reset only 10 min away => resets first
        let samples = vec![sp(0, 10.0, 660), sp(60, 11.0, 660)];
        let p = project_pct(&samples, 60, 120).unwrap();
        assert!(!p.on_pace_to_exceed);
    }

    #[test]
    fn projection_hides_when_not_rising() {
        let samples = vec![sp(0, 50.0, 600), sp(60, 50.0, 600)];
        assert!(project_pct(&samples, 60, 120).is_none());
    }

    #[test]
    fn projection_waits_for_half_a_window() {
        // One whole-point step soon after the first sample is rounding, not pace.
        let samples = vec![sp(0, 40.0, 3600), sp(15, 41.0, 3600)];
        assert!(project_pct(&samples, 15, 300).is_none());
        let samples = vec![sp(0, 40.0, 3600), sp(150, 45.0, 3600)];
        assert!(project_pct(&samples, 150, 300).is_some());
    }

    #[test]
    fn notify_cooldown() {
        let mut st = NotifyState::default();
        assert!(should_notify(&st, true, 1000, 900));
        st.last_notified_at = 1000;
        assert!(!should_notify(&st, true, 1100, 900)); // within cooldown
        assert!(should_notify(&st, true, 1900, 900)); // cooldown elapsed
        assert!(!should_notify(&st, false, 2000, 900)); // not in danger
    }

    #[test]
    fn idle_tab_does_not_rearm_notification() {
        // Two tabs render alternately each second: one on pace, one idle.
        let mut st = NotifyState::default();
        let mut fired = 0;
        for t in 1000..1040 {
            let on_pace = t % 2 == 0;
            if should_notify(&st, on_pace, t, 900) {
                fired += 1;
                st.last_notified_at = t;
            }
        }
        assert_eq!(fired, 1);
    }

    #[test]
    fn append_collapses_close_samples() {
        let mut v = Vec::new();
        append_throttled(&mut v, s(0, 100, 0.1));
        append_throttled(&mut v, s(2, 200, 0.2));
        append_throttled(&mut v, s(4, 300, 0.3)); // within the step of ts=0 -> replace head
        append_throttled(&mut v, s(10, 400, 0.4));
        assert_eq!(v.iter().map(|x| x.ts).collect::<Vec<_>>(), [0, 4, 10]);
        assert_eq!(v[1].tokens, 300);
    }

    #[test]
    fn rate_appears_at_one_render_per_second() {
        // refreshInterval = 1 and streaming both render about once a second.
        let mut v = Vec::new();
        for t in 0..=60 {
            append_throttled(&mut v, s(t, t as u64 * 100, 0.0));
            prune(&mut v, t, 300);
        }
        assert!(v.windows(2).all(|w| w[1].ts - w[0].ts <= MIN_SAMPLE_STEP));
        let (tok_s, _) = session_rate(&v, 60, 300).expect("rate at 1 Hz");
        assert!((tok_s - 100.0).abs() < 1e-6, "{tok_s}");
    }

    #[test]
    fn real_session_burst_and_idle() {
        // Anonymized cumulative stream from a real high-burn session (see
        // tests/fixtures). Proves bursts register and idle gaps hide, against
        // real token/cost figures over real inter-turn timing.
        let raw = include_str!("../../tests/fixtures/burn_stream.jsonl");
        let pts: Vec<(i64, u64, f64)> = raw
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| {
                let v: serde_json::Value = serde_json::from_str(l).unwrap();
                (
                    v["ts"].as_i64().unwrap(),
                    v["tok"].as_u64().unwrap(),
                    v["cost"].as_f64().unwrap(),
                )
            })
            .collect();

        // value held since the last turn at or before `ts` (what ccbar samples)
        let cum_at = |ts: i64| -> (u64, f64) {
            let mut r = (pts[0].1, pts[0].2);
            for &(t, tk, c) in &pts {
                if t <= ts {
                    r = (tk, c)
                } else {
                    break;
                }
            }
            r
        };
        let build = |end: i64, window: u64| -> Vec<Sample> {
            let mut s = Vec::new();
            let mut t = end - window as i64;
            while t <= end {
                let (tk, c) = cum_at(t);
                s.push(Sample {
                    ts: t,
                    tokens: tk,
                    cost: c,
                    pct5h: None,
                    resets_at: None,
                });
                t += 5;
            }
            s
        };

        let last = pts.last().unwrap().0;
        let mut max_tpm = 0.0_f64;
        let mut end = 300;
        while end <= last {
            if let Some((tok_s, _)) = session_rate(&build(end, 300), end, 300) {
                max_tpm = max_tpm.max(tok_s * 60.0);
            }
            end += 60;
        }
        assert!(
            max_tpm > 30_000.0,
            "expected a burst >30k tok/min, got {max_tpm}"
        );

        // 1200s after the final turn: no movement in window -> hidden
        assert!(session_rate(&build(last + 1200, 300), last + 1200, 300).is_none());
    }

    #[test]
    fn transcript_sums_assistant_output_only() {
        // cache_read/input must be ignored; only output_tokens count
        let data = concat!(
            r#"{"type":"user","message":{"role":"user"}}"#,
            "\n",
            r#"{"type":"assistant","message":{"usage":{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":100,"cache_creation_input_tokens":20}}}"#,
            "\n",
            r#"{"type":"assistant","isSidechain":false,"message":{"usage":{"input_tokens":1,"output_tokens":2,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}}}"#,
            "\n",
        );
        let t = sum_complete_lines(data.as_bytes(), &mut Vec::new());
        assert_eq!(t.tokens, 5 + 2);
        assert_eq!(t.consumed, data.len());
        assert_eq!(t.last_response, Some(2));
    }

    #[test]
    fn transcript_counts_each_message_id_once() {
        // One response split into thinking/text/tool_use lines, each repeating usage.
        let line = |id: &str, out: u64| {
            format!(
                r#"{{"type":"assistant","message":{{"id":"{id}","usage":{{"output_tokens":{out}}}}}}}"#
            )
        };
        let data = [
            line("a", 300),
            line("a", 300),
            line("a", 300),
            line("b", 40),
        ]
        .join("\n")
            + "\n";
        let mut last = Vec::new();
        let t = sum_complete_lines(data.as_bytes(), &mut last);
        assert_eq!(t.tokens, 340);
        assert_eq!(last, ["a", "b"]);
        // A later render seeing more lines of "b" must not count it again.
        let more = line("b", 40) + "\n";
        assert_eq!(sum_complete_lines(more.as_bytes(), &mut last).tokens, 0);
    }

    #[test]
    fn transcript_id_reappearing_later_is_not_recounted() {
        let line = |id: &str, out: u64| {
            format!(
                r#"{{"type":"assistant","message":{{"id":"{id}","usage":{{"output_tokens":{out}}}}}}}"#
            )
        };
        let data = [line("a", 2487), line("b", 100), line("a", 2487)].join("\n") + "\n";
        let t = sum_complete_lines(data.as_bytes(), &mut Vec::new());
        assert_eq!(t.tokens, 2587);
    }

    #[test]
    fn transcript_tail_cut_mid_utf8_is_not_an_error() {
        let full = r#"{"type":"assistant","message":{"id":"a","usage":{"output_tokens":5}}}"#;
        let mut data = format!("{full}\n{{\"text\":\"caf").into_bytes();
        data.push(0xC3); // first byte of 'é', second byte not yet written
        let t = sum_complete_lines(&data, &mut Vec::new());
        assert_eq!(t.tokens, 5);
        assert_eq!(t.consumed, full.len() + 1);
    }

    #[test]
    fn transcript_path_change_reprimes_without_burst() {
        let dir = std::env::temp_dir().join(format!("ccbar-burn-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let line = r#"{"type":"assistant","message":{"id":"x","usage":{"output_tokens":9000}}}"#;
        let a = dir.join("a.jsonl");
        let b = dir.join("b.jsonl");
        fs::write(&a, "").unwrap();
        fs::write(&b, format!("{line}\n")).unwrap();
        let mut buf = Buffer::default();
        assert_eq!(transcript_tokens(&mut buf, a.to_str()), Some(0));
        // b already holds history: switching to it must not add that history.
        assert_eq!(transcript_tokens(&mut buf, b.to_str()), Some(0));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn transcript_leaves_partial_trailing_line_unconsumed() {
        let full =
            r#"{"type":"assistant","message":{"usage":{"input_tokens":10,"output_tokens":5}}}"#;
        let data = format!("{full}\n{{\"type\":\"assist"); // half-written next line
        let t = sum_complete_lines(data.as_bytes(), &mut Vec::new());
        assert_eq!(t.tokens, 5);
        assert_eq!(t.consumed, full.len() + 1); // only the complete line + its newline
    }

    #[test]
    fn prune_keeps_window_anchor() {
        let mut v = vec![
            s(0, 0, 0.0),
            s(50, 10, 0.0),
            s(100, 20, 0.0),
            s(400, 30, 0.0),
        ];
        prune(&mut v, 400, 300); // cutoff 100
        // anchor (ts=50, last before cutoff) kept; ts=0 dropped
        assert_eq!(v.first().unwrap().ts, 50);
        assert_eq!(v.last().unwrap().ts, 400);
    }
}
