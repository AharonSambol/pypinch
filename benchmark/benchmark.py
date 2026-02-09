import copy
import json

import orjson
import math
import pickle
import time
import tracemalloc
import pypinch
from pypinch._pypinch import dump_bytes as pinch_rust_dump
from pypinch.serialize.serialize import dump_bytes as pinch_py_dump

# os.environ["MSGPACK_PUREPYTHON"] = "True"
import msgpack

from benchmark_displayer import display_benchmark

# a = 1
# print(load_bytes(pypinch.dump_bytes(a)))
# print(load_bytes(pypinch.dump_bytes(a)) == a)
# print(pypinch.dump_bytes(a))
# print(bytes(dump_bytes(a)))
# exit()
objects = [
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


# objects.clear()
with open("/home/aharon/Downloads/twitter.json") as f:
    objects.append(orjson.loads(f.read()))

with open("/home/aharon/Downloads/citm_catalog.json") as f:
    objects.append(orjson.loads(f.read()))

with open("/home/aharon/Downloads/canada.json") as f:
    objects.append(orjson.loads(f.read()))



def profile(name: str, func: callable):
    tracemalloc.start()
    start = time.perf_counter()
    try:
        res = func()
        try:
            end_len = len(res)
        except TypeError:
            end_len = 0
        elapsed = (time.perf_counter() - start) * 1000
        c, peak = tracemalloc.get_traced_memory()
    except Exception as e:
        print(e)
        # raise e
        return
    finally:
        tracemalloc.stop()
    return {"name": name, "elapsed": elapsed, "end_len": end_len, "peak_memory": peak}


for obj in objects:
    print()
    serialized = pypinch.dump_bytes(obj, use_pointers=True)
    # print(serialized)
    # print(dump_bytes(obj))
    unserialized = pypinch.load_bytes(serialized, modify_input=False)
    print()
    print(str(obj)[:120])
    print(str(unserialized)[:120])
    assert (isinstance(obj, float) and math.isnan(obj) and math.isnan(unserialized)) or unserialized == obj

    # serialized = dump_bytes(obj, settings=Settings(use_pointers=True))
    # unserialized = load_bytes(serialized, settings=LoadSettings(modify_input=True))
    # assert (isinstance(obj, float) and math.isnan(obj) and math.isnan(unserialized)) or unserialized == obj
# exit()

for obj in objects:
    print(str(obj)[:120])
    results = [
        profile("pypinch (python)", lambda: pinch_py_dump(obj)),
        profile("pypinch (rust)", lambda: pinch_rust_dump(obj)),
        profile("mine w pointer & str keys (python)",
                lambda: pinch_py_dump(obj, use_pointers=True, allow_non_string_keys=False)),
        profile("pypinch w pointers (rust)", lambda: pinch_rust_dump(obj, use_pointers=True)),
        profile("json", lambda: json.dumps(obj).encode()),
        profile("orjson", lambda: orjson.dumps(obj)),
        profile("pickle", lambda: pickle.dumps(obj)),
        profile("msgpack", lambda: msgpack.dumps(obj))
    ]
    # results.append(profile("str keys", lambda: dump_bytes(obj, settings=Settings(allow_non_string_keys=False))))
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

exit()
for obj in objects:
    print(str(obj)[:120])
    mine = dump_bytes(obj)
    mine2 = copy.deepcopy(mine)
    mine_w_pointer = dump_bytes(obj, settings=Settings(use_pointers=True, allow_non_string_keys=False))
    try:
        json_serialized = orjson.dumps(obj)
    except:
        json_serialized = None
    pickle_serialized = pickle.dumps(obj)
    try:
        msgpack_serialized = msgpack.dumps(obj)
    except:
        msgpack_serialized = None
    results = [
        profile("mine", lambda: load_bytes(mine, settings=LoadSettings(modify_input=True, use_tuples=True))),
        profile("mine from bytes", lambda: load_bytes(bytes(mine2), settings=LoadSettings(modify_input=False, use_tuples=True))),
        profile("mine w pointer & str keys", lambda: load_bytes(mine_w_pointer, settings=LoadSettings(modify_input=True, use_tuples=True))),
        *([profile("json", lambda: json.loads(json_serialized))] if json_serialized else []),
        *([profile("orjson", lambda: orjson.loads(json_serialized))] if json_serialized else []),
        profile("pickle", lambda: pickle.loads(pickle_serialized)),
        *([profile("msgpack", lambda: msgpack.loads(msgpack_serialized))] if msgpack_serialized else [])
    ]
    results = [x for x in results if x]

    display_benchmark(
        results,
        metrics={
            "elapsed": {
                "label": "Elapsed (ms)",
                "higher_is_better": False,
                "format": "{:.2f}",
            },
            "peak_memory": {
                "label": "Peak Mem",
                "higher_is_better": False,
                "format": "{:.1f}",
            },
        },
    )
    # profile("msgpack", lambda: msgpack.dumps(obj))
