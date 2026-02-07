import gc
import typing
from typing import Tuple, Optional, List

import math
import struct
import tracemalloc
from dataclasses import dataclass
from typing import Dict

from consts import NUMBER_BASE, ObjType, ENDING_FLAG, POSITIVE_INT_FLAG, FALSE_FLAG, TRUE_FLAG, NULL_FLAG, BYTES_FLAG, \
    LIST_FLAG, \
    DICT_FLAG, STR_KEY_DICT_FLAG, FLOAT_FLAG, STR_FLAG, NEGATIVE_INT_FLAG, EMPTY_STR_FLAG, EMPTY_BYTES_FLAG, \
    EMPTY_LIST_FLAG, EMPTY_DICT_FLAG, CONSISTENT_TYPE_LIST_FLAG, INT_FLAG, BOOL_FLAG, POINTER_FLAG, \
    ByteLike, HEADER, CONSISTENT_TYPE_DICT_FLAG, REVERSE_SMALL_INTS
from exceptions import DecodingError


@dataclass
class Settings:
    modify_input: bool = False
    encoding: Optional[str] = None
    use_tuples: bool = False    # TODO
    use_pointers: bool = True
    stop_gc: bool = False
    _pointers: Dict = None


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
    if settings.stop_gc:
        try:
            gc.freeze()
            settings.stop_gc = False
            return load_bytes(buffer, settings)
        finally:
            settings.stop_gc = True
            gc.unfreeze()
    if settings.use_pointers:
        settings._pointers = {}
    if settings.modify_input and type(buffer) is bytearray:
        return load_bytes_from_bytearray(buffer, settings)
    else:
        return load_bytes_from_bytes(buffer, settings)


def load_bytes_from_bytearray(buffer: bytearray, settings: Settings) -> ObjType:
    original_buffer_len = len(buffer)
    del buffer[:len(HEADER)]
    return deserialize_object_from_bytearray(buffer, original_buffer_len, settings)
    

def deserialize_object_from_bytearray(buffer: bytearray, original_buffer_len: int, settings: Settings) -> ObjType:
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
        return deserialize_str_from_bytearray(buffer, original_buffer_len, settings)
    elif flag == EMPTY_STR_FLAG:
        return ""
    elif flag == BYTES_FLAG:
        length, pointer = decode_number(buffer, 0)
        byts = buffer[pointer:pointer + length]
        del buffer[:pointer + length]
        return bytes(byts)
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
        if settings.use_tuples:
            return tuple(deserialize_object_from_bytearray(buffer, original_buffer_len, settings) for _ in range(length))
        return [deserialize_object_from_bytearray(buffer, original_buffer_len, settings) for _ in range(length)]
    elif flag == EMPTY_LIST_FLAG:
        return tuple() if settings.use_tuples else []
    elif flag == CONSISTENT_TYPE_LIST_FLAG:
        typ_flag = buffer[0]
        length, pointer = decode_number(buffer, 1)
        del buffer[:pointer]
        if typ_flag == NULL_FLAG:
            return ((None,) if settings.use_tuples else [None]) * length
        elif typ_flag == INT_FLAG:
            res_list = typing.cast(List[int], [None] * length)
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
            res_list = typing.cast(List[bool], [None] * length)
            length_in_bytes = math.ceil(length / 8)

            try:
                for i, byte in enumerate(buffer[:length_in_bytes]):
                    for j in range(8):
                        res_list[i * 8 + j] = (byte & 128) == 128
                        byte <<= 1
            except IndexError:
                pass
            return res_list
        elif typ_flag == BYTES_FLAG:
            res_list = typing.cast(List[bytes], [None] * length)
            for i in range(length):
                length, pointer = decode_number(buffer, 0)
                res_list[i] = bytes(buffer[pointer:pointer + length])
                del buffer[:pointer + length]
            return res_list
        elif typ_flag == STR_FLAG:
            if settings.use_tuples:
                return tuple(deserialize_str_from_bytearray(buffer, original_buffer_len, settings) for _ in range(length))
            return [deserialize_str_from_bytearray(buffer, original_buffer_len, settings) for _ in range(length)]
        elif typ_flag == FLOAT_FLAG:
            res_list = typing.cast(List[float], [None] * length)
            for i in range(length):
                res_list[i] = struct.unpack("!d", buffer[:8])[0]
                del buffer[:8]
            return res_list
        else:
            raise DecodingError(f"Unexpected type flag: {typ_flag}")
    elif flag == DICT_FLAG:
        length, pointer = decode_number(buffer, 0)
        del buffer[:pointer]
        return {
            deserialize_object_from_bytearray(buffer, original_buffer_len, settings): deserialize_object_from_bytearray(buffer, original_buffer_len, settings)
            for _ in range(length)
        }
    elif flag == EMPTY_DICT_FLAG:
        return {}
    elif flag == STR_KEY_DICT_FLAG:
        length, pointer = decode_number(buffer, 0)
        del buffer[:pointer]
        return {
            deserialize_str_from_bytearray(buffer, original_buffer_len, settings): deserialize_object_from_bytearray(buffer, original_buffer_len, settings)
            for _ in range(length)
        }
    elif flag == CONSISTENT_TYPE_DICT_FLAG:
        raise Exception("not implemented yet") 	# todo
    elif flag == POINTER_FLAG:
        position, pointer = decode_number(buffer, 0)
        del buffer[:pointer]
        return settings._pointers[position]
    else:
        return REVERSE_SMALL_INTS[flag]


def deserialize_str_from_bytearray(buffer: bytearray, original_buffer_len: int, settings: Settings) -> str:
    position = original_buffer_len - len(buffer)
    length, pointer = decode_number(buffer, 0)
    encoded_str = buffer[pointer:pointer + length]
    del buffer[:pointer + length]
    string = encoded_str.decode(encoding=settings.encoding) if settings.encoding else encoded_str.decode()
    if settings.use_pointers:
        settings._pointers[position] = string
    return string


def load_bytes_from_bytes(buffer: bytes, settings: Settings) -> ObjType:
    return deserialize_object(buffer, len(HEADER), settings)[0]


def deserialize_object(buffer: bytes, pointer: int, settings: Settings) -> (ObjType, int):
    flag = buffer[pointer]
    pointer += 1
    if flag == POSITIVE_INT_FLAG:
        return decode_number(buffer, pointer)
    elif flag == NEGATIVE_INT_FLAG:
        num, pointer = decode_number(buffer, pointer)
        return -num, pointer
    elif flag == INT_FLAG:
        raise DecodingError("unexpected flag")
    elif flag == FLOAT_FLAG:
        num = struct.unpack("!d", buffer[pointer:pointer + 8])[0]
        return num, pointer + 8
    elif flag == STR_FLAG:
        return deserialize_str(buffer, pointer, settings)
    elif flag == EMPTY_STR_FLAG:
        return "", pointer
    elif flag == BYTES_FLAG:
        length, pointer = decode_number(buffer, pointer)
        return bytes(buffer[pointer:pointer + length]), pointer + length
    elif flag == EMPTY_BYTES_FLAG:
        return b"", pointer
    elif flag == BOOL_FLAG:
        raise DecodingError("unexpected flag")
    elif flag == TRUE_FLAG:
        return True, pointer
    elif flag == FALSE_FLAG:
        return False, pointer
    elif flag == NULL_FLAG:
        return None, pointer
    elif flag == LIST_FLAG:
        length, pointer = decode_number(buffer, pointer)
        res_list = [None] * length
        for i in range(length):
            res_list[i], pointer = deserialize_object(buffer, pointer, settings)
        return res_list, pointer
    elif flag == EMPTY_LIST_FLAG:
        return (tuple() if settings.use_tuples else []), pointer
    elif flag == CONSISTENT_TYPE_LIST_FLAG:
        typ_flag = buffer[pointer]
        length, pointer = decode_number(buffer, pointer + 1)
        if typ_flag == NULL_FLAG:
            return ((None,) if settings.use_tuples else [None]) * length, pointer
        elif typ_flag == INT_FLAG:
            res_list = typing.cast(List[int], [None] * length)
            for i in range(length):
                if buffer[pointer] == NUMBER_BASE - 1:
                    num, pointer = decode_number(buffer, pointer + 1, base=NUMBER_BASE - 1)
                    res_list[i] = -num
                else:
                    num, pointer = decode_number(buffer, pointer, base=NUMBER_BASE - 1)
                    res_list[i] = num
            return res_list, pointer
        elif typ_flag == BOOL_FLAG:
            res_list = typing.cast(List[bool], [None] * length)
            length_in_bytes = math.ceil(length / 8)
            try:
                for i in range(length_in_bytes):
                    byte = buffer[pointer + i]
                    for j in range(8):
                        res_list[i * 8 + j] = (byte & 128) == 128
                        byte <<= 1
            except IndexError:
                pass
            return res_list, pointer + length_in_bytes
        elif typ_flag == BYTES_FLAG:
            res_list = typing.cast(List[bytes], [None] * length)
            for i in range(length):
                bytes_length, pointer = decode_number(buffer, pointer)
                res_list[i] = bytes(buffer[pointer:pointer + bytes_length])
                pointer += bytes_length
            return res_list, pointer
        elif typ_flag == STR_FLAG:
            res_list = typing.cast(List[str], [None] * length)
            for i in range(length):
                res_list[i], pointer = deserialize_str(buffer, pointer, settings)
            return res_list, pointer
        elif typ_flag == FLOAT_FLAG:
            res_list = typing.cast(List[float], [None] * length)
            for i in range(length):
                res_list[i] = struct.unpack("!d", buffer[pointer:pointer + 8])[0]
                pointer += 8
            return res_list, pointer
        else:
            raise DecodingError(f"Unexpected type flag: {typ_flag}")
    elif flag == DICT_FLAG:
        length, pointer = decode_number(buffer, pointer)
        res_dict = {}
        for i in range(length):
            k, pointer = deserialize_object(buffer, pointer, settings)
            v, pointer = deserialize_object(buffer, pointer, settings)
            res_dict[k] = v
        return res_dict, pointer
    elif flag == EMPTY_DICT_FLAG:
        return {}, pointer
    elif flag == STR_KEY_DICT_FLAG:
        length, pointer = decode_number(buffer, pointer)
        res_dict = {}
        for i in range(length):
            k, pointer = deserialize_str(buffer, pointer, settings)
            v, pointer = deserialize_object(buffer, pointer, settings)
            res_dict[k] = v
        return res_dict, pointer

    elif flag == CONSISTENT_TYPE_DICT_FLAG:
        raise Exception("not implemented yet")  # todo
    elif flag == POINTER_FLAG:
        position, pointer = decode_number(buffer, pointer)
        return settings._pointers[position], pointer
    else:
        return REVERSE_SMALL_INTS[flag], pointer


def deserialize_str(buffer: bytes, pointer: int, settings: Settings) -> Tuple[str, int]:
    start = pointer
    length, pointer = decode_number(buffer, pointer)
    encoded_str = buffer[pointer:pointer + length]
    string = encoded_str.decode(encoding=settings.encoding) if settings.encoding else encoded_str.decode()
    if settings.use_pointers:
        settings._pointers[start] = string
    return string, pointer + length
