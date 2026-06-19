from uuid import UUID

import pytest

import pypinch
from pypinch.exceptions import SerializationError
from pypinch.serialize.settings import CustomType


class CustomClass:
    def __init__(self, a, b, c):
        self.a = a
        self.b = b
        self.c = c

    def serialize(self):
        return f"{self.a},{self.b},{self.c}"

    @classmethod
    def deserialize(cls, string):
        a, b, c = string.split(",")
        return CustomClass(int(a), b, c)

    def __eq__(self, other):
        return self.a == other.a and self.b == other.b and self.c == other.c


@pytest.mark.parametrize(
    ["obj"],
    [
        (UUID('6076c2ca-8847-44f7-94a7-175949ce6e63'),),
        (
            [
                UUID('6076c2ca-8847-44f7-94a7-175949ce6e63'),
                CustomClass(0, "hello", "world"),
            ],
        ),
        (
            [
                UUID('6076c2ca-8847-44f7-94a7-175949ce6e63'),
                UUID('6076c2ca-8847-44f7-94a7-175949ce6e63'),
                UUID('6076c2ca-8847-44f7-94a7-175949ce6e63'),
                UUID('6076c2ca-8847-44f7-94a7-175949ce6e63'),
                UUID('6076c2ca-8847-44f7-94a7-175949ce6e63'),
            ],
        ),
        (
            {
                UUID('6076c2ca-8847-44f7-94a7-175949ce6e63'): CustomClass(-234123, "", "asdhgoe;iwslfjkdnzbj!"),
            },
        ),
    ]
)
def test__serialize_unknown_types(obj):
    # Act
    serialized = pypinch.dump_bytes(obj, custom_types={
        UUID: CustomType(identifier=0, converter=lambda x: str(x)),
        CustomClass: CustomType(identifier="hello?", converter=lambda x: x.serialize())
    })
    deserialized = pypinch.load_bytes(serialized, custom_types={
        0: lambda x: UUID(x),
        "hello?": lambda x: CustomClass.deserialize(x)
    })

    # Assert
    assert deserialized == obj


class InheritingCustomClass(CustomClass):
    pass

def test__serialize_inheriting_custom_type():
    # Act
    serialized = pypinch.dump_bytes(InheritingCustomClass(1, "2", "3"), custom_types={
        CustomClass: CustomType(identifier="hello?", converter=lambda x: x.serialize(), include_subclasses=True)
    })
    deserialized = pypinch.load_bytes(serialized, custom_types={
        0: lambda x: UUID(x),
        "hello?": lambda x: CustomClass.deserialize(x)
    })

    # Assert
    assert deserialized == CustomClass(1, "2", "3")

def test__dont_serialize_inheriting_custom_type():
    # Act
    with pytest.raises(SerializationError):
        serialized = pypinch.dump_bytes(InheritingCustomClass(1, "2", "3"), custom_types={
            CustomClass: CustomType(identifier="hello?", converter=lambda x: x.serialize())
        })

def test__one_way():
    # Act
    serialized = pypinch.dump_bytes(CustomClass(1, "2", "3"), custom_types={
        CustomClass: CustomType(identifier="hello?", converter=lambda x: x.__dict__, one_way=True)
    })

    # Assert
    assert pypinch.load_bytes(serialized) == {"a": 1, "b": "2", "c": "3"}
