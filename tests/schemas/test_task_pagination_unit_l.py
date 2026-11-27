"""Schema tests for Unit L task pagination and searchIncomplete contract."""
import pytest

from .test_wos_api_schema_discipline import (
    API_SCHEMA_DIR,
    api_referencing_registry,
    load_schema,
    validator_for_api_def,
)

TASK_SCHEMA = load_schema(API_SCHEMA_DIR / "task.schema.json")
PAGINATION_SCHEMA = load_schema(API_SCHEMA_DIR / "pagination.schema.json")
TASK_PAGE_VALIDATOR = validator_for_api_def(
    TASK_SCHEMA, "TaskPage", api_referencing_registry()
)
PAGINATED_RESULT_VALIDATOR = validator_for_api_def(
    PAGINATION_SCHEMA, "PaginatedResult", api_referencing_registry()
)


def test_task_page_accepts_search_incomplete_without_cursor():
    TASK_PAGE_VALIDATOR.validate(
        {
            "items": [],
            "hasMore": False,
            "searchIncomplete": True,
        }
    )


def test_task_page_rejects_total_count_field():
    assert not TASK_PAGE_VALIDATOR.is_valid(
        {
            "items": [],
            "hasMore": False,
            "total": 42,
        }
    )


def test_paginated_result_accepts_search_incomplete():
    PAGINATED_RESULT_VALIDATOR.validate(
        {
            "items": [],
            "hasMore": False,
            "searchIncomplete": True,
        }
    )


@pytest.mark.parametrize("bad_value", ["true", 1, None])
def test_task_page_rejects_non_boolean_search_incomplete(bad_value):
    assert not TASK_PAGE_VALIDATOR.is_valid(
        {
            "items": [],
            "hasMore": False,
            "searchIncomplete": bad_value,
        }
    )
