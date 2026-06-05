import os

from .deserialize.lazy_load import Idx
from .exceptions import SerializationError, DeserializationError
from .serialize.settings import CustomType

FORCE_PYTHON = os.environ.get("PYPINCH_FORCE_PYTHON")

_pypinch = None
_BACKEND = None
if not FORCE_PYTHON:
    try:
        from ._pypinch import *
    except ImportError:
        pass

if _pypinch is not None:
    _BACKEND = "rust"
    dump_bytes = _pypinch.dump_bytes
    load_bytes = _pypinch.load_bytes
    lazy_load_bytes = _pypinch.lazy_load_bytes
    bytes_check_if_contains = _pypinch.bytes_check_if_contains
else:
    _BACKEND = "python"
    from .serialize.serialize import dump_bytes
    from .deserialize.deserialize import load_bytes
    from .deserialize.lazy_load import lazy_load_bytes, bytes_check_if_contains

lazy_unpinch = lazy_load_bytes
pinch = dump_bytes
unpinch = load_bytes
