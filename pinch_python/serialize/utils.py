from types import NoneType

from consts import NUMBER_BASE, ENDING_FLAG, INT_FLAG, STR_FLAG, BOOL_FLAG, NULL_FLAG, BYTES_FLAG, FLOAT_FLAG
from exceptions import EncodingError


def encode_number(buffer: bytearray, num: int, base: int = NUMBER_BASE) -> None:
    if num < base:
        buffer.append(num)
    else:
        buffer.append(ENDING_FLAG)
        while num:
            num, remainder = divmod(num, base)
            buffer.append(remainder)
        buffer.append(ENDING_FLAG)
