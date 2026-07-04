# Plan: Fix all warnings & errors (config 422 + Rust dead_code + Svelte a11y/state)

## Decisions (confirmed with user)
- **W2 (`ai_trigger`)**: remove the unused field for now (do NOT wire up trigger feature yet).
- **E1 (422 / Exponential)**: fix the genuine Exponential-model deserialize bug.

---

## E1 — Config `POST` 422 / `AllocationCurveModel::Exponential`
**Cause:** `PositionScalingPanel.svelte` sends `allocation_curve.model` as a plain string `"Exponential"` plus a sibling `exponent`. Rust `AllocationCurveModel::Exponential { exponent }` (`config/models.rs:402`) is externally-tagged and requires `{"Exponential":{"exponent":N}}` → deserialize failure → 422. Frontend types (`types.ts:650-658`) already model `model` as a string and `exponent` as a separate optional field, so aligning Rust to that shape fixes it with **no frontend change**.

### Rust edits
1. `crates/engine/src/config/models.rs:402` — change `Exponential { exponent: f64 }` → unit variant `Exponential`.
2. `crates/engine/src/config/models.rs` `AllocationCurve` struct (~412) — add:
   ```rust
   #[serde(default = "default_exponent")]
   pub exponent: f64,
   ```
   Add `fn default_exponent() -> f64 { 2.0 }` and `exponent: default_exponent()` in the `Default` impl (~427).
3. `crates/engine/src/profile_evaluation/scoring.rs`:
   - `evaluate_allocation_curve` (165) — add param `exponent: f64`; change arm `Exponential { exponent } =>` (191) to `Exponential =>` and use the new param in `.powf(exponent)` (196).
   - Caller (244) — pass an exponent (model is hardcoded `Stepped` here; pass `2.0`).
   - Tests (452–469) — add the `exponent` arg; change `&AllocationCurveModel::Exponential { exponent: 3.0 }` (469) to `&AllocationCurveModel::Exponential` and pass `3.0`.

### Frontend
- None required. (Optional cleanup: make `PositionScalingPanel.emit()` always include `exponent` for a stable round-trip; not necessary because Rust field has a serde default.)

**Note:** the 422 you actually observed was most likely the stale frontend bundle sending `short_term/medium_term/large_term` against the new `deny_unknown_fields` backend — already resolved by your `destroy && build && run`. This E1 fix covers the remaining real (Exponential) failure.

---

## W2 — Remove unused `ai_trigger` from `PipelineContext`
`registry/pipelines.rs:34` field is set but never read (trigger feature unimplemented).
1. `crates/engine/src/registry/pipelines.rs:34` — remove `pub ai_trigger: AiTriggerConfig,`.
2. `crates/engine/src/registry/pipelines.rs:8` — remove `AiTriggerConfig` from the import list.
3. `crates/engine/src/registry/mod.rs:93-95` — remove `let ai_trigger = pair_cfg.map(|p| p.ai_trigger.clone()).unwrap_or_default();`.
4. `crates/engine/src/registry/mod.rs:146` — remove `ai_trigger,` from the `PipelineContext { .. }` literal.
5. `crates/engine/src/registry/mod.rs:376` — remove `let ai_trigger = pair_cfg.ai_trigger.clone();`.
6. `crates/engine/src/registry/mod.rs:435` — remove `ai_trigger,` from the second `PipelineContext { .. }` literal.
- Leave `automation.rs:140` (`p.ai_trigger.trigger.clone()`) and the `InstanceSpecificConfig.ai_trigger` field untouched (still persisted/loaded).

---

## W1 — Remove unused `micro_latest` + `safety` from `AutomationContextLight`
`automation.rs:331/338` fields set but never read.
1. `crates/engine/src/automation.rs:331` — remove `micro_latest` field; `:338` — remove `safety` field.
2. `ctx_to_clone` (~350 `micro_latest:` and 357 `safety:`) — remove those two assignments.
3. After removal, check for now-unused imports (`MarketSnapshot`, `SafetyManager`) — only remove if unused elsewhere in the file (they are used elsewhere, so likely keep).

---

## W3 — a11y: associate labels with controls (15 warnings)
Give each control an `id` and its `<label>` a matching `for` (Svelte `for=`). Files/lines:
- `components/CommissionCalculator.svelte`: 64, 68, 72
- `components/settings/IndicatorWeightPanel.svelte`: 40
- `components/settings/PositionScalingPanel.svelte`: 40, 50, 54, 61, 67, 76, 81
- `components/settings/TriggerConfigPanel.svelte`: 58, 68, 76, 85

Pattern: `<label for="x-id">…</label>` + add `id="x-id"` to the paired `<input>/<select>`. (Mirror the `fieldId()` helper style already used in `TimeframeSettings.svelte`.)

---

## W4 — `state_referenced_locally` (~19 warnings)
Wrap the one-time prop reads in `untrack` so Svelte knows the capture is intentional.
- Add `import { untrack } from 'svelte';`
- Change e.g. `let basePct = $state(initial?.allocation_curve?.base_allocation_pct ?? 1.0);`
  → `let basePct = $state(untrack(() => initial?.allocation_curve?.base_allocation_pct ?? 1.0));`
Files/lines:
- `IndicatorWeightPanel.svelte`: 21
- `PositionScalingPanel.svelte`: 10–16
- `TriggerConfigPanel.svelte`: 17, 19, 22, 25, 28

---

## I1 — Vite bundle > 500 kB
Informational only; leave as-is (out of scope).

---

## Verification
1. `cargo check -p engine` → 0 warnings.
2. `cd crates/frontend && npm run check` → 0 warnings.
3. `./manage.sh test` → all green (esp. `scoring.rs` allocation-curve tests).
4. Live repro: build + run, then `POST /api/instances/BTC-USDT/config` with a `position_scaling.allocation_curve.model = "Exponential"` payload → expect 200 (was 422).
