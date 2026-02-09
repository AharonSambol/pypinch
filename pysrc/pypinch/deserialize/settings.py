from dataclasses import dataclass
from typing import Dict, Optional


@dataclass
class Settings:
    use_tuples: bool
    use_pointers: bool
    pointers: Dict
