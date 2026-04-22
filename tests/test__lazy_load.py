from typing import Any

import pytest

import pypinch
from pypinch import lazy_load_bytes, Idx

SAME_TYPE_LISTS = pypinch.dump_bytes([
    [
        [False, True, False],
        [1, 2, 3, 4],
        ["pointer", "sfad", "sdaga", "pointer"],
        {"asdf": [0.1, 0.2, 0.3, 0.4, 0.5], "pointer": "None"},
        [],
        [None, None, None]
    ]
])

MIX = pypinch.dump_bytes({
    "a": -1,
    "b": {"c": {"d": "d"}},
    "c": [1, False, b"sdfghfg", 6],
    1234: "d",
})

WITH_STRUCTURED_DICT_LIST = pypinch.dump_bytes([
    {"name": "Bob", "age": 24, "country": "USA"},
    {"name": "John", "age": 29, "country": "China"},
    {"name": "Jack", "age": 20, "country": "Spain"},
])


@pytest.mark.parametrize(
    ["data", "path", "expected"],
    [
        (MIX, ["a"], -1),
        (MIX, ["b"], {"c": {"d": "d"}}),
        (MIX, ["b", "c"], {"d": "d"}),
        (MIX, ["b", "c", "d"], "d"),
        (MIX, ["c"], [1, False, b"sdfghfg", 6]),
        (MIX, ["c", Idx(0)], 1),
        (MIX, ["c", Idx(1)], False),
        (MIX, ["c", Idx(2)], b"sdfghfg"),
        (MIX, ["c", Idx(3)], 6),
        (MIX, [1234], "d"),
        (SAME_TYPE_LISTS, [Idx(0)], [[False, True, False], [1, 2, 3, 4], ["pointer", "sfad", "sdaga", "pointer"], {"asdf": [0.1, 0.2, 0.3, 0.4, 0.5], "pointer": "None"}, [], [None, None, None]]),
        (SAME_TYPE_LISTS, [Idx(0), Idx(0)], [False, True, False]),
        (SAME_TYPE_LISTS, [Idx(0), Idx(1)], [1, 2, 3, 4]),
        (SAME_TYPE_LISTS, [Idx(0), Idx(2)], ["pointer", "sfad", "sdaga", "pointer"]),
        (SAME_TYPE_LISTS, [Idx(0), Idx(3)], {"asdf": [0.1, 0.2, 0.3, 0.4, 0.5], "pointer": "None"}),
        (SAME_TYPE_LISTS, [Idx(0), Idx(4)], []),
        (SAME_TYPE_LISTS, [Idx(0), Idx(5)], [None, None, None]),
        (SAME_TYPE_LISTS, [Idx(0), Idx(0), Idx(0)], False),
        (SAME_TYPE_LISTS, [Idx(0), Idx(0), Idx(1)], True),
        (SAME_TYPE_LISTS, [Idx(0), Idx(0), Idx(2)], False),
        (SAME_TYPE_LISTS, [Idx(0), Idx(1), Idx(0)], 1),
        (SAME_TYPE_LISTS, [Idx(0), Idx(1), Idx(1)], 2),
        (SAME_TYPE_LISTS, [Idx(0), Idx(1), Idx(2)], 3),
        (SAME_TYPE_LISTS, [Idx(0), Idx(1), Idx(3)], 4),
        (SAME_TYPE_LISTS, [Idx(0), Idx(2), Idx(0)], "pointer"),
        (SAME_TYPE_LISTS, [Idx(0), Idx(2), Idx(1)], "sfad"),
        (SAME_TYPE_LISTS, [Idx(0), Idx(2), Idx(2)], "sdaga"),
        (SAME_TYPE_LISTS, [Idx(0), Idx(2), Idx(3)], "pointer"),
        (SAME_TYPE_LISTS, [Idx(0), Idx(3), "asdf"], [0.1, 0.2, 0.3, 0.4, 0.5]),
        (SAME_TYPE_LISTS, [Idx(0), Idx(3), "pointer"], "None"),
        (SAME_TYPE_LISTS, [Idx(0), Idx(3), "asdf", Idx(0)], 0.1),
        (SAME_TYPE_LISTS, [Idx(0), Idx(3), "asdf", Idx(1)], 0.2),
        (SAME_TYPE_LISTS, [Idx(0), Idx(3), "asdf", Idx(2)], 0.3),
        (SAME_TYPE_LISTS, [Idx(0), Idx(3), "asdf", Idx(3)], 0.4),
        (SAME_TYPE_LISTS, [Idx(0), Idx(3), "asdf", Idx(4)], 0.5),
        (SAME_TYPE_LISTS, [Idx(0), Idx(5), Idx(0)], None),
        (SAME_TYPE_LISTS, [Idx(0), Idx(5), Idx(1)], None),
        (SAME_TYPE_LISTS, [Idx(0), Idx(5), Idx(2)], None),
        (WITH_STRUCTURED_DICT_LIST, [Idx(0), "name"], "Bob"),
        (WITH_STRUCTURED_DICT_LIST, [Idx(1), "name"], "John"),
        (WITH_STRUCTURED_DICT_LIST, [Idx(2), "name"], "Jack"),
        (WITH_STRUCTURED_DICT_LIST, [Idx(0), "age"], 24),
        (WITH_STRUCTURED_DICT_LIST, [Idx(1), "age"], 29),
        (WITH_STRUCTURED_DICT_LIST, [Idx(2), "age"], 20),
        (WITH_STRUCTURED_DICT_LIST, [Idx(0), "country"], "USA"),
        (WITH_STRUCTURED_DICT_LIST, [Idx(1), "country"], "China"),
        (WITH_STRUCTURED_DICT_LIST, [Idx(2), "country"], "Spain"),
        (WITH_STRUCTURED_DICT_LIST, [Idx(0)], {"name": "Bob", "age": 24, "country": "USA"}),
        (WITH_STRUCTURED_DICT_LIST, [Idx(1)], {"name": "John", "age": 29, "country": "China"}),
        (WITH_STRUCTURED_DICT_LIST, [Idx(2)], {"name": "Jack", "age": 20, "country": "Spain"}),
    ]
)
def test__lazy_load(data: bytes, path: list, expected: Any):
    assert lazy_load_bytes(data, path) == expected
