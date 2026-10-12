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


def test_native_codegen_annotation_preserves_json_without_shape_assertions():
    fields = [('AcceptedAuthoredArtifact', 'document'),
              ('AcceptedSignatureVerification', 'details')]
    fields += [('AcceptedIntakeEvidence', field) for field in
               ('handoff', 'response', 'validationReport', 'materialHistory', 'originalEvaluation')]
    for model, field in fields:
        boundary = SCHEMA['$defs'][model]['properties'][field]
        assert boundary['tsType'] == 'unknown'
        check = Draft202012Validator(boundary)
        for value in ({'native': {'answer': None}}, ['native', None], 'opaque', None):
            check.validate(value)
    # Canonical owner validation still supplies each real native document's shape.
    response_schema = json.loads((ROOT.parent / 'formspec/schemas/response.schema.json').read_text())
    owner = Draft202012Validator(response_schema)
    for invalid_native_response in ([], 'opaque', None):
        assert not owner.is_valid(invalid_native_response)


def accepted_receipt(tasks):
    return {'handoffId': 'original-handoff', 'status': 'accepted',
            'caseLedgerId': 'urn:wos:default_case_01m4kv34fgf68924g68cd6bpza',
            'caseReference': '437681102',
            'processId': 'urn:wos:default_process_01m4mey9j0e6stas981f5ey15v',
            'tasks': tasks}


def test_committed_acceptance_allows_deferred_tasks_without_weakening_identity():
    check = validator('IntakeHandoffReceipt')
    value = accepted_receipt([])
    check.validate(value)
    for field in value:
        altered = dict(value)
        del altered[field]
        assert not check.is_valid(altered), field
    for field in ('caseLedgerId', 'processId'):
        assert not check.is_valid({**value, field: 'invented'}), field
    assert not check.is_valid({**value, 'tasks': None})
    assert not check.is_valid({**value, 'futureTaskId': 'invented'})


def test_acceptance_keeps_full_validation_of_each_created_task():
    task = {'id': 'urn:wos:default_process_01m4mey9j0e6stas981f5ey15v/tasks/review',
            'processId': 'urn:wos:default_process_01m4mey9j0e6stas981f5ey15v',
            'kind': 'review', 'status': 'pending', 'title': 'Review filing',
            'createdAt': '2026-10-11T03:08:50.459Z'}
    check = validator('IntakeHandoffReceipt')
    check.validate(accepted_receipt([task]))
    for field in ('id', 'processId', 'kind', 'status', 'title', 'createdAt'):
        altered = dict(task)
        del altered[field]
        assert not check.is_valid(accepted_receipt([altered])), field
    assert not check.is_valid(accepted_receipt([{}]))
    assert not check.is_valid(accepted_receipt([{**task, 'status': 'invented'}]))
