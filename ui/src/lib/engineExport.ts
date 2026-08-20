// engineExport — shared export envelope for the engine dashboards.
//
// The MME panels already ship per-tab JSON exports; v7.3 extends the same
// contract to TAE / PME / PAE / DIE. Every tab's exporter serializes the
// EXACT state the tab renders — the payload is built from the same
// bindings that paint the screen, so screen and clipboard cannot drift.
//
// Envelope shape:
//   {
//     "schema": "engine-tab-export/v1",
//     "engine": <engine-key>,
//     "tab": <section-key>,
//     "mode": "observe" | "paper" | "live" | null,
//     "exported_at": <epoch ms>,
//     "data": { ... tab-specific visible values ... }
//   }

export type ExportMode = 'observe' | 'paper' | 'live' | null;

export interface EngineExportEnvelope {
    schema: 'engine-tab-export/v1';
    engine: string;
    tab: string;
    mode: ExportMode;
    exported_at: number;
    data: Record<string, unknown>;
}

/** Wrap tab-specific visible values into the canonical envelope + pretty JSON. */
export function buildEngineExport(
    engine: string,
    tab: string,
    mode: ExportMode,
    data: Record<string, unknown>,
): string {
    const envelope: EngineExportEnvelope = {
        schema: 'engine-tab-export/v1',
        engine,
        tab,
        mode,
        exported_at: Date.now(),
        data,
    };
    return JSON.stringify(envelope, null, 2);
}

/** Compact copy of a value (drops undefined, keeps placeholders verbatim). */
export function present(v: unknown): unknown {
    if (v === undefined) return null;
    if (typeof v === 'number' && !Number.isFinite(v)) return null;
    return v;
}

/** Serialize a map of string→string concentrations preserving order. */
export function concMap(m: Record<string, string> | undefined): Record<string, string> | null {
    if (!m || Object.keys(m).length === 0) return null;
    return m;
}
