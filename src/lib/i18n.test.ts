import { describe, expect, it } from 'vitest';

import { localizeAppError, translate } from './i18n';

describe('i18n', () => {
  it('translates stable backend error codes for the English UI', () => {
    expect(localizeAppError('en', { code: 'crop_outside_video' })).toBe(
      'The crop area extends outside the video frame.',
    );
    expect(
      localizeAppError('en', { code: 'export_process_failed', detail: 'exit code 1' }),
    ).toBe(
      'The export process did not complete. (exit code 1)',
    );
  });

  it('uses the same code for Japanese errors and interpolates translated UI values', () => {
    expect(localizeAppError('ja', { code: 'destination_folder_missing' })).toBe(
      '保存先フォルダーが存在しません。',
    );
    expect(translate('en', 'folderPosition', { current: 2, total: 5 })).toBe('2 / 5');
  });
});
