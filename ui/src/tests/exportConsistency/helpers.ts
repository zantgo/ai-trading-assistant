// Shared helpers for the export-consistency harness: render a panel, click
// its EXPORT DATA button, capture the clipboard JSON, and normalize the
// rendered DOM text for cross-checking.

import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, expect } from 'vitest';

export interface ExportCapture {
  /** Normalized text content of the rendered panel. */
  dom: string;
  /** Parsed export payload (the JSON the button copied). */
  payload: any;
  /** Raw JSON text (for substring checks). */
  jsonText: string;
}

afterEach(() => {
  cleanup();
});

/** Collapse whitespace so multiline DOM text is comparable. */
export function norm(s: string): string {
  return s.replace(/\s+/g, ' ').trim();
}

/** Strip HTML tags (interpretation paragraphs carry <strong> markup). */
export function stripTags(s: string): string {
  return s.replace(/<[^>]*>/g, '').replace(/\s+/g, ' ').trim();
}

/**
 * Render `component` (with optional `seed()` mutating the shared store),
 * click its EXPORT DATA button, and return the DOM text + captured JSON.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function renderPanelAndExport(
  component: any,
  props: Record<string, unknown> = {},
  seed?: () => void,
): Promise<ExportCapture> {
  seed?.();
  const writes: string[] = [];
  Object.defineProperty(navigator, 'clipboard', {
    value: {
      writeText: async (t: string) => {
        writes.push(t);
        return true;
      },
    },
    writable: true,
    configurable: true,
  });
  const { container } = render(component, { props });
  const btn = Array.from(container.querySelectorAll('button')).find((b) =>
    (b.textContent ?? '').toUpperCase().includes('EXPORT DATA'),
  );
  if (!btn) throw new Error('EXPORT DATA button not found in rendered panel');
  await fireEvent.click(btn);
  await tick();
  await new Promise((r) => setTimeout(r, 0));
  if (writes.length !== 1) {
    throw new Error(`expected exactly 1 clipboard write, got ${writes.length}`);
  }
  return { dom: norm(container.textContent ?? ''), payload: JSON.parse(writes[0]), jsonText: writes[0] };
}

/** Click a sidebar/selector button in the rendered panel by label text. */
export async function clickButtonByText(container: HTMLElement, text: string): Promise<void> {
  const btn = Array.from(container.querySelectorAll('button')).find((b) =>
    (b.textContent ?? '').includes(text),
  );
  if (!btn) throw new Error(`button with text "${text}" not found`);
  await fireEvent.click(btn);
  await tick();
}

/**
 * Bidirectional consistency assertion:
 *  - `value` must be rendered in the DOM (what the operator sees)
 *  - `value` must also appear verbatim in the exported JSON text
 */
export function expectInDomAndJson(c: ExportCapture, value: string): void {
  expect(c.dom, `DOM must contain "${value}"`).toContain(value);
  expect(c.jsonText, `JSON must contain "${value}"`).toContain(value);
}

/**
 * Unidirectional checks: the JSON must carry a numeric `rawValue` that the
 * screen renders in its formatted form (`displayedForm`).
 */
export function expectJsonNumberRenderedAsDom(c: ExportCapture, displayedForm: string, rawValue: number): void {
  expect(c.dom, `DOM must render "${displayedForm}"`).toContain(displayedForm);
  expect(c.jsonText, `JSON must carry ${rawValue}`).toContain(String(rawValue));
}
