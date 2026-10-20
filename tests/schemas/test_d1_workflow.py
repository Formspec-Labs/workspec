"""Historical D1 authoring evidence, intake pins and current-schema refusal."""
import hashlib
import json
import runpy

from jsonschema import Draft202012Validator

from tests.schemas.conftest import WOS_SPEC_ROOT


def _artifact():
    return json.loads((WOS_SPEC_ROOT / 'examples/d1-passport.workflow.json').read_text())


def test_historical_workflow_has_precise_schema_refusal_and_native_intake_pins():
    artifact = _artifact()
    schema = json.loads((WOS_SPEC_ROOT / 'schemas/wos-workflow.schema.json').read_text())
    errors = list(Draft202012Validator(schema).iter_errors(artifact))
    # Retained original bytes are evidence, not a current supported workflow.
    # Current positive coverage lives in the native model/reference fixtures.
    assert len(errors) == 1, '\n'.join(str(error) for error in errors)
    error, = errors
    assert error.validator == 'additionalProperties'
    assert list(error.absolute_path) == ['governance', 'dueProcess']
    assert "'explanationLevel'" in error.message
    source = WOS_SPEC_ROOT.parent / 'formspec-studio/demo-authoring/outputs/d1/v0.1.6'
    definition_path = source / 'passport-renewal/passport-renewal.definition.json'
    definition = json.loads(definition_path.read_text())
    receipt = json.loads((source / 'receipts/server-definition-hash-v0.1.6.json').read_text())
    assert artifact['intake'] == {'mode': 'publicIntake', 'contractRef': definition['url'], 'contractVersion': definition['version']}
    assert definition['version'] == '0.1.6'
    pin = artifact['extensions']['x-d1-definition-pin']
    assert pin['definitionHash'] == receipt['definition_hash']
    assert pin['rawDefinitionHash'] == 'sha256:' + hashlib.sha256(definition_path.read_bytes()).hexdigest()
    assert pin['definitionHash'] != pin['rawDefinitionHash']


def test_historical_initial_review_is_human_without_placeholder_form_or_decision():
    artifact = _artifact()
    assert artifact['impactLevel'] == 'rights-impacting'
    assert artifact['lifecycle']['initialState'] == 'awaitingReview'
    state = artifact['lifecycle']['states']['awaitingReview']
    assert state['type'] == 'atomic'
    assert not state['transitions']
    action, = state['onEntry']
    assert action['action'] == 'createTask'
    assert action['taskRef'] == 'reviewApplication'
    actors = {actor['id']: actor for actor in artifact['actors']}
    assert actors[action['assignTo']]['type'] == 'human'
    assert action['assignTo'] == 'adjudicator'
    assert action.get('contractRef') is None
    assert not artifact['contracts']
    assert 'signature' not in artifact
    assert artifact['governance']['dueProcess'] == {'explanationLevel': 'individualized'}
    pattern, = artifact['governance']['taskCatalog']
    assert pattern['pattern'] == action['taskRef']
    assert pattern['description']


def test_historical_log_preserves_actual_load_export_and_twin_replay():
    path = WOS_SPEC_ROOT / 'examples/d1-passport.workflow.json'
    log = json.loads((WOS_SPEC_ROOT / 'examples/d1-passport.authoring-log.json').read_text())
    assert log['deterministicReplay'] is True
    assert log['artifactSha256'] == hashlib.sha256(path.read_bytes()).hexdigest()
    assert len(log['replays']) == 2
    for replay in log['replays']:
        calls = [item['request']['params']['name'] for item in replay if 'request' in item]
        assert calls == ['wos_create_kernel', 'wos_add_actor', 'wos_add_actor', 'wos_add_state',
                         'wos_set_initial_state', 'wos_set_impact_level', 'wos_export_document',
                         'wos_load_document', 'wos_export_document']
        assert replay[-1]['exitCode'] == 0
        assert all(not entry['response'].get('result', {}).get('isError') for entry in replay if 'response' in entry)


def test_omission_normalization_preserves_native_null_data():
    functions = runpy.run_path(str(WOS_SPEC_ROOT / 'scripts/replay-d1-authoring.py'))
    value = {'description': None, 'caseFile': {'default': None}, 'x-example': {'content': None},
             'lifecycle': {'states': {'awaitingReview': {'initialState': None, 'onEntry': [{'data': {'value': None}}]}}}}
    paths = []
    normalized = functions['omit_null_defaults'](value, paths)
    assert paths == ['/description', '/lifecycle/states/awaitingReview/initialState']
    assert normalized['caseFile'] == {'default': None}
    assert normalized['x-example'] == {'content': None}
    assert normalized['lifecycle']['states']['awaitingReview']['onEntry'][0]['data'] == {'value': None}


def test_atomic_normalization_omits_only_empty_structural_defaults():
    functions = runpy.run_path(str(WOS_SPEC_ROOT / 'scripts/replay-d1-authoring.py'))
    value = {'lifecycle': {'states': {
        'atomic': {'type': 'atomic', 'states': {}, 'regions': {}},
        'compound': {'type': 'compound', 'states': {'child': {'type': 'atomic'}}, 'initialState': 'child'},
    }}}
    paths = []
    normalized = functions['omit_null_defaults'](value, paths)
    assert paths == ['/lifecycle/states/atomic/regions', '/lifecycle/states/atomic/states']
    assert normalized['lifecycle']['states']['atomic'] == {'type': 'atomic'}
    assert normalized['lifecycle']['states']['compound'] == value['lifecycle']['states']['compound']
    for field in ['states', 'regions']:
        invalid = {'lifecycle': {'states': {'atomic': {'type': 'atomic', field: {'unexpected': {}}}}}}
        import pytest
        with pytest.raises(ValueError, match='nonempty'):
            functions['omit_null_defaults'](invalid, [])


def test_historical_custody_is_an_explicit_first_slice_declaration():
    artifact = _artifact()
    assert artifact['custody'] == {'anchorRequirements': []}
    posture = artifact['extensions']['x-d1-custody-posture']
    assert 'specific-case authorization' in posture['ordinaryReaders']
    assert posture['recoveryWithoutUser'] is True
    assert 'plaintext evidence' in posture['delegatedComputeVisibility']
    assert 'no external delegated compute' in posture['delegatedComputeVisibility']
    assert 'trustProfileRef' not in artifact['custody']
    assert 'exportBundleRef' not in artifact['custody']
