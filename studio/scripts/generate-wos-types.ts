import { compileFromFile } from 'json-schema-to-typescript';
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const SCHEMAS_DIR = path.resolve(__dirname, '../../schemas');
const OUTPUT_DIR = path.resolve(__dirname, '../src/types/wos');

const schemas = [
  { src: 'kernel/wos-kernel.schema.json', name: 'kernel' },
  { src: 'companions/wos-case-instance.schema.json', name: 'case-instance' },
  { src: 'governance/wos-workflow-governance.schema.json', name: 'workflow-governance' },
  { src: 'governance/wos-due-process.schema.json', name: 'due-process' },
  { src: 'governance/wos-assertion-gate.schema.json', name: 'assertion-gate' },
  { src: 'governance/wos-policy-parameters.schema.json', name: 'policy-parameters' },
  { src: 'ai/wos-ai-integration.schema.json', name: 'ai-integration' },
  { src: 'ai/wos-agent-config.schema.json', name: 'agent-config' },
  { src: 'ai/wos-drift-monitor.schema.json', name: 'drift-monitor' },
  { src: 'advanced/wos-advanced.schema.json', name: 'advanced' },
  { src: 'advanced/wos-equity.schema.json', name: 'equity' },
  { src: 'advanced/wos-verification-report.schema.json', name: 'verification-report' },
  { src: 'profiles/wos-integration-profile.schema.json', name: 'integration-profile' },
  { src: 'profiles/wos-semantic-profile.schema.json', name: 'semantic-profile' },
  { src: 'companions/wos-lifecycle-detail.schema.json', name: 'lifecycle-detail' },
  { src: 'kernel/wos-correspondence-metadata.schema.json', name: 'correspondence-metadata' },
  { src: 'sidecars/wos-notification-template.schema.json', name: 'notification-template' },
  { src: 'sidecars/wos-business-calendar.schema.json', name: 'business-calendar' },
  { src: 'assurance/wos-assurance.schema.json', name: 'assurance' },
];

async function generateAll() {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });

  for (const { src, name } of schemas) {
    const schemaPath = path.join(SCHEMAS_DIR, src);
    if (!fs.existsSync(schemaPath)) {
      console.warn(`SKIP ${src} — not found`);
      continue;
    }
    try {
      const ts = await compileFromFile(schemaPath, {
        cwd: SCHEMAS_DIR,
        declareExternallyReferenced: true,
        enableConstEnums: true,
        style: { singleQuote: true, trailingComma: 'all' as any, printWidth: 120 },
      });
      const outPath = path.join(OUTPUT_DIR, `${name}.ts`);
      fs.writeFileSync(outPath, ts);
      console.log(`OK   ${name}.ts`);
    } catch (err: any) {
      console.error(`FAIL ${name}: ${err.message}`);
    }
  }

  const NAMESPACED_MODULES = new Set(['agent-config', 'drift-monitor', 'advanced', 'integration-profile']);

  const barrel = schemas
    .filter(({ name }) => fs.existsSync(path.join(OUTPUT_DIR, `${name}.ts`)))
    .map(({ name }) => {
      if (NAMESPACED_MODULES.has(name)) {
        const pascal = name.split('-').map(w => w[0].toUpperCase() + w.slice(1)).join('');
        return `export * as ${pascal} from './${name}';`;
      }
      return `export * from './${name}';`;
    })
    .join('\n');
  fs.writeFileSync(path.join(OUTPUT_DIR, 'index.ts'), barrel + '\n');
  console.log(`\nBarrel index.ts written (${schemas.length} modules)`);
}

generateAll().catch((err) => {
  console.error(err);
  process.exit(1);
});
