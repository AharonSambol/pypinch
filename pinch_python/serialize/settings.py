from dataclasses import dataclass
from typing import Dict


@dataclass
class Settings:
    allow_non_string_keys: bool = True
    modify_input: bool = False  # TODO
    encoding: str = None
    use_pointers: bool = False
    pointers: Dict = None
