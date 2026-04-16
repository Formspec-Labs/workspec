export function validateAndCast<T>(data: unknown, typeName: string): T {
  if (data === null || data === undefined || typeof data !== 'object') {
    throw new Error(`Invalid ${typeName}: expected non-null object`);
  }
  if (Array.isArray(data) && data.length === 0) {
    throw new Error(`Invalid ${typeName}: expected non-empty object, got empty array`);
  }
  return data as T;
}

export function validateResponse<T>(data: unknown, typeName: string, requiredFields: string[]): T {
  if (data === null || data === undefined || typeof data !== 'object') {
    throw new Error(`Invalid ${typeName}: expected non-null object, got ${data}`);
  }
  const obj = data as Record<string, unknown>;
  for (const field of requiredFields) {
    if (!(field in obj)) {
      throw new Error(`Invalid ${typeName}: missing required field "${field}"`);
    }
  }
  return data as T;
}
