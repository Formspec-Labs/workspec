// Lightweight non-null type guards for fixture imports.
// Real JSON-schema validation lives in wos-kernel-validator.ts — use that
// for any runtime-crossing kernel payload (HTTP responses, socket messages,
// persisted state).

export function validateAndCast<T>(data: unknown, typeName: string): T {
  if (data === null || data === undefined || typeof data !== 'object') {
    throw new Error(`Invalid ${typeName}: expected non-null object`);
  }
  if (Array.isArray(data) && data.length === 0) {
    throw new Error(`Invalid ${typeName}: expected non-empty object, got empty array`);
  }
  return data as T;
}
