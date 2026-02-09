import pypinch
import os


def test_backend_detection():
    force_py = os.environ.get("PYPINCH_FORCE_PYTHON") == "1"
    if force_py:
        assert pypinch._BACKEND == "python"
    else:
        assert pypinch._BACKEND == "rust"