from dataclasses import dataclass
from typing import Dict, Any, Optional, Type, Callable


@dataclass
class CustomType:
    identifier: Any
    converter: Callable[[Any], Any]
    include_subclasses: bool = False
    one_way: bool = False


@dataclass
class Settings:
    allow_non_string_keys: bool
    pointers: Dict[str, int]
    serialize_dates: bool
    str_count: int
    custom_types: Optional[Dict[Type, CustomType]]
