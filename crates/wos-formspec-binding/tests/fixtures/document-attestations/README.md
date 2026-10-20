# Genuine Formspec document evidence

These immutable Responses were produced with Formspec's maintained
`prepareDocumentAttestations` / `finalizeDocumentAttestations` APIs at
`7f0bcdb21e2114171eba0fc5167a366ea57c3f14`, Integrity canonicalization and real
Ed25519 COSE signatures. The opaque and URI variants were signed independently;
no source identity was rewritten after signing. `trusted-test-keys.json` contains
public, reproducible test keys only. Never configure these keys as production
trust. Private fixture keys are deliberately excluded.

The native tests verify these signatures using the maintained Integrity Ring
adapter and actual selected Definition. The principal covers two distinct
document/locale pairs with one act and one whole-Response digest. The witness
uses a different key and an exact intrinsic principal response/signature/digest
tuple. Root and mirror negatives are separate; wrong tuple, foreign source,
legacy principal and missing presentation variants retain genuine signing
where their purpose requires it. Tests also generate fresh ephemeral keys for
native-valid crypto failure, authority-shape, intent-floor and locale ambiguity
controls.

The retained inline source bytes and test witness support bytes explain the
signed digests. Presentation hashes are signed selected test evidence; these
fixtures do not qualify a browser display, email provider, identity provider,
legal authority or deployment. WOS admission and artifact-link assertions use
the real in-memory runtime/store, not a fabricated verification result.
