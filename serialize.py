import math
import struct
import tracemalloc
from dataclasses import dataclass
from types import NoneType
from typing import Dict

from consts import NUMBER_BASE, ObjType, ENDING_FLAG, POSITIVE_INT_FLAG, FALSE_FLAG, TRUE_FLAG, NULL_FLAG, BYTES_FLAG, \
    LIST_FLAG, \
    DICT_FLAG, STR_KEY_DICT_FLAG, FLOAT_FLAG, STR_FLAG, NEGATIVE_INT_FLAG, EMPTY_STR_FLAG, EMPTY_BYTES_FLAG, \
    EMPTY_LIST_FLAG, EMPTY_DICT_FLAG, SMALL_INTS, CONSISTENT_TYPE_LIST_FLAG, INT_FLAG, BOOL_FLAG, POINTER_FLAG
from exceptions import EncodingError


@dataclass
class Settings:
    allow_non_string_keys: bool = True
    modify_input: bool = False  # TODO
    encoding: str = None
    use_pointers: bool = False
    _pointers: Dict = None


DEFAULT_SETTINGS = Settings()


def encode_number(buffer: bytearray, num: int) -> None:
    if num < NUMBER_BASE:
        buffer.append(num)
    else:
        buffer.append(ENDING_FLAG)
        while num:
            num, remainder = divmod(num, NUMBER_BASE)
            buffer.append(remainder)
        buffer.append(ENDING_FLAG)


def dump_bytes(obj: ObjType, settings: Settings = DEFAULT_SETTINGS) -> bytearray:
    if settings.use_pointers:
        settings._pointers = {}
    buffer = bytearray(b"<o>")
    serialize_object_with_type(buffer, obj, settings)
    return buffer


def serialize_object_with_type(buffer: bytearray, obj: ObjType, settings: Settings) -> None:
    if type(obj) is int:
        if num_byte := SMALL_INTS.get(obj):
            buffer.append(num_byte)
        elif obj > 0:
            buffer.append(POSITIVE_INT_FLAG)
            encode_number(buffer, obj)
        else:
            buffer.append(NEGATIVE_INT_FLAG)
            encode_number(buffer, -obj)
    elif type(obj) is bool:
        buffer.append(TRUE_FLAG if obj else FALSE_FLAG)
    elif obj is None:
        buffer.append(NULL_FLAG)
    elif type(obj) is bytes:
        if len(obj) == 0:
            buffer.append(EMPTY_BYTES_FLAG)
        else:
            buffer.append(BYTES_FLAG)
            encode_number(buffer, len(obj))
            buffer.extend(obj)
    elif type(obj) is list:
        if len(obj) > 1:
            first_type = type(obj[0])
            if first_type is not list and first_type is not dict and all(type(x) is first_type for x in obj):
                if first_type is NoneType:
                    buffer.append(CONSISTENT_TYPE_LIST_FLAG)
                    buffer.append(NULL_FLAG)
                    encode_number(buffer, len(obj))
                elif first_type is int:
                    # no longer have type to distinguish between positive and negative numbers so do this instead
                    buffer.append(CONSISTENT_TYPE_LIST_FLAG)
                    buffer.append(INT_FLAG)
                    encode_number(buffer, len(obj))
                    for item in obj:
                        if item < 0:
                            buffer.append(ENDING_FLAG)
                            encode_number(buffer, -item)
                        else:
                            encode_number(buffer, item)
                elif first_type is bool:
                    buffer.append(CONSISTENT_TYPE_LIST_FLAG)
                    buffer.append(BOOL_FLAG)
                    encode_number(buffer, len(obj))
                    byte = n = 0
                    for item in obj:
                        byte = (byte << 1) | item
                        n += 1
                        if n == 8:
                            buffer.append(byte)
                            byte = n = 0
                    if n:
                        buffer.append(byte << (8 - n))
                else:
                    buffer.append(CONSISTENT_TYPE_LIST_FLAG)
                    buffer.append(type_to_flag(first_type))
                    encode_number(buffer, len(obj))
                    for item in obj:
                        serialize_object_without_type(buffer, item, settings)
                return
        if len(obj) == 0:
            buffer.append(EMPTY_LIST_FLAG)
        else:
            buffer.append(LIST_FLAG)
            encode_number(buffer, len(obj))
            for item in obj:
                serialize_object_with_type(buffer, item, settings)
    elif type(obj) is dict:
        # if len(obj) > 1:
        #     k, v = next(iter(obj.items()))
        #     first_key_type = type(k)
        #     first_value_type = type(v)
        #     # TODO settings.allow_non_string_keys
        #     if all(type(k) is first_key_type and type(v) is first_value_type for k, v in obj.items()):
        #         buffer.extend([CONSISTENT_TYPE_DICT_FLAG])
        #         encode_number(buffer, len(obj))
        #         # TODO
        if len(obj) == 0:
            buffer.append(EMPTY_DICT_FLAG)
        elif not settings.allow_non_string_keys:
            buffer.append(STR_KEY_DICT_FLAG)
            encode_number(buffer, len(obj))
            for k, v in obj.items():
                if k is not str:
                    raise EncodingError("Encountered a non string key while allow_non_string_keys is False")
                serialize_object_without_type(buffer, k, settings)
                serialize_object_with_type(buffer, v, settings)
        elif all(type(x) is str for x in obj.keys()):
            buffer.append(STR_KEY_DICT_FLAG)
            encode_number(buffer, len(obj))
            for k, v in obj.items():
                serialize_object_without_type(buffer, k, settings)
                serialize_object_with_type(buffer, v, settings)
        else:
            buffer.append(DICT_FLAG)
            encode_number(buffer, len(obj))
            for k, v in obj.items():
                serialize_object_with_type(buffer, k, settings)
                serialize_object_with_type(buffer, v, settings)
    elif type(obj) is float:
        buffer.append(FLOAT_FLAG)
        buffer.extend(struct.pack("!d", obj))
    elif type(obj) is str:
        if len(obj) == 0:
            buffer.append(EMPTY_STR_FLAG)
        elif settings.use_pointers and (prev_pos := settings._pointers.get(obj)):
            buffer.append(POINTER_FLAG)
            encode_number(buffer, prev_pos)
        else:
            buffer.append(STR_FLAG)
            if settings.use_pointers:
                settings._pointers[obj] = len(buffer)
            encoded_str = obj.encode(encoding=settings.encoding) if settings.encoding else obj.encode()
            encode_number(buffer, len(encoded_str))
            buffer.extend(encoded_str)
    else:
        raise EncodingError(f"Unexpected type: {type(obj)}")


def type_to_flag(typ) -> int:
    if typ is int:
        return INT_FLAG # todo positive/negative?
    elif typ is bool:
        return BOOL_FLAG # todo optimize how these are stored?
    elif typ is NoneType:
        return NULL_FLAG    # todo optimize how these are stored (only once)
    elif typ is bytes:
        return BYTES_FLAG
    # elif typ is list:
    #     return LIST_FLAG    # TODO?
    # elif typ is dict:
    #     # todo if not settings.allow_non_string_keys:
    #     return DICT_FLAG
    elif typ is float:
        return FLOAT_FLAG
    elif typ is str:
        return STR_FLAG
    else:
        raise EncodingError(f"Unexpected type: {typ}")


def serialize_object_without_type(buffer: bytearray, obj: ObjType, settings: Settings) -> None:
    if type(obj) is int:
        if obj > 0:
            encode_number(buffer, obj)
        else:
            encode_number(buffer, -obj)
    elif type(obj) is bool:
        buffer.append(TRUE_FLAG if obj else FALSE_FLAG)
    elif obj is None:
        buffer.append(NULL_FLAG)
    elif type(obj) is bytes:
        encode_number(buffer, len(obj))
        buffer.extend(obj)
    elif type(obj) is list:
        # if len(obj) > 1:
        #     first_type = type(obj[0])
        #     if all(type(x) is first_type for x in obj):
        #         buffer.extend([CONSISTENT_TYPE_LIST_FLAG])
        #         encode_number(buffer, len(obj))
        #         # TODO
        encode_number(buffer, len(obj))
        for item in obj:
            serialize_object_with_type(buffer, item, settings)
    elif type(obj) is dict:
        # if len(obj) > 1:
        #     k, v = next(iter(obj.items()))
        #     first_key_type = type(k)
        #     first_value_type = type(v)
        #     # TODO settings.allow_non_string_keys
        #     if all(type(k) is first_key_type and type(v) is first_value_type for k, v in obj.items()):
        #         buffer.extend([CONSISTENT_TYPE_DICT_FLAG])
        #         encode_number(buffer, len(obj))
        #         # TODO
        if len(obj) == 0:
            buffer.append(EMPTY_DICT_FLAG)
        elif not settings.allow_non_string_keys:
            buffer.append(STR_KEY_DICT_FLAG)
            encode_number(buffer, len(obj))
            for k, v in obj.items():
                if k is not str:
                    raise EncodingError("Encountered a non string key while allow_non_string_keys is False")
                serialize_object_without_type(buffer, k, settings)
                serialize_object_with_type(buffer, v, settings)
        elif all(type(x) is str for x in obj.keys()):
            buffer.append(STR_KEY_DICT_FLAG)
            encode_number(buffer, len(obj))
            for k, v in obj.items():
                serialize_object_without_type(buffer, k, settings)
                serialize_object_with_type(buffer, v, settings)
        else:
            buffer.append(DICT_FLAG)
            encode_number(buffer, len(obj))
            for k, v in obj.items():
                serialize_object_with_type(buffer, k, settings)
                serialize_object_with_type(buffer, v, settings)
    elif type(obj) is float:
        buffer.extend(struct.pack("!d", obj))
    elif type(obj) is str:

        if settings.use_pointers and (prev_pos := settings._pointers.get(obj)):
            buffer.append(POINTER_FLAG)
            encode_number(buffer, prev_pos)
        else:
            encoded_str = obj.encode(encoding=settings.encoding) if settings.encoding else obj.encode()
            if settings.use_pointers:
                settings._pointers[obj] = len(buffer)
            encode_number(buffer, len(encoded_str))
            buffer.extend(encoded_str)
    else:
        raise EncodingError(f"Unexpected type: {type(obj)}")
