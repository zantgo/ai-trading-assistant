# Interactive Setup CLI — Flow & Rationale

**Version:** 7.1 (2026-08-18) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Audience:** Operators using `execution-daemon setup` for headless / first-boot configuration.

---

## 1. Why a CLI?

The dashboard's Welcome Gate is the canonical setup path for GUI deployments. But two operator
workflows don't have a browser handy:

1. **Headless / cloud** — a fresh EC2 box with no GUI. The operator SSHs in, runs
   `execution-daemon setup`, answers a few prompts, and the daemon boots.
2. **Reproducible ops** — operators want to script the *same* setup the GUI does. The CLI
   reuses the same `config.toml` schema and the same `save_workspace` path the dashboard uses,
   so a CLI-driven setup produces byte-identical `config.toml` files (modulo formatting).

The CLI also gives operators a `--status` subcommand that prints the current snapshot-export
config without touching disk — useful for sanity-checking the daemon's runtime state.

---

## 2. Flow

The interactive flow walks the operator through seven prompts in order. Every prompt is
*non-blocking* — pressing Enter accepts the bracketed default. The flow is deliberately
linear: validation happens at the end (after all inputs are collected), not after each prompt,
because round-tripping with the operator mid-flow is hostile to scripted use.

```
1. Exchange             [hyperliquid | bitget]
2. Trading pair base    [BTC]                       ← validated against live REST ticker
3. Timeframes           [1,2,3,4]                   ← multi-select; default = all 4
4. Per-TF timeframe_secs [60/300/900/3600]          ← one prompt per enabled TF
5. Enable snapshot?     [y/N]
6. Snapshot interval    [60]                        ← only asked if (5) = y
7. Output directory     [./snapshots]               ← only asked if (5) = y
```

After the seventh prompt, the flow renders a summary table (see §3) and asks for confirmation.
On `Y` it writes `config.toml`; on `n` it exits with `config.toml` unchanged.

The CLI then asks **"Start the daemon now (headless mode)? [y/N]"**. On `y` it `exec`s a child
process (`execution-daemon --mode headless`) with the same `--config` flag the operator used.
The child process inherits stdio so its logs are visible in the parent's terminal.

### 2.1 Validation rules

| Input | Rule |
|---|---|
| Exchange | Must match `"hyperliquid"` / `"hl"` / `"bitget"` / `"bg"` (case-insensitive). Anything else is rejected with a retry hint. |
| Pair base | Must be 1–10 chars after `.trim().to_uppercase()`. **Live-validated** against `hyperliquid_rest::symbol_exists` / `bitget_rest::symbol_exists` — same call `registry::add_instance` makes at boot. On failure the operator is asked "Try a different symbol? [y/N]". |
| Timeframe selection | At least one must be selected. Comma-separated 1-based indices; whitespace ignored. |
| Per-TF `timeframe_secs` | Must be in `[10, 86400]`. Out-of-range inputs are rejected with a retry hint. |
| Snapshot interval | Must be in `[5, 3600]`. Same retry-hint pattern. |
| Output directory | No validation — the daemon creates it on startup. |

### 2.2 Non-interactive use

```bash
# Print current status (no writes).
execution-daemon setup --sub status

# Dry-run (show what would be written, write nothing).
execution-daemon setup --dry-run

# Skip the "Start now?" prompt.
execution-daemon setup --auto-start

# Point at a non-default config.toml.
execution-daemon setup --config /path/to/config.toml
```

`--dry-run` is useful for CI / smoke-testing the wiring without mutating state. `--auto-start` is
useful for fully scripted first-boot provisioning (e.g. Ansible / cloud-init user-data).

---

## 3. Summary table

The summary is rendered as fixed-width text after the seventh prompt:

```
──────────────────────────────────────────────
Trading Platform — Setup Summary
──────────────────────────────────────────────
  Exchange              : hyperliquid
  Settlement currency   : USDC
  Trading pair          : BTC-USDC
  Timeframes            :
    - micro  (slot micro) — 60s
    - fast   (slot fast)  — 300s
    - slow   (slot slow)  — 900s
    - macro  (slot macro) — 3600s
  Snapshot export       : ENABLED (every 60s, → ./snapshots)
──────────────────────────────────────────────
```

After this, the CLI asks **"Apply these settings to config.toml? [Y/n]"**. On `N` it exits
without touching the file.

---

## 4. Convergence with the GUI

The CLI and the GUI write the *same* `config.toml` shape:

| Section | Source (CLI) | Source (GUI) |
|---|---|---|
| `[hyperliquid]`, `[bitget]`, `[clock_monitor]`, `[quality]`, `[reconnect]`, `[candle_buffer]` | Preserved verbatim from current `config.toml` | Preserved verbatim from current `config.toml` |
| `[snapshot_export]` | Written from the prompts (5–7) | Written from the modal (Save) |
| `[workspace]` (incl. `instances[]`) | Replaced with the single pair from prompt (2) | Replaced on `POST /api/config` (operator's existing workflow) |

Both paths hydrate the same `SnapshotExportRuntime` in `AppState` at boot. The runtime is the
single source of truth — `GET /api/snapshot-export/status` is what the GUI polls and what the
CLI `--status` prints. Therefore, **whatever the operator configures via either path, the other
path sees the same state**.

---

## 5. Why hand-rolled (vs `inquire` / `dialoguer`)?

We considered adding `inquire = "0.7"` for arrow-key select / multi-select prompts but decided
against it:

- The flow has only 7 prompts. The complexity premium of a new dependency (~30KB, 0 transitive)
  isn't justified.
- `inquire` requires a TTY; in non-interactive contexts (CI, scripts) it falls back to a
  "no-input" error. Our hand-rolled flow reads from stdin and tolerates EOF gracefully
  (returning the default).
- We don't need any of `inquire`'s advanced features (autocomplete, fuzzy search, password
  masking) — the prompts are simple text + multi-select.

If future flows grow past the "simple multi-step wizard" boundary, migrating to `inquire` is a
straightforward refactor (replace `prompt()` with `inquire::Text::new(...).prompt()`).

---

## 6. Future work

- **Bulk-pair setup** — currently single-pair only. Operators with N pairs should hand-edit
  `config.toml` (the schema is well-documented in
  [`01-02-global-architecture.md`](01-02-global-architecture.md)).
- **Profile presets** — `--profile conservative` / `--profile aggressive` could pre-fill the
  timeframe + snapshot-export defaults from a curated preset.
- **Wizard mirror over the existing REST surface** — the three live snapshot-export REST
  endpoints (status / config / run-now) already let a remote operator drive the same flow over an
  SSH-like tunnel; a future JSON-RPC wizard would add a multi-step wrapper on top.
