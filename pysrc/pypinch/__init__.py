import os

FORCE_PYTHON = os.environ.get("PYPINCH_FORCE_PYTHON")

_pypinch = None
if not FORCE_PYTHON:
    try:
        from ._pypinch import *
    except ImportError:
        _pypinch = None

if _pypinch is not None:
    dump_bytes = _pypinch.dump_bytes
    # dump_bytes = _pypinch.load_bytes
else:
    from .serialize.serialize import dump_bytes
from .deserialize.deserialize import load_bytes

pinch = dump_bytes
unpinch = load_bytes
