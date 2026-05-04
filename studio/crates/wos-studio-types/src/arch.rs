// Rust guideline compliant 2026-05-04

//! Stage 7 architectural contract: Studio-side port + adapter-seam
//! trait stubs and shared type aliases.
//!
//! These are **contract code** (no implementations). They pin the
//! shapes that Stage 8+ adapters consume so that the
//! reference-architecture spec, the ADR set, and the Rust trait
//! surface stay aligned. See:
//!
//! - [`reference-architecture.md`](../../../specs/reference-architecture.md)
//!   §"Port catalog" + §"External-OSS-adapter seams".
//! - ADR 0086 (parent), 0087 (persistence), 0088 (AI extraction),
//!   0089 (projection target), 0090 (publish/export boundary),
//!   0091 (port/adapter architecture).
//!
//! Per ADR 0091 §2.1 open-question default, the port surface lives
//! inside `wos-studio-types` rather than a dedicated
//! `wos-studio-server-core` crate; promotion to a dedicated crate is
//! revisited only if the trait surface grows past what the boundary
//! guard sustains.

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::Debug;

// =====================================================================
// Type aliases — Stage 7 newtypes for cross-component references.
// =====================================================================

/// Reference to a published projection artifact (e.g., a compiled
/// `$wosWorkflow.json`, a Formspec form artifact). Opaque to the
/// projection-target system; resolved by the emitting adapter.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct ProjectionRef(pub String);

/// Reference to an `ApprovalPackage` — see
/// `studio/specs/review-and-approval.md`.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct ApprovalPackageRef(pub String);

/// Reference to an `ExportBundle` — see
/// `studio/specs/compiler-contract.md` Phase 9 +
/// ADR 0090 §2.1.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct ExportBundleRef(pub String);

/// Reference to an immutable Source Vault blob — content-addressed.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct SourceBlobRef(pub String);

/// Reference to a recorded AI invocation (per ADR 0088 §2.5).
/// Resolves to recorded prompt + retrieval set + output bytes +
/// versioned model/prompt/parser/projection metadata.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct AIInvocationRef(pub String);

/// Multi-signal confidence record per ADR 0088 §2.1.
///
/// **No single signal — least of all the model's self-reported
/// confidence — gates approval alone.** All six fields are present
/// on every AI-extracted candidate; `humanReviewState` ∈
/// {`Approved`, `RevisedThenApproved`} is required before durability.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct ConfidenceRecord {
    pub schema_validation_result: SchemaValidationResult,
    pub citation_support_score: f32,
    pub retrieval_score: f32,
    pub verifier_result: VerifierResult,
    pub risk_tier: RiskTier,
    pub human_review_state: HumanReviewState,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum SchemaValidationResult {
    Passed,
    Failed,
    Recovered,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum VerifierResult {
    Agreed,
    Disagreed,
    Abstained,
    Error,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum RiskTier {
    Low,
    Medium,
    High,
    Block,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum HumanReviewState {
    Pending,
    Approved,
    Rejected,
    RevisedThenApproved,
}

/// AI lineage extension on top of the existing
/// `AuthoringProvenanceRecord` AI subtype (per
/// `studio/specs/authoring-provenance.md` §"AI extraction subtype").
/// Adds the recorded-output replay primitive (ADR 0088 §2.5) and
/// confidence record (ADR 0088 §2.1).
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct AILineageExt {
    pub invocation: AIInvocationRef,
    pub input_hash: String,
    pub output_hash: String,
    pub retrieval_set_hash: String,
    pub validator_verdicts: Vec<String>,
    pub confidence: ConfidenceRecord,
}

// =====================================================================
// Port catalog — Stage 7 trait stubs (no implementations).
// =====================================================================
//
// 16 ports per `studio/specs/reference-architecture.md` §"Port
// catalog". Each port is the abstraction Stage 8+ adapters realize.
// Adapter crates depend on this trait + their substrate library only
// (per ADR 0091 §2.1).

/// Substrate-agnostic error returned by every Studio port.
pub trait StudioPortError: Error + Debug + Send + Sync + 'static {}

// ---- Knowledge layer ports -----------------------------------------

/// Immutable, content-addressed source blob storage. Owner spec:
/// `studio/specs/source-vault.md`.
pub trait SourceVault {
    type Error: StudioPortError;
    fn put(&self, bytes: &[u8]) -> Result<SourceBlobRef, Self::Error>;
    fn get(&self, blob: &SourceBlobRef) -> Result<Vec<u8>, Self::Error>;
}

/// Append-only hash-chained ledger for `AuthoringProvenanceRecord`.
/// Owner spec: `studio/specs/authoring-provenance.md`. Persistence
/// shape per ADR 0087 §2.3.
pub trait AuthoringLedger {
    type Error: StudioPortError;
    type Record;
    type Cursor;
    fn append(&self, record: &Self::Record) -> Result<u64, Self::Error>;
    fn read(&self, cursor: Self::Cursor) -> Result<Vec<Self::Record>, Self::Error>;
    /// Recompute hash chain; fail loud on mismatch.
    fn verify_chain(&self) -> Result<(), Self::Error>;
}

/// Mutable workspace state (the "now" view). Owner spec:
/// `studio/specs/workspace.md`. Rebuildable from the ledger
/// (`SA-MUST-arch-011`).
pub trait WorkingStore {
    type Error: StudioPortError;
}

/// Graph projection of reviewed knowledge. Owner spec:
/// `studio/VISION.md` §9.3 (Policy Knowledge Map).
pub trait PolicyKnowledgeMap {
    type Error: StudioPortError;
}

/// Semantic-retrieval projection (embeddings). New port per
/// reference-architecture spec.
pub trait RetrievalIndex {
    type Error: StudioPortError;
    type Vector;
    type Hit;
    fn query(&self, q: &Self::Vector, top_k: usize) -> Result<Vec<Self::Hit>, Self::Error>;
}

// ---- Authoring layer ports -----------------------------------------

/// Source bytes + content-type → parsed sections. Substrate-specific
/// parsers (PDF, HTML, JSON-LD, Akoma Ntoso, etc.) implement this.
pub trait ParserAdapter {
    type Error: StudioPortError;
    type ParsedSections;
    fn parse(&self, bytes: &[u8], content_type: &str) -> Result<Self::ParsedSections, Self::Error>;
}

/// LLM invocation with structured-output binding. Returns an
/// AI-lineage record per ADR 0088. Implementations record the
/// invocation (prompt/retrieval/output/metadata) for replay per
/// ADR 0088 §2.5.
pub trait ModelAdapter {
    type Error: StudioPortError;
    type Prompt;
    type Output;
    fn invoke(&self, prompt: &Self::Prompt) -> Result<(Self::Output, AILineageExt), Self::Error>;
}

/// Text → embedding vector.
pub trait EmbeddingAdapter {
    type Error: StudioPortError;
    type Vector;
    fn embed(&self, text: &str) -> Result<Self::Vector, Self::Error>;
}

/// Versioned prompt template store with promotion governance. Stage
/// 8 ships a minimal directory-backed registry; production
/// promotion governance is Stage 9+.
pub trait PromptRegistry {
    type Error: StudioPortError;
    type TemplateRef;
    type Template;
    fn get(&self, r: &Self::TemplateRef) -> Result<Self::Template, Self::Error>;
}

// ---- Application boundary ports ------------------------------------

/// OIDC-style identity provider. Resolves sessions; issues per-actor
/// signing keys (per ADR 0087 §2.4).
pub trait IdentityProvider {
    type Error: StudioPortError;
    type Session;
    type Token;
    type Subject;
    fn resolve(&self, token: &Self::Token) -> Result<Self::Session, Self::Error>;
    fn subject(&self, session: &Self::Session) -> Self::Subject;
}

/// Publish-time signing keys. Stage 8 ships file-backed dev-mode;
/// HSM/KMS in Stage 9+ (ADR 0090 §2.2).
pub trait KeyManager {
    type Error: StudioPortError;
    type KeyId;
    type Signature;
    fn sign(&self, key: &Self::KeyId, payload: &[u8]) -> Result<Self::Signature, Self::Error>;
}

/// Async work scheduling for ingestion + AI extraction.
pub trait WorkerQueue {
    type Error: StudioPortError;
    type Job;
    type JobId;
    fn submit(&self, job: Self::Job) -> Result<Self::JobId, Self::Error>;
}

// ---- Validation + Scenario composition (compose existing crates) ---

/// Composes existing `wos-studio-lint` (70 rules S1–S6 per
/// `studio/STUDIO-LINT-MATRIX.md`).
pub trait ValidationRunner {
    type Error: StudioPortError;
    type Workspace;
    type Report;
    fn run(&self, ws: &Self::Workspace) -> Result<Self::Report, Self::Error>;
}

/// Composes existing `wos-studio-scenario` (per
/// `studio/specs/scenario-authoring.md`).
pub trait ScenarioRunner {
    type Error: StudioPortError;
    type Workspace;
    type Report;
    fn run(&self, ws: &Self::Workspace) -> Result<Self::Report, Self::Error>;
}

// ---- Maximalist placeholder ---------------------------------------

/// Streaming corpus subscription (maximalist; not implemented in v1).
/// Named so the architecture stays open to continuous-corpus updates.
pub trait CorpusFeed {
    type Error: StudioPortError;
}

// =====================================================================
// External-OSS-adapter seams — Stage 7 trait stubs.
// =====================================================================
//
// Per `studio/specs/reference-architecture.md`
// §"External-OSS-adapter seams" + ADR 0091 §2.2. External tools attach
// behind these seams as **replaceable reference adapters**, not
// normative dependencies.

/// Vector + graph memory for retrieval-assisted authoring.
/// Reference-adapter candidate: Cognee (prototype only — see
/// governance constraints in ADR 0091 §2.3 and
/// `SA-MUST-arch-032`).
pub trait KnowledgeMemoryAdapter {
    type Error: StudioPortError;
}

/// Source ingestion (corpora, systems-of-record).
/// Reference-adapter candidates: dlt, Airbyte.
pub trait DataConnectorAdapter {
    type Error: StudioPortError;
}

/// Catalog / schema registry for systems-of-record.
/// Reference-adapter candidates: OpenMetadata, DataHub.
pub trait MetadataCatalogAdapter {
    type Error: StudioPortError;
}

/// Data-lineage interop. Reference-adapter candidate: OpenLineage.
pub trait LineageAdapter {
    type Error: StudioPortError;
}

/// Data-contract emission. Reference-adapter candidates: ODCS, Data
/// Contract spec.
pub trait DataContractAdapter {
    type Error: StudioPortError;
}

/// Data-quality checks over ingested sources / projected outputs.
/// Reference-adapter candidates: Great Expectations, Soda.
pub trait QualityCheckAdapter {
    type Error: StudioPortError;
}

// =====================================================================
// Projection target / Export sink — uniform port per ADR 0089.
// =====================================================================

/// Pluggable projection emitter. Every Studio output (WOS workflow,
/// Formspec form, decision artifact, integration binding, scenario
/// suite, approval package, export bundle, future report) implements
/// this trait. See ADR 0089 for the full catalog.
///
/// `ExportSink` (below) is the destination side of the same trait
/// shape (per ADR 0090 §2.3); aliased for expressivity.
pub trait ProjectionTarget {
    type Error: StudioPortError;
    type Knowledge;
    type Intent;
    type Artifact;
    type ValidationReport;

    fn project(
        &self,
        knowledge: &Self::Knowledge,
        intent: &Self::Intent,
    ) -> Result<Self::Artifact, Self::Error>;

    fn validate(&self, artifact: &Self::Artifact) -> Self::ValidationReport;
}

/// Destination for a signed export bundle. Stage 8 ships filesystem;
/// Trellis-network sink in Stage 9+; federation sink in Stage 10+.
/// Per ADR 0090 §2.3, this is the destination view of the same trait
/// shape as `ProjectionTarget`.
pub trait ExportSink {
    type Error: StudioPortError;
    type Bundle;
    type Receipt;
    fn write(&self, bundle: &Self::Bundle) -> Result<Self::Receipt, Self::Error>;
}
