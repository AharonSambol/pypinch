import gc
import math
import struct
import typing
from typing import Tuple, List

from pypinch.consts import NUMBER_BASE, ObjType, POSITIVE_INT_FLAG, FALSE_FLAG, TRUE_FLAG, NULL_FLAG, BYTES_FLAG, \
    LIST_FLAG, \
    DICT_FLAG, STR_KEY_DICT_FLAG, FLOAT_FLAG, STR_FLAG, NEGATIVE_INT_FLAG, EMPTY_STR_FLAG, EMPTY_BYTES_FLAG, \
    EMPTY_LIST_FLAG, EMPTY_DICT_FLAG, CONSISTENT_TYPE_LIST_FLAG, INT_FLAG, BOOL_FLAG, POINTER_FLAG, \
    ByteLike, HEADER, CONSISTENT_TYPE_DICT_FLAG, BIG_ENDIAN_DOUBLE_FORMAT, NUMBER_OF_BITS_IN_BYTE, \
    LEFTMOST_BIT_MASK, BYTES_IN_DOUBLE, NEGATIVE_NUMBER_SIGN, FIRST_FLAGS_LIST, FIRST_SMALL_INT

from pypinch.exceptions import DecodingError
from pypinch.deserialize.settings import Settings
from pypinch.deserialize.utils import decode_number


def load_bytes(
    buffer: ByteLike,
    *,
    modify_input: bool = False,
    use_tuples: bool = False,
    use_pointers: bool = True,
    stop_gc: bool = False,
) -> ObjType:

    try:
        if stop_gc:
            gc.freeze()

        settings = Settings(
            use_tuples=use_tuples,  # TODO
            use_pointers=use_pointers,
            pointers={} if use_pointers else None
        )
        if modify_input and type(buffer) is bytearray:
            original_buffer_len = len(buffer)
            del buffer[:len(HEADER)]
            return deserialize_object_from_bytearray(buffer, original_buffer_len, settings)
        else:
            return deserialize_object(buffer, len(HEADER), settings)[0]
    finally:
        if stop_gc:
            gc.unfreeze()


def deserialize_object_from_bytearray(buffer: bytearray, original_buffer_len: int, settings: Settings) -> ObjType:
    flag = buffer[0]
    del buffer[0]
    if flag < len(FIRST_FLAGS_LIST):
        return FIRST_FLAGS_LIST[flag]
    elif flag == POSITIVE_INT_FLAG:
        num, end = decode_number(buffer, 0)
        del buffer[:end]
        return num
    elif flag == STR_KEY_DICT_FLAG:
        length, pointer = decode_number(buffer, 0)
        del buffer[:pointer]
        return {
            deserialize_str_from_bytearray(buffer, original_buffer_len, settings): deserialize_object_from_bytearray(buffer, original_buffer_len, settings)
            for _ in range(length)
        }
    elif flag == STR_FLAG:
        return deserialize_str_from_bytearray(buffer, original_buffer_len, settings)

    elif flag == DICT_FLAG:
        length, pointer = decode_number(buffer, 0)
        del buffer[:pointer]
        return {
            deserialize_object_from_bytearray(buffer, original_buffer_len, settings): deserialize_object_from_bytearray(buffer, original_buffer_len, settings)
            for _ in range(length)
        }
    elif flag == EMPTY_DICT_FLAG:
        return {}
    elif flag == LIST_FLAG:
        length, pointer = decode_number(buffer, 0)
        del buffer[:pointer]
        if settings.use_tuples:
            return tuple(
                deserialize_object_from_bytearray(buffer, original_buffer_len, settings) for _ in range(length))
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
            def extract_number(_buffer: bytearray) -> int:
                if _buffer[0] == NEGATIVE_NUMBER_SIGN:
                    _num, _pointer = decode_number(_buffer, 1, base=NUMBER_BASE - 1)
                    del _buffer[:_pointer]
                    return -_num
                else:
                    _num, _pointer = decode_number(_buffer, 0, base=NUMBER_BASE - 1)
                    del _buffer[:_pointer]
                    return _num
            if settings.use_tuples:
                return tuple(extract_number(buffer) for _ in range(length))
            return [extract_number(buffer) for _ in range(length)]
        elif typ_flag == BOOL_FLAG:
            res_list = typing.cast(List[bool], [None] * length)
            length_in_bytes = math.ceil(length / NUMBER_OF_BITS_IN_BYTE)
            try:
                for i, byte in enumerate(buffer[:length_in_bytes]):
                    for j in range(NUMBER_OF_BITS_IN_BYTE):
                        res_list[i * NUMBER_OF_BITS_IN_BYTE + j] = (byte & LEFTMOST_BIT_MASK) == LEFTMOST_BIT_MASK
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
                res_list[i] = struct.unpack(BIG_ENDIAN_DOUBLE_FORMAT, buffer[:BYTES_IN_DOUBLE])[0]
                del buffer[:BYTES_IN_DOUBLE]
            return res_list
        else:
            raise DecodingError(f"Unexpected type flag: {typ_flag}")
    elif flag == NEGATIVE_INT_FLAG:
        num, end = decode_number(buffer, 0)
        del buffer[:end]
        return -num
    elif flag == FLOAT_FLAG:
        num = struct.unpack(BIG_ENDIAN_DOUBLE_FORMAT, buffer[:BYTES_IN_DOUBLE])[0]
        del buffer[:BYTES_IN_DOUBLE]
        return num
    elif flag == BYTES_FLAG:
        length, pointer = decode_number(buffer, 0)
        byts = buffer[pointer:pointer + length]
        del buffer[:pointer + length]
        return bytes(byts)
    elif flag == CONSISTENT_TYPE_DICT_FLAG:
        raise Exception("not implemented yet") 	# todo
    elif flag == POINTER_FLAG:
        position, pointer = decode_number(buffer, 0)
        del buffer[:pointer]
        return settings.pointers[position]
    elif flag == BOOL_FLAG:
        raise DecodingError("unexpected flag: BOOL")
    elif flag == INT_FLAG:
        raise DecodingError("unexpected flag: INT")
    else:
        return flag - FIRST_SMALL_INT


def deserialize_str_from_bytearray(buffer: bytearray, original_buffer_len: int, settings: Settings) -> str:
    position = original_buffer_len - len(buffer)
    length, pointer = decode_number(buffer, 0)
    encoded_str = buffer[pointer:pointer + length]
    del buffer[:pointer + length]
    string = encoded_str.decode()
    if settings.use_pointers:
        settings.pointers[position] = string
    return string


def deserialize_object(buffer: bytes, pointer: int, settings: Settings) -> (ObjType, int):
    flag = buffer[pointer]
    pointer += 1
    if flag < len(FIRST_FLAGS_LIST):
        return FIRST_FLAGS_LIST[flag], pointer
    elif flag == POSITIVE_INT_FLAG:
        return decode_number(buffer, pointer)
    elif flag == STR_KEY_DICT_FLAG:
        length, pointer = decode_number(buffer, pointer)
        res_dict = {}
        for i in range(length):
            k, pointer = deserialize_str(buffer, pointer, settings)
            v, pointer = deserialize_object(buffer, pointer, settings)
            res_dict[k] = v
        return res_dict, pointer
    elif flag == STR_FLAG:
        return deserialize_str(buffer, pointer, settings)
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
            # same as: math.ceil(length / NUMBER_OF_BITS_IN_BYTE)
            # the `>> 3` is like dividing by 8 (8 is `1000` in binary)
            # the + 7 is like rounding up
            length_in_bytes = (length + 7) >> 3
            try:
                for i in range(length_in_bytes):
                    byte = buffer[pointer + i]
                    for j in range(NUMBER_OF_BITS_IN_BYTE):
                        res_list[i * NUMBER_OF_BITS_IN_BYTE + j] = (byte & LEFTMOST_BIT_MASK) == LEFTMOST_BIT_MASK
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
                res_list[i] = struct.unpack(BIG_ENDIAN_DOUBLE_FORMAT, buffer[pointer:pointer + BYTES_IN_DOUBLE])[0]
                pointer += BYTES_IN_DOUBLE
            return res_list, pointer
        else:
            raise DecodingError(f"Unexpected type flag: {typ_flag}")
    elif flag == NEGATIVE_INT_FLAG:
        num, pointer = decode_number(buffer, pointer)
        return -num, pointer
    elif flag == FLOAT_FLAG:
        num = struct.unpack(BIG_ENDIAN_DOUBLE_FORMAT, buffer[pointer:pointer + BYTES_IN_DOUBLE])[0]
        return num, pointer + BYTES_IN_DOUBLE
    elif flag == BYTES_FLAG:
        length, pointer = decode_number(buffer, pointer)
        return bytes(buffer[pointer:pointer + length]), pointer + length
    elif flag == POINTER_FLAG:
        position, pointer = decode_number(buffer, pointer)
        return settings.pointers[position], pointer
    elif flag == INT_FLAG:
        raise DecodingError("unexpected flag")
    elif flag == BOOL_FLAG:
        raise DecodingError("unexpected flag")
    elif flag == CONSISTENT_TYPE_DICT_FLAG:
        raise Exception("not implemented yet")  # todo
    else:
        return flag - FIRST_SMALL_INT, pointer


def deserialize_str(buffer: bytes, pointer: int, settings: Settings) -> Tuple[str, int]:
    start = pointer
    length, pointer = decode_number(buffer, pointer)
    string = buffer[pointer:pointer + length].decode()
    if settings.use_pointers:
        settings.pointers[start] = string
    return string, pointer + length
