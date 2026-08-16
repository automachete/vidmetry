import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const japaneseText = /[\u3040-\u30ff\u3400-\u9fff々]/u;
const repositoryRoot = process.cwd();

function filesBelow(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    return entry.isDirectory() ? filesBelow(path) : [path];
  });
}

describe('localization boundary', () => {
  it('keeps product-owned Japanese text out of the Rust backend', () => {
    const files = filesBelow(join(repositoryRoot, 'src-tauri', 'src')).filter((path) =>
      path.endsWith('.rs'),
    );

    for (const path of files) {
      expect(readFileSync(path, 'utf8'), path).not.toMatch(japaneseText);
    }
  });

  it('keeps Japanese UI text in the locale resource', () => {
    const files = filesBelow(join(repositoryRoot, 'src')).filter(
      (path) =>
        (path.endsWith('.ts') || path.endsWith('.svelte')) &&
        !path.endsWith('i18n.ts') &&
        !path.endsWith('.test.ts'),
    );

    for (const path of files) {
      expect(readFileSync(path, 'utf8'), path).not.toMatch(japaneseText);
    }
  });
});
