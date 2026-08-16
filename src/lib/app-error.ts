import { appErrorCodes, type AppErrorCode } from './app-error-codes.generated';

export { appErrorCodes, type AppErrorCode };

export interface AppErrorPayload {
  code: AppErrorCode;
  detail?: string;
}

const knownCodes = new Set<string>(appErrorCodes);

export function isAppErrorPayload(value: unknown): value is AppErrorPayload {
  if (typeof value !== 'object' || value === null) return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.code === 'string' &&
    knownCodes.has(candidate.code) &&
    (candidate.detail === undefined || typeof candidate.detail === 'string')
  );
}
