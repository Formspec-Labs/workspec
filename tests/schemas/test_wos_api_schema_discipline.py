import json
from pathlib import Path

import jsonschema
from jsonschema import Draft202012Validator, FormatChecker
from referencing import Registry
from referencing.jsonschema import DRAFT202012

from .test_record_kind_registry import event_literal_mappings


ROOT = Path(__file__).resolve().parents[2]
SCHEMA_DIR = ROOT / "schemas"
API_SCHEMA_DIR = SCHEMA_DIR / "api"
API_SPEC_DIR = ROOT / "specs" / "api"
OPENAPI_PATH = ROOT / "api" / "wos-public-api.openapi.json"
API_SCHEMAS = sorted(API_SCHEMA_DIR.glob("*.schema.json"))
DEFINITION_ONLY_API_SCHEMAS = {"_common.schema.json"}
ALLOW_ADDITIONAL_PROPERTIES_TRUE = {
    ("error.schema.json", "properties", "context"),
}


def load_schema(path: Path) -> dict:
    return json.loads(path.read_text())


def load_openapi() -> dict:
    return json.loads(OPENAPI_PATH.read_text())


def spec_path_for_schema(path: Path) -> Path:
    resource = path.name.removesuffix(".schema.json")
    return API_SPEC_DIR / f"{resource}.md"


def walk(node, path=()):
    if isinstance(node, dict):
        yield path, node
        for key, value in node.items():
            yield from walk(value, path + (key,))
    elif isinstance(node, list):
        for index, value in enumerate(node):
            yield from walk(value, path + (str(index),))


def pointer_exists(doc: dict, pointer: str) -> bool:
    if pointer in ("", "#"):
        return True
    if pointer.startswith("#"):
        pointer = pointer[1:]
    if not pointer:
        return True
    if not pointer.startswith("/"):
        return False

    current = doc
    for raw_part in pointer.lstrip("/").split("/"):
        part = raw_part.replace("~1", "/").replace("~0", "~")
        if isinstance(current, dict) and part in current:
            current = current[part]
            continue
        if isinstance(current, list) and part.isdigit() and int(part) < len(current):
            current = current[int(part)]
            continue
        return False
    return True


def api_schema_registry() -> dict[str, dict]:
    return {load_schema(path)["$id"]: load_schema(path) for path in API_SCHEMAS}


def api_referencing_registry() -> Registry:
    resources = [
        (schema_id, DRAFT202012.create_resource(schema))
        for schema_id, schema in api_schema_registry().items()
    ]
    return Registry().with_resources(resources)


def validator_for_api_def(
    schema: dict,
    def_name: str,
    registry: Registry,
) -> Draft202012Validator:
    target = schema["$defs"][def_name]
    composed = {
        "$schema": schema.get("$schema", "https://json-schema.org/draft/2020-12/schema"),
        "$id": f"{schema['$id']}#/$defs/{def_name}-example-check",
        "$defs": schema["$defs"],
        **target,
    }
    return Draft202012Validator(
        composed,
        registry=registry,
        format_checker=FormatChecker(),
    )


def local_def_name(ref: str) -> str | None:
    prefix = "#/$defs/"
    if ref.startswith(prefix):
        return ref.removeprefix(prefix)
    return None


def exported_model_names(schema: dict) -> set[str]:
    refs = []
    if isinstance(schema.get("$ref"), str):
        refs.append(schema["$ref"])
    refs.extend(
        entry["$ref"]
        for entry in schema.get("oneOf", [])
        if isinstance(entry, dict) and isinstance(entry.get("$ref"), str)
    )
    return {name for ref in refs if (name := local_def_name(ref))}


def exported_model_refs_by_schema_id() -> dict[str, set[str]]:
    refs_by_id = {}
    for path in API_SCHEMAS:
        schema = load_schema(path)
        refs_by_id[schema["$id"]] = {
            f"#/$defs/{name}" for name in exported_model_names(schema)
        }
    return refs_by_id


def assert_wos_api_ref_targets_exported_model(ref: str, refs_by_id: dict[str, set[str]]):
    if not ref.startswith("https://schemas.formspec.io/wos-api/"):
        return
    schema_id, _, pointer = ref.partition("#")
    assert schema_id in refs_by_id, f"{ref} does not resolve to a registered api schema"
    assert f"#{pointer}" in refs_by_id[schema_id], (
        f"{ref} targets a schema helper, not a schema-exported interface model"
    )


def test_api_schemas_are_meta_valid():
    assert API_SCHEMAS, "expected at least one api schema"
    validator = jsonschema.Draft202012Validator
    for path in API_SCHEMAS:
        validator.check_schema(load_schema(path))


def test_api_schemas_have_matching_api_specs():
    for path in API_SCHEMAS:
        schema = load_schema(path)
        spec_path = spec_path_for_schema(path)
        assert spec_path.exists(), f"{path.name} needs specs/api/{spec_path.name}"
        text = spec_path.read_text()
        assert path.name in text, f"{spec_path} must reference {path.name}"
        assert schema["$id"] in text, f"{spec_path} must reference {schema['$id']}"


def test_api_schema_roots_export_named_interface_models():
    for path in API_SCHEMAS:
        schema = load_schema(path)
        exported_names = exported_model_names(schema)

        if path.name in DEFINITION_ONLY_API_SCHEMAS:
            assert not exported_names, f"{path.name} is definitions-only and must not export models"
            continue

        assert exported_names, (
            f"{path.name} must expose public interface models through top-level $ref/oneOf"
        )
        for name in exported_names:
            assert name in schema.get("$defs", {}), f"{path.name} exports missing $defs/{name}"


def test_api_specs_name_schema_exported_interface_models():
    for path in API_SCHEMAS:
        schema = load_schema(path)
        spec_text = spec_path_for_schema(path).read_text()
        for name in exported_model_names(schema):
            assert name in spec_text, (
                f"{spec_path_for_schema(path).name} must name exported model {name}"
            )


def test_api_schema_exported_interface_model_names_are_unique():
    seen = {}
    for path in API_SCHEMAS:
        for name in exported_model_names(load_schema(path)):
            seen.setdefault(name, []).append(path.name)

    duplicates = {name: paths for name, paths in seen.items() if len(paths) > 1}
    assert not duplicates, f"api exported model names must be unique: {duplicates}"


def test_api_schema_objects_are_closed_except_registered_contexts():
    for path in API_SCHEMAS:
        schema = load_schema(path)
        for node_path, node in walk(schema):
            if node.get("type") != "object":
                continue
            if node.get("additionalProperties") is True:
                assert (path.name, *node_path[-2:]) in ALLOW_ADDITIONAL_PROPERTIES_TRUE
                continue
            assert node.get("additionalProperties") is False or node_path == ()


def test_api_schema_properties_have_descriptions():
    for path in API_SCHEMAS:
        schema = load_schema(path)
        for node_path, node in walk(schema):
            properties = node.get("properties")
            if not isinstance(properties, dict):
                continue
            for property_name, property_schema in properties.items():
                assert "description" in property_schema, (
                    f"{path.name}:{'/'.join(node_path + ('properties', property_name))} "
                    "is missing description"
                )


def test_api_schema_nullability_requires_comment():
    for path in API_SCHEMAS:
        schema = load_schema(path)
        for node_path, node in walk(schema):
            node_type = node.get("type")
            if isinstance(node_type, list) and "null" in node_type:
                assert "why-null" in node.get("$comment", ""), (
                    f"{path.name}:{'/'.join(node_path)} uses nullable type without why-null comment"
                )


def test_api_schema_open_strings_use_named_seams():
    for path in API_SCHEMAS:
        schema = load_schema(path)
        for node_path, node in walk(schema):
            if node.get("type") != "string":
                continue
            has_constraint = any(
                key in node for key in ("enum", "const", "pattern", "format")
            )
            has_named_seam = bool(node.get("x-wos", {}).get("openStringKind"))
            # A `type: string` declaration whose constraint lives in a sibling
            # `oneOf`/`anyOf` arm (e.g. `MutationKind` after the regex
            # standardization — root carries `type: string` plus `oneOf` with
            # an `enum` arm and a `pattern` arm) is structurally constrained
            # by the union; recurse into the arms to confirm.
            has_union_constraint = False
            for combinator in ("oneOf", "anyOf"):
                arms = node.get(combinator)
                if isinstance(arms, list):
                    if all(
                        isinstance(arm, dict) and any(k in arm for k in ("enum", "const", "pattern", "format"))
                        for arm in arms
                    ):
                        has_union_constraint = True
                        break
            assert has_constraint or has_named_seam or has_union_constraint, (
                f"{path.name}:{'/'.join(node_path)} is an unconstrained string "
                "without x-wos.openStringKind"
            )


def test_openapi_snapshot_uses_versioned_public_surface():
    doc = load_openapi()
    assert doc["openapi"] == "3.1.0"
    assert doc["servers"] == [
        {
            "url": "/api/v1",
            "description": "Current public major version.",
        }
    ]
    assert "/api/openapi.json" in doc["paths"]
    assert "/notifications" in doc["paths"]


def test_openapi_components_reference_registered_api_schemas():
    schemas_by_id = api_schema_registry()
    doc = load_openapi()

    for name, schema in doc["components"]["schemas"].items():
        assert set(schema) == {"$ref"}, f"{name} must be a pure schema $ref"

    refs = []
    for _, node in walk(doc):
        ref = node.get("$ref") if isinstance(node, dict) else None
        if isinstance(ref, str) and ref.startswith("https://schemas.formspec.io/wos-api/"):
            refs.append(ref)

    assert refs, "expected OpenAPI snapshot to reference api schemas"
    for ref in refs:
        schema_id, _, pointer = ref.partition("#")
        assert schema_id in schemas_by_id, f"{ref} does not resolve to a registered api schema"
        assert pointer_exists(schemas_by_id[schema_id], pointer), (
            f"{ref} does not resolve to an existing api schema target"
        )


def test_openapi_body_schemas_reference_exported_interface_models():
    refs_by_id = exported_model_refs_by_schema_id()
    doc = load_openapi()

    for schema in doc["components"]["schemas"].values():
        assert_wos_api_ref_targets_exported_model(schema["$ref"], refs_by_id)

    for response in doc["components"].get("responses", {}).values():
        for media in response.get("content", {}).values():
            schema = media.get("schema")
            if isinstance(schema, dict) and isinstance(schema.get("$ref"), str):
                assert_wos_api_ref_targets_exported_model(schema["$ref"], refs_by_id)

    for path_item in doc["paths"].values():
        for operation in path_item.values():
            request_body = operation.get("requestBody", {})
            for media in request_body.get("content", {}).values():
                schema = media.get("schema")
                if isinstance(schema, dict) and isinstance(schema.get("$ref"), str):
                    assert_wos_api_ref_targets_exported_model(schema["$ref"], refs_by_id)

            for response in operation.get("responses", {}).values():
                if "$ref" in response:
                    continue
                for media in response.get("content", {}).values():
                    schema = media.get("schema")
                    if isinstance(schema, dict) and isinstance(schema.get("$ref"), str):
                        assert_wos_api_ref_targets_exported_model(schema["$ref"], refs_by_id)


def test_api_schema_refs_resolve_to_existing_api_schema_targets():
    schemas_by_id = api_schema_registry()
    for path in API_SCHEMAS:
        schema = load_schema(path)
        for node_path, node in walk(schema):
            ref = node.get("$ref") if isinstance(node, dict) else None
            if not isinstance(ref, str):
                continue
            if ref.startswith("#"):
                assert pointer_exists(schema, ref), (
                    f"{path.name}:{'/'.join(node_path)} has unresolved local ref {ref}"
                )
                continue
            if ref.startswith("https://schemas.formspec.io/wos-api/"):
                schema_id, _, pointer = ref.partition("#")
                assert schema_id in schemas_by_id, (
                    f"{path.name}:{'/'.join(node_path)} references unknown api schema {ref}"
                )
                assert pointer_exists(schemas_by_id[schema_id], pointer), (
                    f"{path.name}:{'/'.join(node_path)} has unresolved api ref {ref}"
                )


def test_api_facts_record_kind_reserved_literals_match_kernel():
    kernel_schema = load_schema(SCHEMA_DIR / "wos-workflow.schema.json")
    kernel_record_kind = set(
        kernel_schema["$defs"]["FactsTierRecord"]["properties"]["recordKind"]["enum"]
    )

    api_schema = load_schema(API_SCHEMA_DIR / "provenance.schema.json")
    api_record_kind = api_schema["$defs"]["FactsRecordKind"]
    arms = api_record_kind["oneOf"]
    reserved_arms = [
        arm for arm in arms if isinstance(arm, dict) and isinstance(arm.get("enum"), list)
    ]
    extension_arms = [
        arm for arm in arms if isinstance(arm, dict) and arm.get("pattern") == "^x-[a-z][a-z0-9-]*$"
    ]

    assert len(reserved_arms) == 1, "FactsRecordKind must have one reserved enum arm"
    assert len(extension_arms) == 1, "FactsRecordKind must allow exactly the API vendor arm"
    assert set(reserved_arms[0]["enum"]) == kernel_record_kind


def _api_facts_record(record_kind: str, event: str) -> dict:
    return {
        "tier": "facts",
        "id": "urn:wos:agency-gov_prov_01jqrxabcd3f8xtx9qxkkv3raa",
        "instanceId": "urn:wos:sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsc",
        "recordKind": record_kind,
        "timestamp": "2026-04-23T12:00:00Z",
        "definitionVersion": "1.0.0",
        "event": event,
    }


def test_api_facts_record_kind_event_literals_agree_for_d26_seed():
    schema = load_schema(API_SCHEMA_DIR / "provenance.schema.json")
    validator = validator_for_api_def(
        schema,
        "FactsTierRecord",
        api_referencing_registry(),
    )

    for record_kind, event in event_literal_mappings().items():
        valid = _api_facts_record(record_kind, event)
        assert list(validator.iter_errors(valid)) == []

        wrong_event = _api_facts_record(record_kind, "decide")
        assert list(validator.iter_errors(wrong_event)), (
            f"{record_kind} must require {event}"
        )


def test_api_def_examples_validate_against_their_own_definitions():
    registry = api_referencing_registry()
    for path in API_SCHEMAS:
        schema = load_schema(path)
        for def_name, definition in schema.get("$defs", {}).items():
            examples = definition.get("examples", [])
            if not examples:
                continue

            validator = validator_for_api_def(schema, def_name, registry)
            for index, example in enumerate(examples):
                errors = sorted(validator.iter_errors(example), key=lambda error: error.path)
                assert not errors, (
                    f"{path.name}#/$defs/{def_name}/examples/{index} "
                    f"does not validate: {[error.message for error in errors]}"
                )
