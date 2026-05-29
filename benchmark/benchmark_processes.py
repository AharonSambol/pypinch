import ast
import base64
import os
import pickle
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import List, Tuple

import matplotlib.pyplot as plt
import numpy as np
import orjson


def generate_profiling_graph(results: List, output_file: Path) -> None:
    names = [r["name"] for r in results]

    mem_mib = [r["mem_mib"] for r in results]
    elapsed_ms = [r["elapsed"] for r in results]
    dump_mem = [r["dump_mem_mib"] for r in results]
    dump_elapsed = [r["dump_elapsed"] for r in results]

    x = np.arange(len(names))
    width = 0.35

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    # Plot 1: Memory Usage
    ax1.bar(x - width / 2, mem_mib, width, label='Load Mem', color='#3498db')
    ax1.bar(x + width / 2, dump_mem, width, label='Dump Mem', color='#2980b9')
    ax1.set_ylabel('Memory (KiB)')
    ax1.set_title('Memory Consumption')
    ax1.set_xticks(x)
    ax1.set_xticklabels(names, rotation=15)
    ax1.legend()
    ax1.grid(axis='y', linestyle='--', alpha=0.7)

    # Plot 2: Elapsed Time (ms)
    ax2.bar(x - width / 2, elapsed_ms, width, label='Load Time', color='#e67e22')
    ax2.bar(x + width / 2, dump_elapsed, width, label='Dump Time', color='#d35400')
    ax2.set_ylabel('Time (ms)')
    ax2.set_title('Execution Time')
    ax2.set_xticks(x)
    ax2.set_xticklabels(names, rotation=15)
    ax2.legend()
    ax2.grid(axis='y', linestyle='--', alpha=0.7)

    plt.tight_layout()
    output_filename = str(output_file).split(".")[0] + ".png"
    plt.savefig(output_filename)
    print(f"Graph saved as {output_filename}")


DUMP_TEMPLATE = """
import resource
import time
import pickle
import os
import gc

# To make any existing number in maxrss irrelevant
d = b"2" * 1024 * 1024 * 200

while not os.path.exists({start_file!r}):
    time.sleep(0.5)

with open({input_path!r}, "rb") as f:
    data = pickle.load(f)
gc.freeze()

{setup}

initial_rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss

start = time.perf_counter()
res = encode(data)
stop = time.perf_counter()

max_rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
with open({output_path!r}, "wb") as f:
    if type(res) is str:
        res = res.encode()
    f.write(res)

mem_mib = (max_rss - initial_rss)
time_ms = (stop - start) * 1000
print([mem_mib, time_ms, len(res)])
"""

LOAD_TEMPLATE = """
import resource
import time
import pickle
import os
import gc

# To make any existing number in maxrss irrelevant
d = b"2" * 1024 * 1024 * 200

while not os.path.exists({start_file!r}):
    time.sleep(0.5)
    
with open({path!r}, "rb") as f:
    data = f.read()
gc.freeze()

{setup}

initial_rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss

start = time.perf_counter()
decode(data)
stop = time.perf_counter()

max_rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
mem_mib = (max_rss - initial_rss)
time_ms = (stop - start) * 1000
print([mem_mib, time_ms])
"""


def profile(functions_to_profile: List[Tuple[str, str, str]], input_file: Path, result_file: Path) -> None:
    with tempfile.NamedTemporaryFile() as pickle_file:
        processes = []
        try:
            for i, (name, load_setup, dump_setup) in enumerate(functions_to_profile):
                if os.path.exists(f"start_file{i}"):
                    os.remove(f"start_file{i}")
                if os.path.exists(f"2start_file{i}"):
                    os.remove(f"2start_file{i}")

                dumped_file = open(f"output_file{i}.txt", "w")
                script = DUMP_TEMPLATE.format(input_path=pickle_file.name, output_path=dumped_file.name, setup=dump_setup, start_file=f"start_file{i}")
                dump_process = subprocess.Popen([sys.executable, "-c", script], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                script = LOAD_TEMPLATE.format(path=dumped_file.name, setup=load_setup, start_file=f"2start_file{i}")
                load_process = subprocess.Popen([sys.executable, "-c", script], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                processes.append((name, dump_process, f"start_file{i}", load_process, f"2start_file{i}", dumped_file))

            with open(input_file) as f:
                data_as_python = orjson.loads(f.read())
                if "binary" in input_file.name:
                    data_as_python["data"] = base64.b64decode(data_as_python["data"])
                pickle_file.write(pickle.dumps(data_as_python))
                pickle_file.flush()

            results = []
            for name, dump_process, dump_start_file, load_process, load_start_file, dumped_file in processes:
                print(name)
                try:
                    with open(dump_start_file, "w"):
                        output, err = dump_process.communicate()
                        print(err)
                        dump_mem_mib, dump_time_ms, end_len = ast.literal_eval(output.decode())
                    with open(load_start_file, "w"):
                        output, err = load_process.communicate()
                        print(err)
                        mem_mib, time_ms = ast.literal_eval(output.decode())
                except SyntaxError:
                    continue
                os.remove(dump_start_file)
                os.remove(load_start_file)
                os.remove(dumped_file.name)
                results.append({"name": name, "end_len": end_len, "mem_mib": mem_mib, "elapsed": time_ms, "dump_mem_mib": dump_mem_mib, "dump_elapsed": dump_time_ms})
        finally:
            for name, dump_process, dump_start_file, load_process, load_start_file, dumped_file in processes:
                try:
                    dump_process.kill()
                except Exception:
                    pass
                try:
                    load_process.kill()
                except Exception:
                    pass
                try:
                    os.remove(dump_start_file)
                except Exception:
                    pass
                try:
                    os.remove(load_start_file)
                except Exception:
                    pass
                try:
                    os.remove(dumped_file.name)
                except Exception:
                    pass

        results = [x for x in results if x]
        generate_profiling_graph(results, result_file)


if __name__ == '__main__':
    to_profile = [
        (
            "pypinch",
            "from pypinch._pypinch import load_bytes\ndecode=load_bytes",
            "from pypinch._pypinch import dump_bytes\nencode=dump_bytes",
        ),
        (
            "msgspec",
            "import msgspec\ndecode=msgspec.msgpack.decode",
            "import msgspec\nencode=msgspec.msgpack.encode",
        ),
        (
            "msgpack",
            "import msgpack\ndecode=msgpack.loads",
            "import msgpack\nencode=msgpack.dumps",
        ),
        (
            "json",
            "import json\ndecode=lambda x: json.loads(x.decode())",
            "import json\nencode=lambda x: json.dumps(x).encode()",
        ),
        (
            "orjson",
            "import orjson\ndecode=orjson.loads",
            "import orjson\nencode=orjson.dumps",
        ),
        (
            "simplejson",
            "import simplejson\ndecode=simplejson.loads",
            "import simplejson\nencode=simplejson.dumps",
        ),
        (
            "rapidjson",
            "import rapidjson\ndecode=rapidjson.loads",
            "import rapidjson\nencode=rapidjson.dumps",
        ),
        (
            "bson",
            "import bson\ndecode=bson.loads",
            "import bson\nencode=bson.dumps",
        ),
        (
            "ion",
            "import amazon.ion.simpleion as ion\ndecode=ion.loads",
            "import amazon.ion.simpleion as ion\nencode=ion.dumps",
        ),
    ]
    assets_dir = Path(__file__).parent.parent / "assets"
    input_directory = assets_dir / "benchmark_data"
    for file in os.listdir(input_directory):
        profile(to_profile, input_directory / file, assets_dir / "benchmark_results" / Path(file).name)