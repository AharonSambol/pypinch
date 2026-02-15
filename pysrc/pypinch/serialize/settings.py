from dataclasses import dataclass
from typing import Dict


@dataclass
class Settings:
    allow_non_string_keys: bool
    modify_input: bool
    pointers: Dict[str, int]
    serialize_dates: bool
    str_count: int
