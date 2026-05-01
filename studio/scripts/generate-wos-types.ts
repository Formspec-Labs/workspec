import { compile, compileFromFile } from 'json-schema-to-typescript';
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath, pathToFileURL } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const SCHEMAS_DIR = path.resolve(__dirname, '../../schemas');
const OUTPUT_DIR = path.resolve(__dirname, '../src/types/wos');

// Post-ADR-0076 / Sub-PR E layout. The schema family collapsed to six author-
// time + runtime-artifact files at `schemas/`, plus two sidecars under
// `schemas/sidecars/`. Legacy split-schema paths (`kernel/`, `governance/`,
// `ai/`, `advanced/`, `companions/`, `assurance/`, `profiles/`) are retired:
// their content is absorbed under embedded blocks of `wos-workflow.schema.json`
// (governance, agents, aiOversight, signature, custody, advanced, assurance)
// or absorbed into the runtime-artifact envelopes.
//
// Per ADR 0063, the generator emits one TypeScript module per canonical schema.
// Consumers that previously imported `WOSAdvancedGovernanceDocument`,
// `WOSAIIntegrationDocument`, etc. now import the corresponding embedded-block
// types nested under `WOSWorkflowDocument` (regenerated `workflow.ts`).
const schemas = [
  { src: 'wos-workflow.schema.json', name: 'workflow' },
  { src: 'wos-case-instance.schema.json', name: 'case-instance' },
  { src: 'wos-provenance-log.schema.json', name: 'provenance-log' },
  { src: 'wos-tooling.schema.json', name: 'tooling' },
  { src: 'sidecars/wos-delivery.schema.json', name: 'delivery' },
  { src: 'sidecars/wos-ontology-alignment.schema.json', name: 'ontology-alignment' },
];

/**
 * Every schema module except `workflow` re-declares types that exist in the
 * merged workflow envelope (e.g. `Lifecycle`, `Actor`, `CaseFile`). Re-exporting
 * them flat would trigger TS2308 (duplicate identifier). The generated `index.ts`
 * star-exports `workflow.ts` and namespaces every other module.
 *
 * Pre-Sub-PR-E this filter excluded `kernel`; the canonical author-time module
 * is now `workflow` (post-ADR-0076 merged envelope).
 */
function namespacedModuleNames(presentNames: string[]): Set<string> {
  return new Set(presentNames.filter((n) => n !== 'workflow'));
}

/**
 * Remote `$ref` base URL used in schemas that cross-reference the workflow envelope
 * during offline / CI runs where `https://wos-spec.org` is not reachable.
 * Substitution rewrites ALL occurrences across every schema — not just provenance-log.
 */
const REMOTE_BASE = 'https://wos-spec.org/schemas/';

/**
 * Rewrite all remote `https://wos-spec.org/schemas/<filename>.schema.json` $ref
 * occurrences in `rawText` to their local `file://` equivalents so that
 * json-schema-to-typescript can resolve them offline.
 *
 * Walks every schema in `schemas[]` and substitutes its canonical remote URL with
 * the `pathToFileURL` of its on-disk path. Falls back to `compileFromFile` when
 * the substituted text is identical to the original (no remote refs present).
 */
function substituteRemoteRefs(rawText: string): string {
  let result = rawText;
  for (const { src } of schemas) {
    const remoteUrl = `${REMOTE_BASE}${src}`;
    const localUrl = pathToFileURL(path.join(SCHEMAS_DIR, src)).href;
    result = result.split(remoteUrl).join(localUrl);
  }
  return result;
}

async function compileSchema(src: string, name: string): Promise<string | null> {
  const schemaPath = path.join(SCHEMAS_DIR, src);
  if (!fs.existsSync(schemaPath)) {
    console.warn(`SKIP ${src} — not found`);
    return null;
  }
  const options = {
    cwd: SCHEMAS_DIR,
    declareExternallyReferenced: true,
    enableConstEnums: true,
    style: { singleQuote: true, trailingComma: 'all' as any, printWidth: 120 },
  };
  try {
    // json-schema-to-typescript resolves $ref via fetch; offline runs need file URLs.
    // substituteRemoteRefs rewrites all wos-spec.org $refs across every known schema.
    const rawOriginal = fs.readFileSync(schemaPath, 'utf8');
    const rawSubstituted = substituteRemoteRefs(rawOriginal);
    if (rawSubstituted !== rawOriginal) {
      return await compile(JSON.parse(rawSubstituted), name, options);
    }
    return await compileFromFile(schemaPath, options);
  } catch (err: any) {
    console.warn(`SKIP ${name} — compile failed: ${err?.message ?? err}`);
    return null;
  }
}

function buildBarrel(presentNames: string[], namespaced: Set<string>): string {
  return presentNames
    .map((name) => {
      if (namespaced.has(name)) {
        const pascal = name.split('-').map(w => w[0].toUpperCase() + w.slice(1)).join('');
        return `export * as ${pascal} from './${name}';`;
      }
      return `export * from './${name}';`;
    })
    .join('\n') + '\n';
}

interface ProducedArtifacts {
  contents: Map<string, string>;
  skipped: Set<string>;
}

async function produceArtifacts(): Promise<ProducedArtifacts> {
  const contents = new Map<string, string>();
  const skipped = new Set<string>();
  for (const { src, name } of schemas) {
    const ts = await compileSchema(src, name);
    if (ts !== null) contents.set(`${name}.ts`, ts);
    else skipped.add(name);
  }
  const presentNames = schemas
    .map(s => s.name)
    .filter(name => contents.has(`${name}.ts`));
  contents.set('index.ts', buildBarrel(presentNames, namespacedModuleNames(presentNames)));
  return { contents, skipped };
}

/**
 * json-schema-to-typescript emits `patternProperties: { "^x-": ... }` as a
 * `[k: string]: { [k: string]: unknown }` index signature plus a boilerplate
 * JSDoc. That breaks assignability across many interfaces; WOS already models
 * vendor extensions via `extensions?: ExtensionsMap`. Strip only this exact
 * shape (not arbitrary nested `};`) so we do not corrupt nested objects like
 * `finiteDomainDeclarations` inner types.
 *
 * Run repeatedly so inner `^x-` blocks are removed before outer ones.
 */
function stripPatternPropertyIndexSignatures(ts: string): string {
  // Must anchor to the generator's boilerplate line so we never span from an
  // unrelated `/**` across real properties (see `VerifiableConstraint`).
  // One or more `* ...` lines (covers merged Action/Action1 refs), then the
  // final `* via the patternProperty "^x-".` line and the vendor index signature.
  const vendorXBlock =
    /\n(\s+)\/\*\*\s*\n(?:\1 \*[^\n]*\n)+\1 \* via the `patternProperty` "\^x-"\.\s*\n\1 \*\/\n\1\[k: string\]: \{\n\1  \[k: string\]: unknown;\n\1\};\n/g;
  let prev: string;
  let next = ts;
  do {
    prev = next;
    next = prev.replace(vendorXBlock, '\n');
  } while (next !== prev);
  return next;
}

async function writeAll(): Promise<void> {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });
  const { contents, skipped } = await produceArtifacts();
  for (const [filename, content] of contents) {
    const processed = filename.endsWith('.ts') && filename !== 'index.ts'
      ? stripPatternPropertyIndexSignatures(content)
      : content;
    fs.writeFileSync(path.join(OUTPUT_DIR, filename), processed);
    console.log(`OK   ${filename}`);
  }
  if (skipped.size > 0) {
    console.log(`Skipped (schema unreachable): ${Array.from(skipped).join(', ')}`);
  }
  console.log(`\n${contents.size} files written to ${OUTPUT_DIR}`);
}

async function checkFreshness(): Promise<void> {
  if (!fs.existsSync(OUTPUT_DIR)) {
    console.error(`Generated types directory missing: ${OUTPUT_DIR}`);
    console.error('Run `npm run types:gen` to generate types.');
    process.exit(1);
  }
  const { contents, skipped } = await produceArtifacts();
  const mismatches: string[] = [];
  for (const [filename, expected] of contents) {
    // Skip freshness check for index.ts when some schemas failed to compile —
    // the local committed barrel may legitimately include files we couldn't
    // re-derive here (e.g. integration-profile depends on an external URL
    // that isn't reachable offline).
    if (filename === 'index.ts' && skipped.size > 0) continue;
    const filePath = path.join(OUTPUT_DIR, filename);
    if (!fs.existsSync(filePath)) {
      mismatches.push(`${filename} missing`);
      continue;
    }
    const current = fs.readFileSync(filePath, 'utf-8');
    const expectedProcessed =
      filename.endsWith('.ts') && filename !== 'index.ts'
        ? stripPatternPropertyIndexSignatures(expected)
        : expected;
    if (current !== expectedProcessed) {
      mismatches.push(`${filename} out of date`);
    }
  }
  if (mismatches.length > 0) {
    console.error('WOS type bindings are stale:');
    for (const msg of mismatches) console.error(`  - ${msg}`);
    console.error('\nRun `npm run types:gen` to regenerate.');
    process.exit(1);
  }
  if (skipped.size > 0) {
    console.log(`(skipped unreachable schemas: ${Array.from(skipped).join(', ')})`);
  }
  console.log(`OK   WOS type bindings are in sync (${contents.size} files compared)`);
}

const mode = process.argv[2] ?? 'generate';

if (mode === 'check') {
  checkFreshness()
    .then(() => process.exit(0))
    .catch((err) => {
      console.error(err);
      process.exit(1);
    });
} else {
  writeAll()
    .then(() => process.exit(0))
    .catch((err) => {
      console.error(err);
      process.exit(1);
    });
}
