import pytest

from pypinch.exceptions import DeserializationError
import pypinch


SERIALIZED = pypinch.dump_bytes(
    [{}, {"a": "1", "b": 2}, {1: "a"}, [1, 2, 3, 4], [True, False] * 10, [1, True, b"Aag" * 100], 1234567865432135456857435241356878576435241321435465879586435241312345678654321354568574352413568785764352413214354658795864352413]
)


@pytest.mark.parametrize(
    ["data"],
    [
        (SERIALIZED[:i],)
        for i in range(len(SERIALIZED))
    ]
)
def test__load_invalid_data__dies_gracefully(data: bytes):
    try:
        pypinch.load_bytes(data)
    except DeserializationError:
        return True
    except:
        assert False
