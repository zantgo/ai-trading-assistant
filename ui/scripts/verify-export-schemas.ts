#!/usr/bin/env bun
// verify-export-schemas.ts — CI guard for the per-tab export payload
// architecture.
//
// Runs the per-tab builder unit tests programmatically and reports any
// failures. The pre-existing vitest suite at `ui/src/lib/exportBuilders/`
// already validates every payload shape exhaustively; this script adds
// a CI-friendly entry-point that prints a one-line summary per builder.
//
// Usage:
//   bun run ui/scripts/verify-export-schemas.ts
//
// Exit code 0 = all builders pass; non-zero = at least one failed.

import { spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const uiRoot = resolve(__dirname, '..');

const BUILDERS = [
  { name: 'shared',          file: 'src/lib/exportBuilders/shared.test.ts' },
  { name: 'chartsTab',       file: 'src/lib/exportBuilders/chartsTab.test.ts' },
  { name: 'riskTab',         file: 'src/lib/exportBuilders/riskTab.test.ts' },
  { name: 'opportunityTab',  file: 'src/lib/exportBuilders/opportunityTab.test.ts' },
  { name: 'alignmentTab',    file: 'src/lib/exportBuilders/alignmentTab.test.ts' },
  { name: 'analysisTab',     file: 'src/lib/exportBuilders/analysisTab.test.ts' },
  { name: 'recommendationTab', file: 'src/lib/exportBuilders/recommendationTab.test.ts' },
  { name: 'metricsTab',      file: 'src/lib/exportBuilders/metricsTab.test.ts' },
  { name: 'mtfTab',          file: 'src/lib/exportBuilders/mtfTab.test.ts' },
  { name: 'BottomConsole',   file: 'src/components/BottomConsole.test.ts' },
];

console.log('Per-Tab Export Schema Verification');
console.log('===================================\n');

let totalFailures = 0;

for (const builder of BUILDERS) {
  const result = spawnSync(
    'bun',
    ['run', 'test', '--', builder.file],
    {
      cwd: uiRoot,
      encoding: 'utf8',
      timeout: 60_000,
    },
  );
  if (result.status === 0) {
    const lines = result.stdout.split('\n');
    const testLine = lines.find((l) => l.includes('Tests  '));
    console.log(`  ✓ ${builder.name.padEnd(22)} ${testLine?.trim() ?? 'OK'}`);
  } else {
    totalFailures++;
    console.log(`  ✗ ${builder.name.padEnd(22)} FAILED`);
    const tail = (result.stdout || '') + (result.stderr || '');
    console.log(tail.slice(-2000));
  }
}

console.log('');
if (totalFailures === 0) {
  console.log(`All ${BUILDERS.length} builders passed.`);
  console.log('Each payload matches its rendered panel 1:1.');
  process.exit(0);
} else {
  console.log(`${totalFailures} builder(s) failed — fix before committing.`);
  process.exit(1);
}

