from dataclasses import dataclass
from typing import List, Dict, Any, Callable, Optional


@dataclass
class Settings:
    use_tuples: bool
    pointers: List[str]
    custom_types: Optional[Dict[Any, Callable[[bytes], Any]]]
