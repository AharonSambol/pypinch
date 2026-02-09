import os

FORCE_PYTHON = os.environ.get("PYPINCH_FORCE_PYTHON")

_pypinch = None
_BACKEND = None
if not FORCE_PYTHON:
    try:
        from ._pypinch import *
    except ImportError:
        _pypinch = None

if _pypinch is not None:
    _BACKEND = "rust"
    dump_bytes = _pypinch.dump_bytes
    load_bytes = _pypinch.load_bytes
else:
    _BACKEND = "python"
    from .serialize.serialize import dump_bytes
    from .deserialize.deserialize import load_bytes

pinch = dump_bytes
unpinch = load_bytes
