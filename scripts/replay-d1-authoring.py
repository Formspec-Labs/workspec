#!/usr/bin/env python3
"""Replay maintained WOS MCP authoring for the bounded D1 intake workflow.

This client authors configuration only. It creates no cases and runs no engine.
The explicit supplement covers fields without an MCP editing tool. The final
artifact comes from maintained load/export; omission of emitted null defaults
is logged separately and never changes an authored value.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
STACK = ROOT.parent
DEFINITION = STACK / 'formspec-studio/demo-authoring/outputs/d1/v0.1.6/passport-renewal/passport-renewal.definition.json'
HASH_RECEIPT = STACK / 'formspec-studio/demo-authoring/outputs/d1/v0.1.6/receipts/server-definition-hash-v0.1.6.json'
URL = 'urn:wos:demo:d1:passport-renewal'
SEMANTIC_HASH = 'sha256:7dd5277eae83e2e96ede564c09901607916a1edcfa6364e21f13fb813ed39005'
RAW_HASH = '39e4086f46c1c2479f8f3bd87f20a9ad3054973088f1165c0a081a51df5d3b39'


# Only named optional schema properties are normalized. Never traverse case data,
# actions, extension JSON, default values, or content JSON.
OPTIONAL_TOP = frozenset({'$schema', 'description', 'status', 'caseFile', 'provenance',
                          'execution', 'evaluationMode', 'maxRelationshipEventDepth'})
OPTIONAL_STATE = frozenset({'initialState', 'outcomeCode'})


def omit_null_defaults(value, paths):
    import copy
    result = copy.deepcopy(value)
    for key in OPTIONAL_TOP:
        if key in result and result[key] is None:
            del result[key]
            paths.append('/' + key)
    for state_id, state in result['lifecycle']['states'].items():
        if state.get('type') == 'atomic':
            for key in ('states', 'regions'):
                if key in state:
                    if state[key] != {}:
                        raise ValueError(f'Atomic state {state_id} has nonempty {key}; cannot normalize')
                    del state[key]
                    paths.append('/lifecycle/states/' + state_id + '/' + key)
        for key in OPTIONAL_STATE:
            if key in state and state[key] is None:
                del state[key]
                paths.append('/lifecycle/states/' + state_id + '/' + key)
    paths.sort()
    return result


def replay(binary: Path):
    entries = []
    with tempfile.TemporaryFile(mode='w+') as stderr:
        process = subprocess.Popen([str(binary)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                   stderr=stderr, text=True)
        def call(name, arguments):
            request = {'jsonrpc': '2.0', 'id': len(entries) + 1, 'method': 'tools/call',
                       'params': {'name': name, 'arguments': arguments}}
            process.stdin.write(json.dumps(request) + '\n')
            process.stdin.flush()
            response = json.loads(process.stdout.readline())
            entries.append({'request': request, 'response': response})
            if 'error' in response or response.get('result', {}).get('isError'):
                raise RuntimeError(json.dumps(response))
            return json.loads(response['result']['content'][0]['text'])
        try:
            project = call('wos_create_kernel', {})['project_id']
            for actor, kind in [('adjudicator', 'human'), ('intakeService', 'system')]:
                call('wos_add_actor', {'project_id': project, 'actor_id': actor, 'kind': kind})
            call('wos_add_state', {'project_id': project, 'state_id': 'awaitingReview', 'kind': 'atomic'})
            call('wos_set_initial_state', {'project_id': project, 'state_id': 'awaitingReview'})
            call('wos_set_impact_level', {'project_id': project, 'level': 'rights-impacting'})
            seed = json.loads(call('wos_export_document', {'project_id': project})['document'])
            omissions = []
            authored = omit_null_defaults(seed, omissions)
            supplement = {
                'url': URL, 'version': '0.1.0', 'title': 'Passport renewal: intake review',
                'status': 'draft',
                'description': 'Bounded D1 public intake and initial human review. Actor identifiers are deployment configuration slots.',
                'intake': {'mode': 'publicIntake', 'contractRef': 'urn:formspec:demo:d1:passport-renewal', 'contractVersion': '0.1.6'},
                'governance': {
                    'dueProcess': {'explanationLevel': 'individualized'},
                    'taskCatalog': [{'id': 'reviewApplication', 'pattern': 'reviewApplication', 'verifiable': 'partially',
                                     'verificationMethod': 'Compare recorded observations with the retained application and evidence.',
                                     'title': 'Review passport renewal application',
                                     'description': 'Review the accepted application and its retained evidence. Record individualized observations. This task does not issue a decision.'}],
                },
                'provenance': {'tier': 'facts'},
                'custody': {'anchorRequirements': []},
                'extensions': {'x-d1-custody-posture': {
                    'ordinaryReaders': 'Formspec source service and scope-bound WOS evidence service; authenticated reviewers only after specific-case authorization',
                    'recoveryWithoutUser': True,
                    'delegatedComputeVisibility': 'Ordinary configured Formspec/WOS server components process plaintext evidence; no external delegated compute is authorized by this workflow',
                }, 'x-d1-definition-pin': {'definitionHash': SEMANTIC_HASH, 'rawDefinitionHash': 'sha256:' + RAW_HASH,
                                        'receipt': 'formspec-studio/demo-authoring/outputs/d1/v0.1.6/receipts/server-definition-hash-v0.1.6.json'}},
            }
            authored.update(supplement)
            action = {'action': 'createTask', 'taskRef': 'reviewApplication', 'assignTo': 'adjudicator',
                      'description': 'Review the accepted passport renewal application.'}
            authored['lifecycle']['states']['awaitingReview']['onEntry'] = [action]
            entries.append({'authoringSupplement': supplement,
                            'onEntrySupplement': {'awaitingReview': [action]},
                            'seedNullDefaultsOmitted': omissions})
            loaded = call('wos_load_document', {'json': json.dumps(authored)})['project_id']
            exported = json.loads(call('wos_export_document', {'project_id': loaded})['document'])
            final_omissions = []
            final = omit_null_defaults(exported, final_omissions)
            entries.append({'exportNullDefaultsOmitted': final_omissions})
            return final, entries
        finally:
            process.stdin.close()
            process.wait(timeout=10)
            stderr.seek(0)
            entries.append({'stdioStderr': stderr.read(), 'exitCode': process.returncode})


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, default=ROOT / 'examples/d1-passport.workflow.json')
    parser.add_argument('--log', type=Path, default=ROOT / 'examples/d1-passport.authoring-log.json')
    args = parser.parse_args()
    definition = json.loads(DEFINITION.read_text())
    receipt = json.loads(HASH_RECEIPT.read_text())
    assert definition['url'] == 'urn:formspec:demo:d1:passport-renewal'
    assert definition['version'] == '0.1.6'
    assert hashlib.sha256(DEFINITION.read_bytes()).hexdigest() == RAW_HASH
    assert receipt['definition_hash'] == SEMANTIC_HASH
    assert receipt['raw_sha256'] == RAW_HASH
    binary = args.binary.resolve(strict=True)
    first, first_log = replay(binary)
    second, second_log = replay(binary)
    assert first == second, 'maintained replay produced different workflow values'
    rendered = json.dumps(first, indent=2, sort_keys=True) + '\n'
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(rendered)
    args.log.write_text(json.dumps({'binary': str(binary), 'binarySha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                                  'artifactSha256': hashlib.sha256(rendered.encode()).hexdigest(),
                                  'deterministicReplay': True, 'definitionReceipt': receipt,
                                  'replays': [first_log, second_log]}, indent=2) + '\n')
    print(json.dumps({'output': str(args.output), 'log': str(args.log), 'deterministicReplay': True}))


if __name__ == '__main__':
    main()
