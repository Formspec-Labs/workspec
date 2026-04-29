import Ajv2020, { type ErrorObject, type ValidateFunction } from 'ajv/dist/2020';
import addFormats from 'ajv-formats';
import kernelSchema from '../../../schemas/wos-workflow.schema.json';

export interface KernelValidationIssue {
  severity: 'error' | 'warning';
  category: 'structure' | 'policy' | 'soundness' | 'satisfiability';
  message: string;
  targetId?: string;
}

export interface KernelValidationResult {
  isValid: boolean;
  issues: KernelValidationIssue[];
}

let cachedValidator: ValidateFunction | null = null;

function getValidator(): ValidateFunction {
  if (cachedValidator) return cachedValidator;
  const ajv = new Ajv2020({ allErrors: true, strict: false, allowUnionTypes: true });
  addFormats(ajv);
  cachedValidator = ajv.compile(kernelSchema);
  return cachedValidator;
}

function issueFromAjvError(err: ErrorObject): KernelValidationIssue {
  const instancePath = err.instancePath || '(root)';
  const targetId = extractStateTargetId(err.instancePath);
  return {
    severity: 'error',
    category: 'structure',
    message: `${instancePath}: ${err.message ?? 'invalid'}`,
    ...(targetId ? { targetId } : {}),
  };
}

function extractStateTargetId(instancePath: string): string | undefined {
  const match = instancePath.match(/^\/lifecycle\/states\/([^/]+)/);
  if (!match) return undefined;
  return match[1];
}

export function validateKernelDocument(kernel: unknown): KernelValidationResult {
  const validator = getValidator();
  const ok = validator(kernel);
  if (ok) return { isValid: true, issues: [] };
  const issues = (validator.errors ?? []).map(issueFromAjvError);
  return { isValid: false, issues };
}

export function assertValidKernelDocument(kernel: unknown, context: string): void {
  const result = validateKernelDocument(kernel);
  if (!result.isValid) {
    const summary = result.issues.slice(0, 5).map(i => i.message).join('; ');
    throw new Error(`${context}: kernel validation failed: ${summary}`);
  }
}
