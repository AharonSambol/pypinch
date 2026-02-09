from dataclasses import dataclass
from typing import Dict, Optional


@dataclass
class Settings:
    encoding: Optional[str]
    use_tuples: bool
    use_pointers: bool
    pointers: Dict
