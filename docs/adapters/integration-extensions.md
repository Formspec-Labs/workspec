# Integration Profile Extensions and Security Considerations (non-normative)

<!-- relocated-from: profiles/integration.md §10 + §11 per ADR 0076 D-8 — vendor-adapter content moved to non-normative `docs/adapters/`. The normative integration content (binding types, CloudEvents extension attributes, contract validation, correlation, idempotency) lives in `specs/kernel/spec.md` §9.2; this document covers extension points and security guidance for vendor adapters. -->

## Extension Points

### 1 Custom Binding Types

The `type` property accepts values prefixed with `x-` for custom integration binding types. Custom bindings MUST follow the common binding property structure (S3.3) but MAY define additional type-specific properties.

### 2 Custom Policy Engine Types

The `engineType` property in policy engine bindings accepts values prefixed with `x-` for custom policy engines.

### 3 Binding-Level Extensions

Every integration binding supports an `extensions` object for custom metadata. Extension property names MUST begin with `x-`.

---

---

## Security Considerations

### 1 External System Trust

Integration bindings invoke external systems that are outside the WOS Processor's trust boundary. The WOS Processor SHOULD:

- Validate TLS certificates for HTTPS endpoints.
- Authenticate to external services using credentials managed outside the Integration Profile Document. Credentials MUST NOT appear in Integration Profile Documents.
- Log all external invocations for audit.

### 2 Policy Engine Trust

Policy engine decisions are authoritative only within their declared scope. The WOS Processor MUST NOT allow a policy engine decision to weaken governance constraints declared in WOS governance documents (Workflow Governance, AI Integration, Advanced Governance). A policy engine can restrict -- never relax.

### 3 Input Injection

FEL expressions in `inputMapping` and tool `arguments` evaluate against case state data. When case state contains untrusted user input, the evaluated values are passed to external systems. The WOS Processor SHOULD sanitize values before constructing command-line arguments (S3.6.1) to prevent injection attacks. Formspec Definition contract validation (S4) provides structural sanitization for HTTP-based integrations.

---