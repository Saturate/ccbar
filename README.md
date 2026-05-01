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

### git-status

Standalone dirty/ahead/behind block (without branch name). Useful if you want
`git-branch` and `git-status` on different lines or with a separator between them.

Most configs should just use `git-branch` which includes status inline.

### model

Model name and context window size.

```
opus 4.6/1M
```

Color by model family:
- Opus — magenta
- Sonnet — cyan
- Haiku — green

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

### tokens

Input and output token counts for the session.

```
45.2k 8.1k
```

- Input in green, output in yellow
- Auto-scales: raw below 1k, `X.Xk` for thousands, `X.XM` for millions

### cost

Session cost in USD.

```toml
[blocks.cost]
warn = 1.0   # yellow above this
crit = 5.0   # red above this
```

```
$0.42    # green (below warn)
$2.50    # yellow (above warn)
$8.12    # red (above crit)
```

### duration

Session duration. Auto-scales from seconds up to days.

```toml
[blocks.duration]
# Default: auto-scaled (7d23h58m12s)
# Override with a format string:
format = "{total_h}h{m:02}m{s:02}s"
```

Default rendering:
- `< 1m` → `42s`
- `< 1h` → `4m12s`
- `< 1d` → `3h26m5s`
- `>= 1d` → `7d23h58m12s`

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

## CLI

```
ccbar              # render statusline (reads JSON from stdin)
ccbar --setup      # add statusLine to ~/.claude/settings.json
ccbar --init       # write default config to ~/.config/ccbar/config.toml
ccbar --validate   # check config syntax, report line/block counts
ccbar --version    # print version
```

## How it works

Claude Code pipes a JSON blob to the statusline command's stdin on every refresh.
ccbar parses it, loads the TOML config, runs each block, joins them with the
separator, and prints the result. Git info is collected via `git` CLI calls from
the working directory.

The binary is ~600 KB with LTO and strip. Startup is ~1-5ms — well within the
1-second refresh budget.

## Future

- GitHub PR status block (via `gh api`)
- Azure DevOps PR/work item/pipeline blocks (via REST API)
- Transcript JSONL parsing (token speed, block timer)
- Anthropic usage API (weekly/monthly actuals)
- Atomic block composition (`parts` override in TOML)
## License

MIT
