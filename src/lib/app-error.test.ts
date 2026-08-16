import { describe, expect, it } from 'vitest';

import { isAppErrorPayload } from './app-error';

describe('app error contract', () => {
  it('accepts a known code with an optional diagnostic detail', () => {
    expect(isAppErrorPayload({ code: 'folder_read_failed' })).toBe(true);
    expect(isAppErrorPayload({ code: 'folder_read_failed', detail: 'access denied' })).toBe(true);
  });

  it('rejects prose, unknown codes, and malformed details', () => {
    expect(isAppErrorPayload('フォルダーを読み取れません。')).toBe(false);
    expect(isAppErrorPayload({ code: 'future_error' })).toBe(false);
    expect(isAppErrorPayload({ code: 'folder_read_failed', detail: 5 })).toBe(false);
  });
});
