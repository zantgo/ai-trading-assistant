// Mirrors `IndicatorsView.svelte::stateDisplay()` so the export JSON
// surfaces the same rich lifecycle state labels the screen renders.
// Order of precedence matches the screen (lifecycle map wins, fallback
// to legacy heuristic).

import type {
  FeedState,
  IndicatorDto,
  IndicatorLifecycleStatus,
  IndicatorMeta,
} from '../types';

export type IndicatorLifecycleDisplay = {
  label: string;
  state: 'Live' | 'Loading' | 'Stale' | 'Failed';
  bars_seen: number;
  bars_required: number;
  last_updated_at: number | null;
  last_error: string | null;
  feed_state: FeedState | string | null;
};

/**
 * Signal capability tokens — mirrors the screen-side lookup in
 * `IndicatorsView.svelte::signalCapability` (registry
 * `signal_capability` field: "AlwaysActive" | "Conditional" | "DataOnly").
 * The map here is only a fallback when the caller passes no capability.
 */
const SIGNAL_CAPABILITY: Record<string, 'Conditional' | 'DataOnly' | 'AlwaysActive' | string> = {
  // Audit fix (m9): the map was empty — `capabilityFor()` always returned
  // ''. Keep it keyed by the registry's `signal_capability` tokens so the
  // fallback is truthful when a caller passes no capability.
  AlwaysActive: 'AlwaysActive',
  Conditional: 'Conditional',
  DataOnly: 'DataOnly',
};

function formatStateLabel(raw: string): string {
  if (!raw || raw === '--') return '--';
  if (raw === 'WARMING') return raw;
  return raw.replace(/_/g, ' ');
}

/**
 * Effective lifecycle state, mirroring the screen-side defensive patch in
 * `IndicatorsView.svelte::lifecycleStatus`: when the backend reports a
 * sticky `Loading` state but `bars_seen >= bars_required` (and
 * `bars_required > 0`), the indicator is functionally `Live`.
 */
export function effectiveLifecycleState(
  lc: IndicatorLifecycleStatus,
): 'Live' | 'Loading' {
  if (lc.state === 'Live') return 'Live';
  if (lc.state === 'Loading' && lc.bars_seen >= lc.bars_required && lc.bars_required > 0) {
    return 'Live';
  }
  return 'Loading';
}

function capabilityFor(key: string): string {
  return SIGNAL_CAPABILITY[key] ?? '';
}

/**
 * Build the same lifecycle display the screen renders for an
 * indicator row. Returns null when neither the lifecycle map nor
 * the entry is meaningful — the row stays an empty pill.
 *
 * `capability` (from the registry `signal_capability`) and `pending`
 * (shadow-tick candle in flight — the screen appends " ⦿") are passed in
 * by the export builder so the JSON state strings match the screen 1:1.
 */
export function lifecycleDisplay(
  key: string,
  dto: IndicatorDto | undefined,
  lc: IndicatorLifecycleStatus | undefined,
  capability?: string | null,
  pending?: boolean,
): IndicatorLifecycleDisplay | null {
  if (lc) {
    const feedState: FeedState | string | null =
      (lc as { feed_state?: string }).feed_state ??
      (lc as { feedState?: string }).feedState ?? null;
    if (effectiveLifecycleState(lc) === 'Live') {
      const sl = dto?.state_label;
      if (sl && sl !== 'WARMING') {
        return {
          label: formatStateLabel(sl) + (pending ? ' \u25C9' : ''),
          state: 'Live',
          bars_seen: lc.bars_seen,
          bars_required: lc.bars_required,
          last_updated_at: lc.last_updated_at ?? null,
          last_error: lc.last_error ?? null,
          feed_state: feedState,
        };
      }
      if (feedState === 'WaitingFeed') {
        return {
          label: 'WAITING FEED \u23F3',
          state: 'Live',
          bars_seen: lc.bars_seen,
          bars_required: lc.bars_required,
          last_updated_at: lc.last_updated_at ?? null,
          last_error: lc.last_error ?? null,
          feed_state: feedState,
        };
      }
      const lcSilent = (lc as { silent?: boolean }).silent === true;
      const cap = capability || capabilityFor(key);
      if (lcSilent || cap === 'Conditional' || cap === 'DataOnly') {
        return {
          label: 'SILENT',
          state: 'Live',
          bars_seen: lc.bars_seen,
          bars_required: lc.bars_required,
          last_updated_at: lc.last_updated_at ?? null,
          last_error: lc.last_error ?? null,
          feed_state: feedState,
        };
      }
      return {
        label: 'AWAITING DATA',
        state: 'Live',
        bars_seen: lc.bars_seen,
        bars_required: lc.bars_required,
        last_updated_at: lc.last_updated_at ?? null,
        last_error: lc.last_error ?? null,
        feed_state: feedState,
      };
    }
    if (lc.state === 'Loading') {
      return {
        label: `Warming (${lc.bars_seen}/${lc.bars_required})`,
        state: 'Loading',
        bars_seen: lc.bars_seen,
        bars_required: lc.bars_required,
        last_updated_at: lc.last_updated_at ?? null,
        last_error: lc.last_error ?? null,
        feed_state: feedState,
      };
    }
    return {
      label: lc.state,
      state: lc.state,
      bars_seen: lc.bars_seen,
      bars_required: lc.bars_required,
      last_updated_at: lc.last_updated_at ?? null,
      last_error: lc.last_error ?? null,
      feed_state: feedState,
    };
  }
  if (!dto?.state_label || dto.state_label === '--') return null;
  const cap = capability || capabilityFor(key);
  if (dto.state_label !== 'WARMING') {
    if ((dto.signals?.length ?? 0) === 0 && (cap === 'Conditional' || cap === 'DataOnly')) {
      return {
        label: 'SILENT',
        state: 'Live',
        bars_seen: 0,
        bars_required: 0,
        last_updated_at: null,
        last_error: null,
        feed_state: null,
      };
    }
    return {
      label: formatStateLabel(dto.state_label),
      state: 'Live',
      bars_seen: 0,
      bars_required: 0,
      last_updated_at: null,
      last_error: null,
      feed_state: null,
    };
  }
  return null;
}

export type LifecycleMeta = IndicatorLifecycleStatus & {
  /** Always emit a not_active flag so consumers can reconstruct 8-card layout. */
  not_active: boolean;
};

export function lifecycleMeta(
  lc: IndicatorLifecycleStatus | undefined,
  exists: boolean,
): LifecycleMeta {
  if (!lc) {
    return {
      state: 'Loading',
      bars_seen: 0,
      bars_required: 0,
      stale_threshold_secs: 0,
      not_active: !exists,
    };
  }
  return { ...lc, not_active: false };
}