# Plan: Move Live Workspace as Default in GENERAL Mode

## Goal
1. Make "Live Charts" the default sub-tab when clicking GENERAL mode (currently defaults to "Timeframe Settings")
2. Remove the duplicate "Live Workspace" from USER-CONTROLLED mode

## Changes

### 1. `crates/frontend/src/state.svelte.ts` — line 87

Change the `general` default view from `'timeframe_settings'` to `'terminal'`:

```diff
- modeViews: { general: 'timeframe_settings', wizard: 'workflow', ... },
+ modeViews: { general: 'terminal', wizard: 'workflow', ... },
```

### 2. `crates/frontend/src/App.svelte` — lines 80-81

Remove the `terminal` tab entry from `user` mode tabs (the first entry):

```diff
  user: [
-     { view: 'terminal', label: 'Live Workspace', icon: 'trending-up' },
      { view: 'monitor', label: 'State Panel', icon: 'monitor' },
```

## Result
- Clicking **GENERAL** mode now opens **Live Charts** (all charts via LiveTerminal) immediately
- Live Charts remains accessible under GENERAL > rightmost sub-tab
- User can still access Timeframe Settings and Workspace Settings via their sub-tabs
- The duplicate "Live Workspace" is gone from USER-CONTROLLED mode

## Verification
```bash
cd crates/frontend && npm run check
```
