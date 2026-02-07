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

from deserialize import load_bytes, decode_number

from deserialize import Settings as LoadSettings

os.environ["MSGPACK_PUREPYTHON"] = "True"
import msgpack

from benchmark_displayer import display_benchmark
from serialize import dump_bytes, Settings, encode_number


objects = [
    # 1231,
    # 1,
    # 332634,
    # 5437890568343289547384,
    # 0,
    # -1231,
    # -1,
    # -332634,
    # -5437890568343289547384,
    #
    # 1.000000000001,
    # 100000000000000000000000000000000000000000.1,
    # 23423523.543262346234,
    # 4.4,
    # -1.000000000001,
    # -100000000000000000000000000000000000000000.1,
    # -23423523.543262346234,
    # -4.4,
    #
    # math.nan,
    # math.inf,
    # -math.inf,
    #
    # "afsag",
    # "092u384oiwjrklsgmfoisgjldkxfmoweij;lksgzwaoi;elgjskznwoi;jetlaksfdnv" * 1_000_000,
    # "😎",
    # "לא",
    # "",
    #
    # b"1234",
    # b"abcdefghijklmnopqrstuvwxyz",
    # b"",
    #
    # None,
    # True,
    # False,
    #
    # [None] * 10,
    [b"1234", b"asgsa", b"sgaeg4we"],
    [0.1, 0.2, 0.3, 0.4],
    [-1, 0, 1, 2, 3],
    list(range(50, 1000)),
    ["aaaa", "aaaa", "aaaa"],
    [1, "asdg", b"234sa", 4.5, [1, 2, 3, 4, 5], False, [], None],

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
        return
    finally:
        tracemalloc.stop()
    return {"name": name, "elapsed": elapsed, "end_len": end_len, "peak_memory": peak}


for obj in objects:
    serialized = dump_bytes(obj)
    unserialized = load_bytes(serialized, settings=LoadSettings(modify_input=False))
    assert (isinstance(obj, float) and math.isnan(obj) and math.isnan(unserialized)) or unserialized == obj

    # serialized = dump_bytes(obj, settings=Settings(use_pointers=True))
    # unserialized = load_bytes(serialized, settings=LoadSettings(modify_input=True))
    # assert (isinstance(obj, float) and math.isnan(obj) and math.isnan(unserialized)) or unserialized == obj
# exit()

# for obj in objects:
#     print(str(obj)[:120])
#     results = [
#         profile("mine", lambda: dump_bytes(obj)),
#         # profile("mine w pointer", lambda: dump_bytes(obj, settings=Settings(use_pointers=True))),
#         profile("mine w pointer & str keys",
#                lambda: dump_bytes(obj, settings=Settings(use_pointers=True, allow_non_string_keys=False))),
#         profile("json", lambda: json.dumps(obj).encode()),
#         profile("orjson", lambda: orjson.dumps(obj)),
#         profile("pickle", lambda: pickle.dumps(obj)),
#         profile("msgpack", lambda: msgpack.dumps(obj))
#     ]
#     # results.append(profile("str keys", lambda: dump_bytes(obj, settings=Settings(allow_non_string_keys=False))))
#     results = [x for x in results if x]
#
#     display_benchmark(
#         results,
#         metrics={
#             "elapsed": {
#                 "label": "Elapsed (ms)",
#                 "higher_is_better": False,
#                 "format": "{:.2f}",
#             },
#             "end_len": {
#                 "label": "End Len",
#                 "higher_is_better": False,
#                 "format": "{:.1f}",
#             },
#             "peak_memory": {
#                 "label": "Peak Mem",
#                 "higher_is_better": False,
#                 "format": "{:.1f}",
#             },
#         },
#     )
#     # profile("msgpack", lambda: msgpack.dumps(obj))

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
