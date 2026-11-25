# ccbar

Fast, configurable statusline for Claude Code. Rust binary with TOML block config.

```
~/code/github/myproject main !
opus 4.6/1M │ ctx ━━━┄┄┄┄┄┄┄┄┄ 20% │ 45.2k 8.1k │ $0.42 │ 3m12s │ 2h14m ━━━━┄┄┄┄ 38%
```

## Install

```bash
cargo install ccbar
```

## Setup

```bash
ccbar --setup    # configure Claude Code to use ccbar
ccbar --init     # write default ccbar config (optional — sensible defaults built in)
```

`--setup` adds the `statusLine` entry to `~/.claude/settings.json` automatically.
Restart Claude Code (or open `/hooks`) to pick up the change.

To configure manually instead, add this to your Claude Code `settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "ccbar",
    "refreshInterval": 1
  }
}
```

## Config

Config lives at `~/.config/ccbar/config.toml`. Lines are rendered top-to-bottom,
blocks left-to-right within each line.

```toml
[[lines]]
blocks = ["dir", "git-branch"]

[[lines]]
blocks = ["model", "context-bar", "tokens", "cost", "duration", "rate-limit"]
```

Add as many `[[lines]]` as you want. Reorder blocks freely. Remove blocks you
don't need — if a block has no data, it's hidden automatically.

### Separator

```toml
[separator]
char = " │ "
style = "dim"
```

## Parts (sub-block composition)

Any multi-part block supports filtering and reordering via `parts`:

```toml
[blocks.git-branch]
parts = ["branch", "dirty"]   # drops icon and ahead-behind

[blocks.context-bar]
parts = ["bar", "pct"]        # drops the 'ctx' label

[blocks.model]
parts = ["name"]              # drops /1M context size
```

Omit `parts` to render all parts in default order. Order in `parts` controls
render order. See each block below for available part names.

## Colors

Override any part's color using a `[blocks.<name>.colors]` table:

```toml
[blocks.duration.colors]
days = "cyan"
hours = "magenta"
minutes = "yellow"
seconds = "green"

[blocks.git-branch.colors]
branch = "blue"
dirty = "red"
```

Available colors: `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`,
`bright-red`, `bright-green`, `bright-yellow`, `bright-blue`, `bright-magenta`,
`bright-cyan`, `bright-white`, `dim`.

Omit `colors` to use built-in defaults.

## Blocks

### dir

Displays the current working directory with `~` abbreviation.

```toml
[blocks.dir]
segments = 0              # 0 = full path, N = last N path segments
abbreviate_home = true    # replace $HOME with ~
```

If `current_dir` differs from `project_dir`, shows both:
`~/project > ~/other/dir`

If inside a subdirectory of the project:
`~/project/src/components`

### git-branch

Git branch name with inline dirty flag and ahead/behind counts.

```
 main !↑2↓1
```

- Branch name in magenta
- `!` in yellow when working tree is dirty
- `↑N` / `↓N` in cyan for commits ahead/behind upstream

Hidden when not in a git repo.

Parts: `icon`, `branch`, `dirty`, `ahead-behind`

### git-status

Standalone dirty/ahead/behind block (without branch name). Useful if you want
`git-branch` and `git-status` on different lines or with a separator between them.

Most configs should just use `git-branch` which includes status inline.

Parts: `dirty`, `ahead-behind`

### model

Model name and context window size.

```
opus 4.6/1M
```

Color by model family:
- Opus — magenta
- Fable — yellow
- Sonnet — cyan
- Haiku — green

Parts: `name`, `context-size`

### context-bar

Visual progress bar showing context window utilization.

```toml
[blocks.context-bar]
width = 12                # bar width in characters
thresholds = [50, 75, 90] # color shift points (green → yellow → orange → red)
```

```
ctx ━━━┄┄┄┄┄┄┄┄┄ 20%
```

Parts: `label`, `bar`, `pct`

### tokens

Input and output token counts for the session.

```
45.2k 8.1k
```

- Input in green, output in yellow
- Auto-scales: raw below 1k, `X.Xk` for thousands, `X.XM` for millions

Parts: `input`, `output`

### cost

Session cost in USD.

```toml
[blocks.cost]
warn = 1.0        # yellow above this
crit = 5.0        # red above this
currency = "DKK"  # any ISO 4217 code (default: USD)
```

```
$0.42     # USD (default)
kr 2.88   # DKK
€0.38     # EUR
£0.33     # GBP
```

Currency conversion uses [frankfurter.app](https://frankfurter.app) (ECB reference
rates). Rates are cached for 24 hours at `~/.cache/ccbar/rates.json`. If the API is
unreachable, cost falls back to USD.

### duration

Session duration. Auto-scales from seconds up to days.

```toml
[blocks.duration]
# Default: auto-scaled (7d23h58m12s)
# Override with a format string:
format = "{total_h}h{m:02}m{s:02}s"
```

Default rendering (each unit has a subtle color):
- `< 1m` → `42s`
- `< 1h` → `4m12s`
- `< 1d` → `3h26m5s`
- `>= 1d` → `7d23h58m12s`

Parts: `days`, `hours`, `minutes`, `seconds`

Using a `format` string disables parts and renders as a single string.

Format tokens:

| Token | Meaning | Example |
|-------|---------|---------|
| `{d}` | Days | `7` |
| `{h}` | Hours (0-23) | `23` |
| `{m}` | Minutes (0-59) | `58` |
| `{s}` | Seconds (0-59) | `12` |
| `{h:02}` | Hours zero-padded | `03` |
| `{m:02}` | Minutes zero-padded | `08` |
| `{s:02}` | Seconds zero-padded | `05` |
| `{total_h}` | Total hours (no day rollover) | `191` |
| `{total_m}` | Total minutes (no hour rollover) | `11518` |
| `{total_s}` | Total seconds (raw) | `691092` |

Examples:

```toml
format = "{d}d {h}h {m}m {s}s"         # 7d 23h 58m 12s
format = "{total_h}:{m:02}:{s:02}"     # 191:58:12
format = "{h}h{m}m"                     # 23h58m  (no seconds)
```

### rate-limit

5-hour and 7-day rate limit bars with optional countdowns.

```toml
[blocks.rate-limit]
show_countdown = true   # show time until reset
show_bar = true         # show progress bar
bar_width = 8           # bar width in characters
```

```
 2h14m ━━━━┄┄┄┄ 38%   1d17h ┄┄┄┄┄┄┄┄ 12%
```

Bar color follows the same thresholds as context-bar (green → yellow → orange → red).

When the 5-hour or 7-day window reaches 100%, a spend-limit segment is appended (countdown to period reset, spend in USD, percentage, period initial). It also shows on its own when no 5h/7d windows are sent. Claude Code only sends `rate_limits.spend_limit` behind a Claude apps gateway with spend limits; the dollar amounts arrive on a ~5 minute poll, so until then only the percentage is shown.

```
 2h13m ━━━━━━━━ 100%   2d7h ━━━━┄┄┄┄ 60%   10d10h ━━━━┄┄┄┄ $314.12/$500 62%/m
```

Claude Code sends `rate_limits` only after a session's first response. Until
then, a tab shows the values another tab last saw, marked with a dim `*`. Those
values come from a shared cache in `~/.cache/ccbar/`, kept separately for each
`CLAUDE_CONFIG_DIR`. Windows past their reset time are dropped, and nothing is
shown once the cached values have gone an hour without changing, since usage on
other devices doesn't reach this cache.

Parts: `5h`, `7d`, `spend`

### burn

Burn **rate** (velocity): how fast tokens or cost are being spent, plus an
optional prediction of when the shared 5-hour limit will be hit. Complements
`rate-limit`, which shows how full the bucket is but not how fast it is filling.

Not in the default lines — add `"burn"` to a `[[lines]]` to show it. It hides
during warm-up (fewer than 2 samples) and shows a dimmed `↑ 0/m` when idle (no
movement over the window).

```toml
[blocks.burn]
window_secs = 300        # rolling window the rate is measured over
unit = "tok/m"           # tok/m | tok/s | cost ($/m) | tok/turn
scope = "account"        # account (sum all sessions) | project | session
show_projection = true   # "5h cap ~38m", red when on pace to exceed before reset
# warn/crit default per unit: tok/m 3000/6000, tok/s 50/100,
# cost 0.05/0.15 ($/m), tok/turn 1500/4000. Set them in the chosen unit.
# warn = 3000
# crit = 6000
notify = false           # opt-in OS notification (macOS/Linux) on 5h-cap risk
notify_cooldown_secs = 900
```

```
↑ 4.2k/m · 5h cap ~38m
```

How it tracks: each session keeps a small rolling sample buffer in
`~/.cache/ccbar/burn-<session_id>.json`. Account scope sums every active
session's windowed delta; a closed session ages out of the window and drops to
zero on its own. Writes are atomic, so multiple windows never corrupt the files.

**Token source.** Token burn counts **output (generation) tokens**. Cache reads
repeat on every message within a turn and input spikes on a cache miss, so
neither reflects real spend; output is the signal that does. Output tokens are
read live from the session transcript (`transcript_path`), so burn tracks
**mid-turn** instead of waiting for the turn to finish. Claude Code writes one
transcript line per content block, each repeating the response's usage, so each
message id is counted once. stdin has no running output total
(`total_output_tokens` is the last response only), so without a transcript the
token units hide. **Cost** comes from stdin's running total, so the `cost` unit
works without a transcript, but it moves only when stdin's `total_cost_usd`
does, not mid-response. `tok/turn` shows the output of the most recent response.

**5h-cap projection** measures the velocity of the account's shared
`used_percentage` bucket as this tab last received it (not this session's
tokens), so an idle tab shows the reset countdown instead; `ccbar --burn` takes
the soonest projection from any tab. It projects when the bucket reaches 100%
and compares that with the reset time. The percentage arrives in whole
points, so it waits until the samples cover half the window before projecting.
With `notify = true`, at most one OS notification fires per cooldown across all
windows while you are on a pace that exhausts the bucket before it resets.
Sessions that haven't rendered for 30 seconds (closed or suspended tabs) drop
out of the account and project totals.

**Limitation.** Subagent (Task) usage lives in separate transcript files and is
**not** counted here, so a deep-research fan-out is still under-reported live.
Catching that needs a hook-based plugin consuming `SubagentStop`, not the
statusline.

Debug the account-wide figure without touching your statusline config:

```
ccbar --burn
# account: 62.0k tok/m · $0.08/m · 3 session(s) · 5h cap ~22m (before reset!)
```

## CLI

```
ccbar              # render statusline (reads JSON from stdin)
ccbar --setup      # add statusLine to ~/.claude/settings.json
ccbar --init       # write default config to ~/.config/ccbar/config.toml
ccbar --validate   # check config syntax, report line/block counts
ccbar --docs       # print config reference (for Claude Code context)
ccbar --burn       # print account-wide burn rate from cache (debug)
ccbar --version    # print version
ccbar --help       # show help
```

## How it works

Claude Code pipes a JSON blob to the statusline command's stdin on every refresh.
ccbar parses it, loads the TOML config, runs each block, joins them with the
separator, and prints the result. Git info is collected via `git` CLI calls from
the working directory.

The binary is ~600 KB with LTO and strip. Startup is ~1-5ms — well within the
1-second refresh budget.

## Configuring with Claude Code

Just paste this into Claude Code:

```
Run `ccbar --docs` and configure my statusline. <describe what you want>
```

For example:
- "Run `ccbar --docs` and configure my statusline. Put cost and duration on line 1, everything else on line 2"
- "Run `ccbar --docs` and configure my statusline. Remove the rate limit bar and make the context bar wider"
- "Run `ccbar --docs` and configure my statusline. Warn me when cost hits $3"

## Future

- GitHub PR status block (via `gh api`)
- Azure DevOps PR/work item/pipeline blocks (via REST API)
- Transcript JSONL parsing (token speed, block timer)
- Anthropic usage API (weekly/monthly actuals)
## License

MIT
