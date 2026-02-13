import struct
from datetime import datetime
from typing import Union, List, Tuple

from pypinch.consts import NUMBER_BASE, ObjType, POSITIVE_INT_FLAG, FALSE_FLAG, TRUE_FLAG, NULL_FLAG, BYTES_FLAG, \
    LIST_FLAG, \
    DICT_FLAG, STR_KEY_DICT_FLAG, FLOAT_FLAG, STR_FLAG, NEGATIVE_INT_FLAG, EMPTY_STR_FLAG, EMPTY_BYTES_FLAG, \
    EMPTY_LIST_FLAG, EMPTY_DICT_FLAG, SMALL_INTS, CONSISTENT_TYPE_LIST_FLAG, INT_FLAG, BOOL_FLAG, POINTER_FLAG, HEADER, \
    BIG_ENDIAN_DOUBLE_FORMAT, NUMBER_OF_BITS_IN_BYTE
from pypinch.exceptions import EncodingError
from pypinch.serialize.settings import Settings
from pypinch.serialize.utils import encode_number

_pack_double = struct.Struct(BIG_ENDIAN_DOUBLE_FORMAT).pack


def dump_bytes(obj: ObjType, *, allow_non_string_keys: bool = True, modify_input: bool = False,
               use_pointers: bool = False, serialize_dates: bool = True) -> bytearray:
    settings = Settings(
        allow_non_string_keys=allow_non_string_keys,
        modify_input=modify_input,  # TODO
        use_pointers=False,
        pointers={} if use_pointers else None,
        serialize_dates=serialize_dates,
    )
    buffer = bytearray(HEADER)
    serialize_object_with_type(buffer, obj, settings)
    return buffer


def serialize_object_with_type(buffer: bytearray, obj: ObjType, settings: Settings) -> None:
    typ = type(obj)
    if typ is str:
        encode_normally = True
        if len(obj) == 0:
            buffer.append(EMPTY_STR_FLAG)
            encode_normally = False
            # todo                          python 3.9
        elif settings.use_pointers and (prev_pos := settings.pointers.get(obj)):
            temp_buffer = bytearray()
            temp_buffer.append(POINTER_FLAG)
            encode_number(temp_buffer, prev_pos)
            if len(temp_buffer) <= len(obj) + 1:
                buffer.extend(temp_buffer)
                encode_normally = False
        if encode_normally:
            buffer.append(STR_FLAG)
            if settings.use_pointers:
                settings.pointers[obj] = len(buffer)
            encoded_str = obj.encode()
            encode_number(buffer, len(encoded_str))
            buffer.extend(encoded_str)
    elif typ is int:
        if obj >= 0:
            if obj < len(SMALL_INTS):
                buffer.append(SMALL_INTS[obj])
            else:
                buffer.append(POSITIVE_INT_FLAG)
                encode_number(buffer, obj)
        else:
            buffer.append(NEGATIVE_INT_FLAG)
            encode_number(buffer, -obj)
    elif typ is bool:
        buffer.append(TRUE_FLAG if obj else FALSE_FLAG)
    elif obj is None:
        buffer.append(NULL_FLAG)
    elif typ is list or typ is tuple:
        if len(obj) == 0:
            buffer.append(EMPTY_LIST_FLAG)
        elif is_consistent_type_list(obj, settings):
            first_type = type(obj[0])
            if first_type is str and settings.use_pointers:
                serialize_normal_list(buffer, obj, settings)
            elif obj[0] is None:
                buffer.append(CONSISTENT_TYPE_LIST_FLAG)
                buffer.append(NULL_FLAG)
                encode_number(buffer, len(obj))
            elif first_type is int:
                # no longer have the flag to distinguish between positive and negative numbers so do this instead
                buffer.append(CONSISTENT_TYPE_LIST_FLAG)
                buffer.append(INT_FLAG)
                encode_number(buffer, len(obj))
                for item in obj:
                    if item <= 0:
                        buffer.append(NUMBER_BASE - 1)
                        encode_number(buffer, -item, base=NUMBER_BASE - 1)
                    else:
                        encode_number(buffer, item, base=NUMBER_BASE - 1)
            elif first_type is bool:
                buffer.append(CONSISTENT_TYPE_LIST_FLAG)
                buffer.append(BOOL_FLAG)
                encode_number(buffer, len(obj))
                byte = number_of_bits = 0
                for item in obj:
                    byte = (byte << 1) | item
                    number_of_bits += 1
                    if number_of_bits == NUMBER_OF_BITS_IN_BYTE:
                        buffer.append(byte)
                        byte = number_of_bits = 0
                if number_of_bits:
                    buffer.append(byte << (NUMBER_OF_BITS_IN_BYTE - number_of_bits))
            else:
                buffer.append(CONSISTENT_TYPE_LIST_FLAG)
                try:
                    buffer.append({str: STR_FLAG, bytes: BYTES_FLAG, float: FLOAT_FLAG, datetime: STR_FLAG}[first_type])
                except KeyError:
                    raise EncodingError(f"Unexpected type: {first_type}")

                encode_number(buffer, len(obj))
                for item in obj:
                    serialize_object_without_type(buffer, item, settings)
        else:
            serialize_normal_list(buffer, obj, settings)
    elif typ is dict:
        if len(obj) == 0:
            buffer.append(EMPTY_DICT_FLAG)
        elif not settings.use_pointers and not settings.allow_non_string_keys:
            buffer.append(STR_KEY_DICT_FLAG)
            encode_number(buffer, len(obj))
            for k, v in obj.items():
                if type(k) is not str:
                    raise EncodingError("Encountered a non string key while allow_non_string_keys is False")
                serialize_object_without_type(buffer, k, settings)
                serialize_object_with_type(buffer, v, settings)
        elif not settings.use_pointers and all(type(x) is str for x in obj.keys()):
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
    elif typ is float:
        buffer.append(FLOAT_FLAG)
        buffer.extend(_pack_double(obj))
    elif typ is bytes:
        if len(obj) == 0:
            buffer.append(EMPTY_BYTES_FLAG)
        else:
            buffer.append(BYTES_FLAG)
            encode_number(buffer, len(obj))
            buffer.extend(obj)
    elif typ is datetime and settings.serialize_dates:
        return serialize_object_with_type(buffer, obj.isoformat(), settings)
    else:
        if typ is datetime and not settings.serialize_dates:
            raise EncodingError(f"Unexpected type: datetime, with flag serialize_dates disabled")
        raise EncodingError(f"Unexpected type: {typ}")


def serialize_normal_list(buffer: bytearray, obj: Union[List, Tuple], settings: Settings) -> None:
    buffer.append(LIST_FLAG)
    encode_number(buffer, len(obj))
    for item in obj:
        serialize_object_with_type(buffer, item, settings)


def is_consistent_type_list(obj: Union[List, Tuple], settings: Settings) -> bool:
    if len(obj) <= 1:
        return False
    first_type = type(obj[0])
    if first_type in [list, dict, tuple]:
        return False
    if first_type is str and settings.use_pointers:
        return all(type(x) is str and x not in settings.pointers for x in obj)
    return all(type(x) is first_type for x in obj)


def serialize_object_without_type(buffer: bytearray, obj: ObjType, settings: Settings) -> None:
    typ = type(obj)
    if typ is int:
        encode_number(buffer, obj if obj > 0 else -obj)
    elif typ is bool:
        buffer.append(TRUE_FLAG if obj else FALSE_FLAG)
    elif obj is None:
        buffer.append(NULL_FLAG)
    elif typ is bytes:
        encode_number(buffer, len(obj))
        buffer.extend(obj)
    elif typ is list or typ is tuple:
        encode_number(buffer, len(obj))
        for item in obj:
            serialize_object_with_type(buffer, item, settings)
    elif typ is dict:
        if len(obj) == 0:
            buffer.append(EMPTY_DICT_FLAG)
        elif not settings.use_pointers and not settings.allow_non_string_keys:
            buffer.append(STR_KEY_DICT_FLAG)
            encode_number(buffer, len(obj))
            for k, v in obj.items():
                if type(k) is not str:
                    raise EncodingError("Encountered a non string key while allow_non_string_keys is False")
                serialize_object_without_type(buffer, k, settings)
                serialize_object_with_type(buffer, v, settings)
        elif not settings.use_pointers and all(type(x) is str for x in obj.keys()):
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
    elif typ is float:
        buffer.extend(_pack_double(obj))
    elif typ is str:
        encoded_str = obj.encode()
        if settings.use_pointers:
            settings.pointers[obj] = len(buffer)
        encode_number(buffer, len(encoded_str))
        buffer.extend(encoded_str)
    elif typ is datetime and settings.serialize_dates:
        return serialize_object_without_type(buffer, obj.isoformat(), settings)
    else:
        raise EncodingError(f"Unexpected type: {typ}")
