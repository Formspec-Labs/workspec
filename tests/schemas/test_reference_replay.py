"""Reference replay input schemas share the exact maintained executable fixture."""
import copy
import json
from jsonschema import Draft202012Validator
from tests.schemas.conftest import SCHEMAS_ROOT, WOS_SPEC_ROOT, _REGISTRY

SCHEMA = json.loads((SCHEMAS_ROOT / 'wos-reference-replay.schema.json').read_text())
FIXTURE = json.loads((WOS_SPEC_ROOT / 'crates/wos-conformance/tests/fixtures/reference-due-process-replay.json').read_text())

def validator(name):
    return Draft202012Validator({'$ref': SCHEMA['$id'] + '#/$defs/' + name}, registry=_REGISTRY)

def test_exact_executable_adapters_and_request_follow_published_schema():
    Draft202012Validator.check_schema(SCHEMA)
    validator('ReferenceReplayAdapters').validate(FIXTURE['adapters'])
    validator('ReplayRequest').validate(FIXTURE['request'])
    workflow = json.loads((SCHEMAS_ROOT / 'wos-workflow.schema.json').read_text())
    Draft202012Validator(workflow, registry=_REGISTRY).validate(json.loads(bytes(FIXTURE['request']['candidateBytes'])))
    fixture = json.loads(FIXTURE['request']['fixtureJson'])
    for event in fixture['event_sequence']:
        if 'governance_recipe' in event:
            validator('GovernanceRecipe').validate(event['governance_recipe'])
        if 'signature_recipe' in event.get('task_submission', {}):
            validator('SignatureRecipe').validate(event['task_submission']['signature_recipe'])

def test_null_and_invented_crypto_verdicts_are_closed():
    fixture = json.loads(FIXTURE['request']['fixtureJson'])
    recipe = copy.deepcopy(fixture['event_sequence'][2]['governance_recipe'])
    recipe['target'] = None
    assert not validator('GovernanceRecipe').is_valid(recipe)
    recipe = copy.deepcopy(fixture['event_sequence'][1]['task_submission']['signature_recipe'])
    recipe['signature']['verified'] = True
    assert not validator('SignatureRecipe').is_valid(recipe)
    adapter = copy.deepcopy(FIXTURE['adapters'])
    adapter['hostedDeliveryQualified'] = True
    assert not validator('ReferenceReplayAdapters').is_valid(adapter)

def test_selector_and_review_claims_remain_closed():
    selector = {'attempt': {'eventIndex': 2, 'kind': 'governanceOperation', 'occurrence': 0},
                'subject': {'kind': 'notice', 'target': {'bindingRef': 'adverse', 'firingOccurrence': 0}}}
    assert validator('NativeSelector').is_valid(selector)
    selector['subject']['determinationId'] = 'guessed-id'
    assert not validator('NativeSelector').is_valid(selector)
    assert not validator('EngineeringClaim').is_valid('section508Certified')
    assert validator('EngineeringClaim').is_valid('accessibilityStructureReview')
