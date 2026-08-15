import { describe, expect, it } from 'vitest';

import { localizeRuntimeError, translate } from './i18n';

describe('i18n', () => {
  it('translates runtime validation errors for the English UI', () => {
    expect(localizeRuntimeError('en', '切り取り範囲が動画フレームの外側です。')).toBe(
      'The crop area extends outside the video frame.',
    );
    expect(localizeRuntimeError('en', 'FFmpegが終了コードSome(1)で停止しました。')).toBe(
      'FFmpeg stopped with exit code Some(1).',
    );
  });

  it('keeps Japanese errors and interpolates translated UI values', () => {
    expect(localizeRuntimeError('ja', '保存先フォルダーが存在しません。')).toBe(
      '保存先フォルダーが存在しません。',
    );
    expect(translate('en', 'folderPosition', { current: 2, total: 5 })).toBe('2 / 5');
  });
});
