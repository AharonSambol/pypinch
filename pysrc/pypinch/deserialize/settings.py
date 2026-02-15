from dataclasses import dataclass
from typing import List


@dataclass
class Settings:
    use_tuples: bool
    pointers: List[str]     # TODO: for small strings, will they be saved in her multiple times?
