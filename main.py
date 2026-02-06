import copy
import gc
import json
import sys

import orjson
import math
import os
import pickle
import time
import tracemalloc

os.environ["MSGPACK_PUREPYTHON"] = "True"
import msgpack

from benchmark_displayer import display_benchmark
from serialize import dump_bytes, Settings

objects = [
    1231,
    1,
    332634,
    5437890568343289547384,
    0,
    -1231,
    -1,
    -332634,
    -5437890568343289547384,

    1.000000000001,
    100000000000000000000000000000000000000000.1,
    23423523.543262346234,
    4.4,
    -1.000000000001,
    -100000000000000000000000000000000000000000.1,
    -23423523.543262346234,
    -4.4,

    math.nan,
    math.inf,
    -math.inf,

    "afsag",
    "092u384oiwjrklsgmfoisgjldkxfmoweij;lksgzwaoi;elgjskznwoi;jetlaksfdnv" * 1_000_000,
    "😎",
    "לא",
    "",
    #
    # b"1234",
    # b"abcdefghijklmnopqrstuvwxyz",
    # b"",
    #
    # None,
    # True,
    # False,
    #
    # [1, 2, 3],
    # list(range(50, 1000)),
    # [1, "asdg", b"234sa", 4.5, [1, 2, 3, 4, 5], False, [], None],
    #
    {"a": "sdgaeiogn", "waegw": 123, "sdagweg": list(range(10)), "aegsag": {"asdg": 235, "Asg": b"asg"}},
    {1: "afdbda", "ar": "23wesd", False: 23453, 1234: 12324356, "": {"sgdfn32rwefsdvre": 34}},

    {"a": "sdgaeiogn", "content": b"1243567" * 1024 * 1024 * 50, "sdagweg": list(range(10)), "aegsag": {"asdg": 235, "Asg": b"asg"}},
    [True, False, False] * 1000,
]
# objects.clear()

recursive_obj = {"a": "sdgaeiogn", "waegw": 123, "sdagweg": list(range(10)), "aegsag": {"asdg": 235, "Asg": "asg"}, "asgdagwe": "asgdouvjknmwefasdvsaivdljnm,efsdvlnk", "saegas": "asrgiufhkjnaseodigfun", "sargasdba": {"Asdg": "segaserhbaewh", "segaseh": "aehsarjt"}}
for i in range(17):
    recursive_obj[f"self{i}"] = copy.deepcopy(recursive_obj)
objects.append(recursive_obj)


def profile(name: str, func: callable):
    tracemalloc.start()
    start = time.perf_counter()
    try:
        res = func()
        end_len = len(res)
        elapsed = (time.perf_counter() - start) * 1000
        c, peak = tracemalloc.get_traced_memory()
    except Exception as e:
        print(e)
        return
    finally:
        tracemalloc.stop()
    return {"name": name, "elapsed": elapsed, "end_len": end_len, "peak_memory": peak}
from pympler import asizeof

for obj in objects:
    print(str(obj)[:120])
    print(asizeof.asizeof(obj))
    results = []
    results.append(profile("mine", lambda: dump_bytes(obj)))
    results.append(profile("mine w pointer", lambda: dump_bytes(obj, settings=Settings(use_pointers=True))))
    results.append(profile("json", lambda: json.dumps(obj).encode()))
    results.append(profile("orjson", lambda: orjson.dumps(obj)))
    results.append(profile("pickle", lambda: pickle.dumps(obj)))
    results.append(profile("msgpack", lambda: msgpack.dumps(obj)))
    results = [x for x in results if x]

    display_benchmark(
        results,
        metrics={
            "elapsed": {
                "label": "Elapsed (ms)",
                "higher_is_better": False,
                "format": "{:.2f}",
            },
            "end_len": {
                "label": "End Len",
                "higher_is_better": False,
                "format": "{:.1f}",
            },
            "peak_memory": {
                "label": "Peak Mem",
                "higher_is_better": False,
                "format": "{:.1f}",
            },
        },
    )
    # profile("msgpack", lambda: msgpack.dumps(obj))
