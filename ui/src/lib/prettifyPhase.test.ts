// M9 (production audit): the {@html} sinks must escape backend-sourced
// free text before keyword wrapping — a latent stored-XSS hardening test.

import { describe, expect, it } from 'vitest';
import { highlightKeywords, escapeHtml } from './prettifyPhase';

describe('highlightKeywords XSS hardening', () => {
  it('escapes HTML metacharacters in backend text', () => {
    const hostile = 'STRONG market <img src=x onerror=alert(1)> & "quoted"';
    const out = highlightKeywords(hostile);
    expect(out).toContain('&lt;img');
    expect(out).not.toContain('<img');
    expect(out).toContain('&amp;');
    expect(out).toContain('&quot;');
    // The keyword wrap still works post-escape.
    expect(out).toContain('<strong>STRONG</strong>');
  });

  it('escapeHtml round-trips ordinary text unchanged', () => {
    expect(escapeHtml('plain text with numbers 123')).toBe('plain text with numbers 123');
    expect(escapeHtml('a < b & c > d')).toBe('a &lt; b &amp; c &gt; d');
  });
});
