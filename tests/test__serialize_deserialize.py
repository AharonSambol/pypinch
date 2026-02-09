import copy
import datetime
import math
from zoneinfo import ZoneInfo

import pytest

from src import pinch_python

ALL_TYPES_OF_OBJECTS = [
    (1231,),
    (1,),
    (332634,),
    (5437890568343289547384,),
    (0,),
    (-1231,),
    (-1,),
    (-332634,),
    (-5437890568343289547384,),

    (1.000000000001,),
    (100000000000000000000000000000000000000000.1,),
    (23423523.543262346234,),
    (4.4,),
    (-1.000000000001,),
    (-100000000000000000000000000000000000000000.1,),
    (-23423523.543262346234,),
    (-4.4,),

    (math.inf,),
    (-math.inf,),

    ("afsag",),
    pytest.param("092u384oiwjrklsgmfoisgjldkxfmoweij;lksgzwaoi;elgjskznwoi;jetlaksfdnv" * 1_000_000, id="long string"),
    pytest.param("".join(chr(i) for i in range(10000)), id="lots of unicode"),
    ("",),

    (b"1234",),
    (b"abcdefghijklmnopqrstuvwxyz",),
    (b"",),
    pytest.param(b"".join(bytes(i) for i in range(10000)), id="lots of bytes"),

    (None,),
    (True,),
    (False,),

    ([None] * 10,),
    ([b"1234", b"asgsa", b"sgaeg4we"],),
    ([0.1, 0.2, 0.3, 0.4],),
    ([-91, 0, 1, 2, 3, 4, 5, 6, 7, 8],),
    (list(range(50, 1000)),),
    (["aaaa", "aaaa", "aaaa"],),
    ([1, "asdg", b"234sa", 4.5, [1, 2, 3, 4, 5], False, [], None],),

    ({"a": "sdgaeiogn", "waegw": 123, "sdagweg": list(range(10)), "aegsag": {"asdg": 235, "Asg": b"asg"}},),
    ({1: "afdbda", "ar": "23wesd", False: 23453, 1234: 12324356, "": {"sgdfn32rwefsdvre": 34}},),

    ({"a": "sdgaeiogn", "content": b"1243567" * 1024 * 1024 * 50, "sdagweg": list(range(10)),
      "aegsag": {"asdg": 235, "Asg": b"asg"}},),
    pytest.param([True, False, False] * 1000, id="list of booleans"),
]


@pytest.mark.parametrize(
    ["obj"],
    ALL_TYPES_OF_OBJECTS
)
def test__serialize_deserialize__modify_input(obj):
    # Arrange
    original_obj = copy.deepcopy(obj)

    # Act
    serialized = pinch_python.dump_bytes(obj, modify_input=True)
    unserialized = pinch_python.load_bytes(serialized)

    # Assert
    assert unserialized == original_obj


def test__serialize_deserialize__nan():
    # Act
    serialized = pinch_python.dump_bytes(float("nan"))
    unserialized = pinch_python.load_bytes(serialized)

    # Assert
    assert math.isnan(unserialized)

@pytest.mark.parametrize(
    ["input_tuple", "expected"],
    [
        (tuple(), []),
        ((1, 2, 3), [1, 2, 3]),
        ((((),),), [[[]]]),
        ((1, None, 2.3, "rtjg", b"5y4rthf", [], {}, tuple()), [1, None, 2.3, "rtjg", b"5y4rthf", [], {}, []]),
    ]
)
def test__tuples_serialize_deserialize__into_list(input_tuple, expected):
    # Act
    serialized = pinch_python.dump_bytes(float("nan"))
    unserialized = pinch_python.load_bytes(serialized)

    # Assert
    assert math.isnan(unserialized)

@pytest.mark.parametrize(
    ["obj", "expected"],
    [
        (datetime.datetime(2026, 10, 4, 23, 2, 9, 53, tzinfo=datetime.timezone.utc), "2026-10-04T23:02:09.000053+00:00"),
        (
            [
                datetime.datetime(2026, 10, 4, 23, 2, 9, 53, tzinfo=datetime.timezone.utc),
                datetime.datetime(1995, 1, 2, 6, 3, 18, tzinfo=ZoneInfo("America/Los_Angeles")),
                datetime.datetime(2050, 4, 1, tzinfo=ZoneInfo("Asia/Kolkata")),
            ],
            [
                "2026-10-04T23:02:09.000053+00:00",
                "1995-01-02T06:03:18-08:00",
                "2050-04-01T00:00:00+05:30",
            ]
        ),
    ]
)
def test__serialize_unknown_types(obj, expected):
    # Act
    serialized = pinch_python.dump_bytes(obj)
    unserialized = pinch_python.load_bytes(serialized)

    # Assert
    assert unserialized == expected


@pytest.mark.parametrize(
    ["obj"],
    ALL_TYPES_OF_OBJECTS
)
def test__serialize_deserialize__with_pointers(obj):
    # Arrange
    original_obj = copy.deepcopy(obj)

    # Act
    serialized = pinch_python.dump_bytes(obj, use_pointers=True)
    unserialized = pinch_python.load_bytes(serialized)

    # Assert
    assert unserialized == original_obj


@pytest.mark.parametrize(
    ["obj"],
    ALL_TYPES_OF_OBJECTS
)
def test__serialize_deserialize__dont_modify_input(obj):
    # Arrange
    original_obj = copy.deepcopy(obj)

    # Act
    serialized = pinch_python.dump_bytes(obj, modify_input=False)
    unserialized = pinch_python.load_bytes(serialized)

    # Assert
    assert unserialized == original_obj
    assert obj == original_obj


@pytest.mark.parametrize(
    ["obj"],
    ALL_TYPES_OF_OBJECTS
)
def test__serialize_deserialize__dont_modify_serialized_data(obj):
    # Arrange
    original_obj = copy.deepcopy(obj)
    serialized = pinch_python.dump_bytes(obj)
    original_serialized = copy.deepcopy(serialized)

    # Act
    unserialized = pinch_python.load_bytes(serialized, modify_input=False)

    # Assert
    assert unserialized == original_obj
    assert serialized == original_serialized


@pytest.mark.parametrize(
    ["obj"],
    ALL_TYPES_OF_OBJECTS
)
def test__serialize_deserialize__modify_serialized_data(obj):
    # Arrange
    original_obj = copy.deepcopy(obj)
    serialized = pinch_python.dump_bytes(obj)

    # Act
    unserialized = pinch_python.load_bytes(serialized, modify_input=True)

    # Assert
    assert unserialized == original_obj


@pytest.mark.parametrize(
    ["obj"],
    ALL_TYPES_OF_OBJECTS
)
def test__serialize_deserialize__bytes_serialized_data(obj):
    # Arrange
    original_obj = copy.deepcopy(obj)
    serialized = bytes(pinch_python.dump_bytes(obj))

    # Act
    unserialized = pinch_python.load_bytes(serialized, modify_input=True)

    # Assert
    assert unserialized == original_obj


@pytest.mark.parametrize(
    ["obj", "expected"],
    [
        ([1, 2, 3], (1, 2, 3)),
        ({"a": [], "b": [[1], ["f"]]}, {"a": tuple(), "b": ((1,), ("f",))}),
    ]
)
def test__serialize_deserialize__use_tuples(obj, expected):
    # Act
    serialized = pinch_python.dump_bytes(obj)
    unserialized = pinch_python.load_bytes(serialized, use_tuples=True, modify_input=True)

    # Assert
    assert unserialized == expected


@pytest.mark.parametrize(
    ["obj", "encoding"],
    [
        ("abcdef", "utf-16"),
        ("abcdef", "utf-32-le"),
        ("abcdef", "ascii"),
        ("abcdef", "cp775"),
        ("abcdef", "windows-1256"),
    ]
)
def test__serialize_deserialize__with_encoding(obj, encoding):
    # Arrange
    original_object = copy.deepcopy(obj)

    # Act
    serialized = pinch_python.dump_bytes(obj, encoding=encoding)
    unserialized = pinch_python.load_bytes(serialized, modify_input=True, encoding=encoding)

    # Assert
    assert unserialized == original_object
