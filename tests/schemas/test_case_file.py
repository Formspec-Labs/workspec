"""First-case boundary checks; source qualification remains source-owner work."""
import copy
import json
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry
from referencing.jsonschema import DRAFT202012

ROOT = Path(__file__).resolve().parents[2]
SCHEMA = json.loads((ROOT / 'schemas/api/case-file.schema.json').read_text())
REGISTRY = Registry().with_resources([
    (value['$id'], DRAFT202012.create_resource(value))
    for path in (ROOT / 'schemas/api').glob('*.schema.json')
    for value in [json.loads(path.read_text())]
])


def validator(name):
    return Draft202012Validator(
        {'$schema': SCHEMA['$schema'], '$id': SCHEMA['$id'],
         '$defs': SCHEMA['$defs'], '$ref': '#/$defs/' + name},
        registry=REGISTRY,
    )


def test_native_documents_retain_owner_authority_without_cloned_fields():
    props = SCHEMA['$defs']['AcceptedIntakeEvidence']['properties']
    for field in ('handoff', 'response', 'validationReport', 'materialHistory'):
        assert 'properties' not in props[field]
        assert props[field]['x-wos']['ownerSchema'].startswith('https://formspec.org/')
    assert 'originalEvaluation' in SCHEMA['$defs']['AcceptedIntakeEvidence']['required']
    assert 'displayedLocale' not in SCHEMA['$defs']['AcceptedIntakeEvidence']['required']


def test_pending_receipt_cannot_claim_case_or_task_identity():
    value = {'handoffId': 'source-id', 'status': 'pending', 'code': 'source-unavailable'}
    validator('IntakeHandoffReceipt').validate(value)
    for field in ('caseLedgerId', 'processId', 'tasks'):
        altered = {**value, field: [] if field == 'tasks' else 'invented'}
        assert not validator('IntakeHandoffReceipt').is_valid(altered)


def test_attachment_descriptor_refuses_storage_or_disclosure_inputs():
    value = {'attachmentId': 'opaque', 'fieldPath': '/photo', 'fileName': 'photo.jpg',
             'mediaType': 'image/jpeg', 'sizeBytes': 42, 'hash': 'sha256:' + 'a' * 64}
    check = validator('AcceptedAttachmentDescriptor')
    check.validate(value)
    for field in ('url', 'storageKey', 'applicantToken', 'caseLedgerId'):
        assert not check.is_valid({**value, field: 'untrusted'})
    for field, bad in [('hash', 'wrong'), ('sizeBytes', -1), ('fieldPath', 'photo')]:
        changed = copy.deepcopy(value)
        changed[field] = bad
        assert not check.is_valid(changed)


def test_artifact_raw_digest_and_definition_semantic_digest_are_distinct_fields():
    evidence = SCHEMA['$defs']['AcceptedIntakeEvidence']['properties']
    artifact = SCHEMA['$defs']['AcceptedAuthoredArtifact']['properties']
    assert 'raw' in artifact['hash']['description']
    assert 'semantic' in evidence['definitionHash']['description']
