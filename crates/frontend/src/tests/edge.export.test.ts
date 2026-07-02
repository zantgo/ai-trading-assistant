// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from 'vitest';
import { useEdgeStore } from '../stores/edges.svelte';

describe('TEST-UI: Edge Builder JSON Export', () => {
    afterEach(() => {
        vi.restoreAllMocks();
    });

    it('exportConfig() downloads the draft config as pretty JSON', () => {
        const edge = useEdgeStore();
        edge.draftName = 'My Strategy';

        // Capture the Blob content parts (jsdom's Blob lacks async .text()).
        const parts: string[] = [];
        const RealBlob = globalThis.Blob;
        class CapturingBlob extends RealBlob {
            constructor(p: BlobPart[], opts?: BlobPropertyBag) {
                super(p, opts);
                for (const part of p) parts.push(String(part));
            }
        }
        vi.stubGlobal('Blob', CapturingBlob);
        (URL as unknown as { createObjectURL: (b: Blob) => string }).createObjectURL = () => 'blob:mock';
        (URL as unknown as { revokeObjectURL: (u: string) => void }).revokeObjectURL = () => {};

        let anchor: HTMLAnchorElement | null = null;
        const realCreate = document.createElement.bind(document);
        vi.spyOn(document, 'createElement').mockImplementation((tag: string) => {
            const el = realCreate(tag) as HTMLElement;
            if (tag === 'a') {
                anchor = el as HTMLAnchorElement;
                (el as HTMLAnchorElement).click = () => {};
            }
            return el;
        });

        edge.exportConfig();

        // Filename derives from the draft name.
        expect(anchor).not.toBeNull();
        expect(anchor!.download).toBe('My Strategy.json');

        // Blob content is the serialized EdgeConfig.
        const text = parts.join('');
        const parsed = JSON.parse(text);
        expect(parsed.archetype).toBe('trend_following');
        expect(Array.isArray(parsed.indicators)).toBe(true);
        expect(parsed.indicators.length).toBeGreaterThan(0);
        // Pretty-printed (2-space indent).
        expect(text).toContain('\n  "archetype"');
    });

    it('exportConfig() falls back to a default filename when unnamed', () => {
        const edge = useEdgeStore();
        edge.draftName = '';

        (URL as unknown as { createObjectURL: (b: Blob) => string }).createObjectURL = () => 'blob:mock';
        (URL as unknown as { revokeObjectURL: (u: string) => void }).revokeObjectURL = () => {};

        let anchor: HTMLAnchorElement | null = null;
        const realCreate = document.createElement.bind(document);
        vi.spyOn(document, 'createElement').mockImplementation((tag: string) => {
            const el = realCreate(tag) as HTMLElement;
            if (tag === 'a') {
                anchor = el as HTMLAnchorElement;
                (el as HTMLAnchorElement).click = () => {};
            }
            return el;
        });

        edge.exportConfig();
        expect(anchor!.download).toBe('edge_config.json');
    });
});
