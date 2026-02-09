from dataclasses import dataclass
from typing import Dict


@dataclass
class Settings:
    allow_non_string_keys: bool
    modify_input: bool
    encoding: str
    use_pointers: bool
    pointers: Dict
    serialize_dates: bool
