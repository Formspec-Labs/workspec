# /// script
# requires-python = ">=3.12"
# dependencies = ["mcp==2.2.0"]
# ///
"""One official-SDK journey; native reports and requests remain unmodified."""

import argparse
import asyncio
import hashlib
import importlib.metadata
import json
from pathlib import Path
import platform
import traceback

from mcp import Client, StdioServerParameters
from mcp.shared.exceptions import MCPError
from jsonschema.validators import validator_for
from referencing import Registry, Resource
from referencing.jsonschema import DRAFT202012


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def validate_schema(schema):
    """Check schema syntax and every reference, including unevaluated branches."""
    validator_for(schema).check_schema(schema)
    resolver = Registry().resolver_with_root(Resource.from_contents(schema, default_specification=DRAFT202012))

    def references(value, current):
        count = 0
        if isinstance(value, dict):
            if "$id" in value:
                current = current.in_subresource(Resource.from_contents(value, default_specification=DRAFT202012))
            if "$ref" in value:
                current.lookup(value["$ref"])
                count += 1
            for child in value.values():
                count += references(child, current)
        elif isinstance(value, list):
            for child in value:
                count += references(child, current)
        return count

    return references(schema, resolver)


class Journey:
    def __init__(self, args):
        self.args = args
        self.evidence = Path(args.evidence)
        self.evidence.mkdir(parents=True, exist_ok=True)
        self.inputs = json.loads(Path(args.inputs).read_text())
        self.transcript = self.evidence / "calls.jsonl"
        self.transcript.write_text("")
        self.process = 0

    def record(self, record):
        with self.transcript.open("a") as stream:
            stream.write(json.dumps({"process": self.process, **record}) + "\n")

    async def call(self, client, name, arguments, error=False, protocol_error=False):
        entry = {"name": name, "arguments": arguments}
        try:
            result = await client.call_tool(name, arguments)
        except MCPError as exc:
            self.record({**entry, "protocol_error": exc.error.model_dump(mode="json", by_alias=True)})
            assert protocol_error and exc.error.code == -32602, str(exc)
            return
        except Exception as exc:
            self.record({**entry, "exception": {"type": type(exc).__name__, "message": str(exc)}})
            raise
        raw = result.model_dump(mode="json", by_alias=True)
        self.record({**entry, "result": raw, "value": result.structured_content})
        assert not protocol_error, "unknown tool must be a protocol fault"
        assert result.is_error == error, raw
        if error:
            return result
        value = result.structured_content
        assert value is not None, "SDK default validation requires structured results"
        assert json.loads(result.content[0].text) == value, "text and structured results diverged"
        return value

    async def connect(self, client):
        self.record({"negotiation": {
            "protocol_version": client.protocol_version,
            "server_info": client.server_info.model_dump(mode="json", by_alias=True),
        }})
        catalog = await client.list_tools()
        raw = catalog.model_dump(mode="json", by_alias=True, exclude_none=True)
        save(self.evidence / f"catalog-{self.process}.json", raw)
        listed = {tool["name"]: tool for tool in raw["tools"]}
        declared = {tool["name"]: tool for tool in self.inputs["catalog"]["tools"]}
        assert listed.keys() == declared.keys()
        checked = []
        for name in listed:
            for key in ["inputSchema", "outputSchema"]:
                assert listed[name][key] == declared[name][key], (name, key)
                try:
                    references = validate_schema(listed[name][key])
                except Exception as exc:
                    checked.append({"tool": name, "schema": key, "error": str(exc)})
                    save(self.evidence / f"catalog-validation-{self.process}.json", checked)
                    raise
                checked.append({"tool": name, "schema": key, "metaschema_valid": True, "resolved_references": references})
        save(self.evidence / f"catalog-validation-{self.process}.json", checked)
        self.tools = listed
        await self.call(client, "wos_ping", {})

    async def export(self, client, pid):
        return (await self.call(client, "wos_export_document", {"project_id": pid}))["document"]

    async def edit_state(self, client, pid, state):
        await self.call(client, "wos_edit_workflow", {
            "project_id": pid, "action": "state", "pointer": "/lifecycle/states/open", "state": state,
        })

    async def run(self):
        save(self.evidence / "environment.json", {
            "python": platform.python_version(), "mcp": importlib.metadata.version("mcp"),
            "binary": self.args.binary,
            "binary_sha256": hashlib.sha256(Path(self.args.binary).read_bytes()).hexdigest(),
            "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            "lock_sha256": hashlib.sha256(Path(str(Path(__file__)) + ".lock").read_bytes()).hexdigest(),
        })
        save(self.evidence / "inputs.json", self.inputs)
        parameters = StdioServerParameters(command=self.args.binary)
        async with Client(parameters, read_timeout_seconds=30) as client:
            await self.connect(client)
            # Cheap incremental authoring uses only advertised tools.
            created = await self.call(client, "wos_create_kernel", {})
            assert created["draft"] is True
            draft = created["project_id"]
            actor_schema = self.tools["wos_add_actor"]["inputSchema"]
            actor_validator = validator_for(actor_schema)(actor_schema, registry=Registry())
            valid_actor = {"project_id": draft, "actor_id": "reviewer", "kind": "human"}
            invalid_actor = {"project_id": draft, "actor_id": "invalid", "kind": "invented"}
            assert actor_validator.is_valid(valid_actor)
            assert not actor_validator.is_valid(invalid_actor)
            await self.call(client, "wos_add_actor", valid_actor)
            await self.call(client, "wos_add_state", {"project_id": draft, "state_id": "review", "kind": "atomic"})
            await self.call(client, "wos_set_initial_state", {"project_id": draft, "state_id": "review"})
            await self.call(client, "wos_edit_workflow", {"project_id": draft, "fields": {"title": "Official client draft", "url": "urn:test:official-client-draft"}})
            native = json.loads(await self.export(client, draft))
            assert native["title"] == "Official client draft"
            assert native["lifecycle"]["initialState"] == "review"
            assert native["actors"] == [{"id": "reviewer", "type": "human"}]
            before_actor_refusal = await self.export(client, draft)
            await self.call(client, "wos_add_actor", invalid_actor, error=True)
            assert await self.export(client, draft) == before_actor_refusal
            await self.call(client, "wos_close_project", {"project_id": draft})

            # Complete authored values enter through canonical load. The reviewed
            # scenario stays unchanged while a named edit supplies the typed event.
            candidate = self.inputs["candidate"]
            pid = (await self.call(client, "wos_load_document", {"json": candidate}))["project_id"]
            await self.call(client, "wos_edit_workflow", {"project_id": pid, "fields": {"title": "Official client reviewed workflow"}})
            await self.edit_state(client, pid, self.inputs["passing_state"])
            expected = json.loads(candidate)
            expected["title"] = "Official client reviewed workflow"
            expected["lifecycle"]["states"]["open"] = self.inputs["passing_state"]
            assert json.loads(await self.export(client, pid)) == expected

            # Reference resolution uses exact selected Definition/Mapping inputs.
            reference_pid = (await self.call(client, "wos_load_document", {"json": self.inputs["reference_candidate"]}))["project_id"]
            missing = await self.call(client, "wos_lint", {"project_id": reference_pid, "scope": "project"})
            assert missing["validation"]["status"] == "fail", missing
            selected = await self.call(client, "wos_lint", {"project_id": reference_pid, **self.inputs["validation"]})
            assert selected["validation"]["status"] == "pass", selected
            assert any("formspec" in stage for stage in selected["validation"]["stages"])
            await self.call(client, "wos_close_project", {"project_id": reference_pid})

            requests = self.inputs["checks"]
            save(self.evidence / "exact-check-requests.json", requests)
            first = await self.call(client, "wos_check_authoring", {"project_id": pid, **requests})
            self.assert_checked(first, True)
            await self.edit_state(client, pid, self.inputs["failing_state"])
            failed = await self.call(client, "wos_check_authoring", {"project_id": pid, **requests})
            self.assert_checked(failed, False)
            assert failed["static_identity"] != first["static_identity"]
            assert self.replay(failed)["identity"] != self.replay(first)["identity"]
            assert self.replay(failed)["failures"] or self.replay(failed)["differences"]
            assert self.replay(failed)["finalSnapshot"]["configuration"] == ["open"]
            await self.edit_state(client, pid, self.inputs["passing_state"])
            repaired = await self.call(client, "wos_check_authoring", {"project_id": pid, **requests})
            self.assert_checked(repaired, True)
            self.assert_identities(first, repaired)
            assert first["validation"] == repaired["validation"]
            before = await self.export(client, pid)
            await self.call(client, "wos_edit_workflow", {"project_id": pid, "action": "unknown"}, error=True)
            await self.call(client, "wos_edit_workflow", {"project_id": pid, "action": "state", "pointer": "/lifecycle/states/open", "state": {"type": "invented"}}, error=True)
            await self.call(client, "wos_no_such_tool", {}, protocol_error=True)
            refused = await self.call(client, "wos_run_conformance", {"project_id": pid, "fixture_json": requests["replays"][0]["arguments"]["fixture_json"], "expected_observations": [{"pointer": "/durableRecovery", "expected": True}]})
            assert refused["passed"] is False and refused["status"] == "refused", refused
            assert "missing-observation /durableRecovery" in refused["refusals"], refused
            assert await self.export(client, pid) == before
            save(self.evidence / "exported-workflow.json", json.loads(before))
            (self.evidence / "exported-workflow.bytes").write_text(before)
            await self.call(client, "wos_close_project", {"project_id": pid})

        self.process = 1
        # New SDK context launches a distinct binary process with an empty registry.
        async with Client(parameters, read_timeout_seconds=30) as client:
            await self.connect(client)
            empty = await self.call(client, "wos_list_projects", {})
            assert empty["count"] == 0
            pid = (await self.call(client, "wos_load_document", {"json": before}))["project_id"]
            assert await self.export(client, pid) == before
            reopened = await self.call(client, "wos_check_authoring", {"project_id": pid, **requests})
            self.assert_checked(reopened, True)
            self.assert_identities(repaired, reopened)
            assert repaired["validation"] == reopened["validation"]
            assert reopened["replays"][0]["arguments"] == repaired["replays"][0]["arguments"]
            await self.call(client, "wos_close_project", {"project_id": pid})
        save(self.evidence / "outcome.json", {"passed": True, "processes": 2, "shared_decisions": ["verified", "blocked", "verified", "verified"]})

    @staticmethod
    def replay(report):
        return report["replays"][0]["result"]["report"]

    def assert_checked(self, report, passed):
        assert report["decision"]["status"] == ("verified" if passed else "blocked"), report
        assert report["validation"]["result"]["report"]["validation"]["status"] == "pass", report
        replay = self.replay(report)
        assert replay["passed"] is passed, replay
        assert replay["status"] == ("pass" if passed else "fail"), replay
        if passed:
            assert replay["finalSnapshot"]["configuration"] == ["done"], replay

    def assert_identities(self, left, right):
        for key in ["static_identity", "static_owner_identity", "decision"]:
            assert left[key] == right[key], key
        for key in ["identity", "inputs", "runtimeIdentity", "execution", "status", "passed", "failures", "differences", "refusals"]:
            assert self.replay(left)[key] == self.replay(right)[key], key


async def main(args):
    async with asyncio.timeout(120):
        await Journey(args).run()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True)
    parser.add_argument("--evidence", required=True)
    parser.add_argument("--inputs", required=True)
    args = parser.parse_args()
    try:
        asyncio.run(main(args))
    except BaseException:
        Path(args.evidence).mkdir(parents=True, exist_ok=True)
        (Path(args.evidence) / "failure.txt").write_text(traceback.format_exc())
        raise
