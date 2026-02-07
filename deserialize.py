import typing
from typing import Tuple, Optional, List

import math
import struct
import tracemalloc
from dataclasses import dataclass
from types import NoneType
from typing import Dict

from consts import NUMBER_BASE, ObjType, ENDING_FLAG, POSITIVE_INT_FLAG, FALSE_FLAG, TRUE_FLAG, NULL_FLAG, BYTES_FLAG, \
    LIST_FLAG, \
    DICT_FLAG, STR_KEY_DICT_FLAG, FLOAT_FLAG, STR_FLAG, NEGATIVE_INT_FLAG, EMPTY_STR_FLAG, EMPTY_BYTES_FLAG, \
    EMPTY_LIST_FLAG, EMPTY_DICT_FLAG, SMALL_INTS, CONSISTENT_TYPE_LIST_FLAG, INT_FLAG, BOOL_FLAG, POINTER_FLAG, \
    ByteLike, HEADER, CONSISTENT_TYPE_DICT_FLAG, REVERSE_SMALL_INTS
from exceptions import EncodingError, DecodingError


@dataclass
class Settings:
    modify_input: bool = False  # TODO
    encoding: Optional[str] = None
    use_tuples: bool = False    # TODO


DEFAULT_SETTINGS = Settings()


def decode_number(num: ByteLike, pointer: int, base: int = NUMBER_BASE) -> Tuple[int, int]:
    if num[pointer] != ENDING_FLAG:
        return num[pointer], pointer + 1
    power = res = 0
    pointer += 1
    while num[pointer] != ENDING_FLAG:
        res += num[pointer] * base ** power
        power += 1
        pointer += 1
    return res, pointer + 1


def load_bytes(buffer: ByteLike, settings: Settings = DEFAULT_SETTINGS) -> ObjType:
    if settings.modify_input and type(buffer) is bytearray:
        return load_bytes_from_bytearray(buffer, settings)
    else:
        return load_bytes_from_bytes(buffer, settings)


def load_bytes_from_bytearray(buffer: ByteLike, settings: Settings) -> ObjType:
    del buffer[:len(HEADER)]
    return deserialize_object(buffer, settings)
    

def deserialize_object(buffer: ByteLike, settings: Settings) -> ObjType:
    flag = buffer[0]
    del buffer[0]
    if flag == POSITIVE_INT_FLAG:
        num, end = decode_number(buffer, 0)
        del buffer[:end]
        return num
    elif flag == NEGATIVE_INT_FLAG:
        num, end = decode_number(buffer, 0)
        del buffer[:end]
        return -num
    elif flag == INT_FLAG:
        raise DecodingError("unexpected flag")
    elif flag == FLOAT_FLAG:
        num = struct.unpack("!d", buffer[:8])[0]
        del buffer[:8]
        return num
    elif flag == STR_FLAG:
        return deserialize_str(buffer, settings)
    elif flag == EMPTY_STR_FLAG:
        return ""
    elif flag == BYTES_FLAG:
        length, pointer = decode_number(buffer, 0)
        byts = buffer[pointer:pointer + length]
        del buffer[:pointer + length]
        return byts
    elif flag == EMPTY_BYTES_FLAG:
        return b""
    elif flag == BOOL_FLAG:
        raise DecodingError("unexpected flag")
    elif flag == TRUE_FLAG:
        return True
    elif flag == FALSE_FLAG:
        return False
    elif flag == NULL_FLAG:
        return None
    elif flag == LIST_FLAG:
        length, pointer = decode_number(buffer, 0)
        del buffer[:pointer]
        return [deserialize_object(buffer, settings) for _ in range(length)]
    elif flag == EMPTY_LIST_FLAG:
        return []
    elif flag == CONSISTENT_TYPE_LIST_FLAG:
        typ_flag = buffer[0]
        length, pointer = decode_number(buffer, 1)
        del buffer[:pointer]
        res_list = [None] * length
        if typ_flag == NULL_FLAG:
            return res_list
        elif typ_flag == INT_FLAG:
            res_list = typing.cast(List[int], res_list)
            for i in range(length):
                if buffer[0] == NUMBER_BASE-1:
                    num, pointer = decode_number(buffer, 1, base=NUMBER_BASE-1)
                    res_list[i] = -num
                    del buffer[:pointer]
                else:
                    num, pointer = decode_number(buffer, 0, base=NUMBER_BASE-1)
                    res_list[i] = num
                    del buffer[:pointer]
            return res_list
        elif typ_flag == BOOL_FLAG:
            res_list = typing.cast(List[bool], res_list)
            length_in_bytes = math.ceil(length / 8)
            # padding = 8 * length_in_bytes - length

            try:
                for i, byte in enumerate(buffer[:length_in_bytes]):
                    for j in range(8):
                        res_list[i * 8 + j] = (byte & 128) == 128
                        byte <<= 1
            except IndexError:
                pass
            return res_list
        else:
            raise Exception("not implemented yet")  # todo
    elif flag == DICT_FLAG:
        length, pointer = decode_number(buffer, 0)
        del buffer[:pointer]
        return {
            deserialize_object(buffer, settings): deserialize_object(buffer, settings)
            for _ in range(length)
        }
    elif flag == EMPTY_DICT_FLAG:
        return {}
    elif flag == STR_KEY_DICT_FLAG:
        length, pointer = decode_number(buffer, 0)
        del buffer[:pointer]
        return {
            deserialize_str(buffer, settings): deserialize_object(buffer, settings)
            for _ in range(length)
        }
    elif flag == CONSISTENT_TYPE_DICT_FLAG:
        raise Exception("not implemented yet") 	# todo
    elif flag == POINTER_FLAG:
        pass 	# todo
    else:
        return REVERSE_SMALL_INTS[flag]


def deserialize_str(buffer: bytearray, settings: Settings) -> str:
    length, pointer = decode_number(buffer, 0)
    encoded_str = buffer[pointer:pointer + length]
    del buffer[:pointer + length]
    string = encoded_str.decode(encoding=settings.encoding) if settings.encoding else encoded_str.decode()
    return string


def load_bytes_from_bytes(buffer: ByteLike, settings: Settings) -> ObjType:
    pass 	# todo