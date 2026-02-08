from typing import Union

ObjType = Union[int, float, str, bool, bytes, list, tuple, dict, None]
ByteLike = Union[bytes, bytearray]
NUMBER_BASE = 255
ENDING_FLAG = 255
HEADER = b"<o>"
BIG_ENDIAN_DOUBLE_FORMAT = "!d"
BYTES_IN_DOUBLE = 8
NUMBER_OF_BITS_IN_BYTE = 8
LEFTMOST_BIT_MASK = 128


nums = iter(range(255))
POSITIVE_INT_FLAG = bytes([next(nums)])[0]
NEGATIVE_INT_FLAG = bytes([next(nums)])[0]
INT_FLAG = bytes([next(nums)])[0]
FLOAT_FLAG = bytes([next(nums)])[0]
STR_FLAG = bytes([next(nums)])[0]
EMPTY_STR_FLAG = bytes([next(nums)])[0]
BYTES_FLAG = bytes([next(nums)])[0]
EMPTY_BYTES_FLAG = bytes([next(nums)])[0]
BOOL_FLAG = bytes([next(nums)])[0]
TRUE_FLAG = bytes([next(nums)])[0]
FALSE_FLAG = bytes([next(nums)])[0]
NULL_FLAG = bytes([next(nums)])[0]
LIST_FLAG = bytes([next(nums)])[0]
EMPTY_LIST_FLAG = bytes([next(nums)])[0]
CONSISTENT_TYPE_LIST_FLAG = bytes([next(nums)])[0]
DICT_FLAG = bytes([next(nums)])[0]
EMPTY_DICT_FLAG = bytes([next(nums)])[0]
STR_KEY_DICT_FLAG = bytes([next(nums)])[0]
CONSISTENT_TYPE_DICT_FLAG = bytes([next(nums)])[0]
POINTER_FLAG = bytes([next(nums)])[0]

SMALL_INTS = {i: bytes([x])[0] for i, x in enumerate(nums)}
REVERSE_SMALL_INTS = {v: k for k, v in SMALL_INTS.items()}
