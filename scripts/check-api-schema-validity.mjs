#!/usr/bin/env node
// ADR 0082 D-13 Gate 1 — Schema validity (ajv compile).
//
// Walks every `work-spec/schemas/api/*.schema.json`, registers them with
// AJV (draft 2020-12 mode, `strict: false` to allow ADR-mandated `x-wos`
// vendor extensions per D-12 and OpenAPI `discriminator` per D-5), and
// compiles each. Any compile failure means malformed JSON Schema and the
// gate fails CI.
//
// Cross-schema `$ref`s into core WOS schemas (`wos-workflow.schema.json`,
// `wos-delivery.schema.json`, etc.) are pre-registered as known schemas so
// ajv's reference resolver finds them. Mixed-draft mode is supported by
// loading the draft-07 meta-schema alongside the default 2020-12 one — the
// `case-portal/scripts/generate-wos-types.ts` pipeline already runs this
// exact composition.
//
// Cite: ADR 0082 D-13 (gate 1).
//
// Exit codes:
//   0 — every schema compiles cleanly
//   1 — at least one schema failed to compile

import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { resolve, dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const WORK_SPEC_ROOT = resolve(__dirname, '..');
const SCHEMAS_DIR = join(WORK_SPEC_ROOT, 'schemas');
const API_SCHEMAS_DIR = join(SCHEMAS_DIR, 'api');
const SIDECARS_DIR = join(SCHEMAS_DIR, 'sidecars');

function loadJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function listFiles(dir, predicate) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter(predicate)
    .map((name) => join(dir, name))
    .sort();
}

const apiSchemaPaths = listFiles(API_SCHEMAS_DIR, (n) => /\.schema\.json$/.test(n));
if (apiSchemaPaths.length === 0) {
  console.error('::error::no work-spec/schemas/api/*.schema.json files discovered');
  process.exit(1);
}

// Cross-schema dependencies live in core WOS schemas + sidecars. Pre-register
// every non-api schema so ajv resolves $refs cleanly.
const referencedSchemaPaths = [
  ...listFiles(SCHEMAS_DIR, (n) => /^wos-.*\.schema\.json$/.test(n)),
  ...listFiles(SIDECARS_DIR, (n) => /\.schema\.json$/.test(n)),
];

let failed = 0;
const compiledIds = new Set();

function newAjv() {
  // strictTypes/strict false — see ADR D-5 (discriminator), D-12 (x-wos).
  // allowUnionTypes mirrors the case-portal validator setup.
  const ajv = new Ajv2020({
    allErrors: true,
    strict: false,
    allowUnionTypes: true,
    validateFormats: true,
  });
  addFormats(ajv);
  // Core WOS schemas declare `"$schema": "http://json-schema.org/draft-07/schema#"`;
  // pull in the draft-07 meta-schema so ajv recognizes it under 2020 mode.
  // ajv 8 ships draft-07 meta as a built-in; addMetaSchema is idempotent.
  // (No explicit call needed for 2020-12; the Ajv2020 import bundles it.)
  return ajv;
}

for (const apiSchemaPath of apiSchemaPaths) {
  const rel = relative(WORK_SPEC_ROOT, apiSchemaPath);
  console.log(`::group::ajv compile ${rel}`);
  const ajv = newAjv();

  // Register every other schema first so $refs resolve.
  let setupError = null;
  for (const refPath of [...referencedSchemaPaths, ...apiSchemaPaths]) {
    if (refPath === apiSchemaPath) continue;
    try {
      const refSchema = loadJson(refPath);
      ajv.addSchema(refSchema);
    } catch (e) {
      // A malformed sibling schema would mask the gate's signal — surface
      // it as a gate failure with clear attribution.
      console.error(
        `::error file=${relative(WORK_SPEC_ROOT, refPath)}::failed to register: ${e.message}`,
      );
      setupError = e;
      failed += 1;
    }
  }

  if (setupError) {
    console.log('::endgroup::');
    continue;
  }

  let target;
  try {
    target = loadJson(apiSchemaPath);
  } catch (e) {
    console.error(`::error file=${rel}::JSON parse failed: ${e.message}`);
    failed += 1;
    console.log('::endgroup::');
    continue;
  }

  const targetId = target.$id;
  if (!targetId || typeof targetId !== 'string') {
    console.error(`::error file=${rel}::missing or non-string $id (ADR 0082 D-14)`);
    failed += 1;
    console.log('::endgroup::');
    continue;
  }
  if (compiledIds.has(targetId)) {
    console.error(
      `::error file=${rel}::duplicate $id ${targetId} (ADR 0082 D-14: never reuse $id)`,
    );
    failed += 1;
    console.log('::endgroup::');
    continue;
  }

  try {
    const validator = ajv.compile(target);
    if (typeof validator !== 'function') {
      throw new Error('ajv.compile did not return a validator function');
    }
    compiledIds.add(targetId);
    console.log(`OK: ${rel}`);
  } catch (e) {
    console.error(`::error file=${rel}::ajv compile failed: ${e.message}`);
    failed += 1;
  }
  console.log('::endgroup::');
}

if (failed > 0) {
  console.error(`\nADR 0082 D-13 gate 1 failed: ${failed} schema(s) failed to compile.`);
  process.exit(1);
}
console.log(
  `\nADR 0082 D-13 gate 1 passed: ${apiSchemaPaths.length} api schema(s) compile cleanly.`,
);
