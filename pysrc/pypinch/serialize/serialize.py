import struct
from datetime import datetime
from typing import Union, List, Tuple, Dict, Type

from pypinch.consts import NUMBER_BASE, ObjType, POSITIVE_INT_FLAG, FALSE_FLAG, TRUE_FLAG, NULL_FLAG, BYTES_FLAG, \
    LIST_FLAG, \
    DICT_FLAG, STR_KEY_DICT_FLAG, FLOAT_FLAG, STR_FLAG, NEGATIVE_INT_FLAG, EMPTY_STR_FLAG, EMPTY_BYTES_FLAG, \
    EMPTY_LIST_FLAG, EMPTY_DICT_FLAG, AMOUNT_OF_USED_FLAGS, CONSISTENT_TYPE_LIST_FLAG, BOOL_FLAG, \
    POINTER_FLAG, HEADER, \
    BIG_ENDIAN_DOUBLE_FORMAT, NUMBER_OF_BITS_IN_BYTE, \
    ASCII_STR_FLAG, INVALID_UTF_8_START_BYTE_COMPACT_ASCII, LIST_OF_STRUCTURED_DICTS_FLAG, POINTER_FLAG_1BYTE, \
    POINTER_FLAG_2BYTE, POINTER_FLAG_3BYTE, POINTER_FLAG_4BYTE, CUSTOM_TYPE_FLAG
from pypinch.exceptions import SerializationError
from pypinch.serialize.settings import Settings, CustomType
from pypinch.serialize.utils import encode_number

_pack_double = struct.Struct(BIG_ENDIAN_DOUBLE_FORMAT).pack


def dump_bytes(
        obj: ObjType,
        *,
        allow_non_string_keys: bool = True,
        serialize_dates: bool = False,
        custom_types: Dict[Type, CustomType] = None
) -> bytes:
    try:
        settings = Settings(
            allow_non_string_keys=allow_non_string_keys,
            pointers={},
            serialize_dates=serialize_dates,
            str_count=0,
            custom_types=custom_types,
        )
        buffer = bytearray(HEADER)
        serialize_object(buffer, obj, settings)
        return bytes(buffer)
    except SerializationError:
        raise
    except MemoryError:
        raise
    except Exception as e:
        raise SerializationError() from e


def serialize_object(buffer: bytearray, obj: ObjType, settings: Settings) -> None:
    typ = type(obj)
    if typ is str:
        if len(obj) == 0:
            buffer.append(EMPTY_STR_FLAG)
            return
        if (prev_pos := settings.pointers.get(obj)) is not None:
            if prev_pos < 2 ** 8:
                buffer.append(POINTER_FLAG_1BYTE)
                buffer.append(prev_pos)
            elif prev_pos < 2 ** 16:
                buffer.append(POINTER_FLAG_2BYTE)
                buffer.append(prev_pos >> 8)
                buffer.append(prev_pos & 0b11111111)
            elif prev_pos < 2 ** 24:
                buffer.append(POINTER_FLAG_3BYTE)
                buffer.append(prev_pos >> 16)
                buffer.append(prev_pos >> 8 & 0b11111111)
                buffer.append(prev_pos & 0b11111111)
            elif prev_pos < 2 ** 32:
                buffer.append(POINTER_FLAG_4BYTE)
                buffer.append(prev_pos >> 24)
                buffer.append(prev_pos >> 16 & 0b11111111)
                buffer.append(prev_pos >> 8 & 0b11111111)
                buffer.append(prev_pos & 0b11111111)
            else:
                buffer.append(POINTER_FLAG)
                encode_number(buffer, prev_pos)
            return
        else:
            settings.pointers[obj] = settings.str_count
        settings.str_count += 1
        try:
            encoded_str = obj.encode(encoding="ascii")
            buffer.append(ASCII_STR_FLAG)
        except UnicodeEncodeError:
            buffer.append(STR_FLAG)
            encoded_str = obj.encode()
        encode_number(buffer, len(encoded_str))
        buffer.extend(encoded_str)
    elif typ is int:
        if obj >= 0:
            if obj < NUMBER_BASE - AMOUNT_OF_USED_FLAGS:
                buffer.append(AMOUNT_OF_USED_FLAGS + obj)
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
        elif is_consistent_type_list(obj):
            first_type = type(obj[0])
            if obj[0] is None:
                buffer.append(CONSISTENT_TYPE_LIST_FLAG)
                buffer.append(NULL_FLAG)
                encode_number(buffer, len(obj))
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
            elif first_type is dict:
                first_keys = obj[0].keys()
                if (
                        all(type(k) is str for k in first_keys)
                        and all(len(x) == len(first_keys) for x in obj[1:])
                        and all(x.keys() == first_keys for x in obj[1:])
                ):
                    buffer.append(LIST_OF_STRUCTURED_DICTS_FLAG)
                    encode_number(buffer, len(obj))
                    encode_number(buffer, len(first_keys))

                    # first dict:
                    for k, v in obj[0].items():
                        serialize_object(buffer, k, settings)
                        serialize_object(buffer, v, settings)

                    # the rest:
                    for item in obj[1:]:
                        for key in first_keys:
                            serialize_object(buffer, item[key], settings)
                else:
                    serialize_normal_list(buffer, obj, settings)
            elif first_type is float:
                buffer.append(CONSISTENT_TYPE_LIST_FLAG)
                buffer.append(FLOAT_FLAG)
                encode_number(buffer, len(obj))
                for item in obj:
                    buffer.extend(_pack_double(item))
            elif first_type is bytes:
                buffer.append(CONSISTENT_TYPE_LIST_FLAG)
                buffer.append(BYTES_FLAG)
                encode_number(buffer, len(obj))
                for item in obj:
                    encode_number(buffer, len(item))
                    buffer.extend(item)
            else:
                raise SerializationError(f"Unexpected type: {first_type}")
        else:
            serialize_normal_list(buffer, obj, settings)
    elif typ is dict:
        if len(obj) == 0:
            buffer.append(EMPTY_DICT_FLAG)
        # TODO: on lists as well and in serialize_without_type
        elif all(type(x) is str for x in obj.keys()):
            buffer.append(STR_KEY_DICT_FLAG)
            encode_number(buffer, len(obj))
            for k, v in obj.items():
                if (prev_pos := settings.pointers.get(k)) is not None:
                    # TODO: pointer optimization here too?
                    buffer.append(NUMBER_BASE - 1)
                    encode_number(buffer, prev_pos)
                else:
                    serialize_str_without_type(buffer, k, settings, base=NUMBER_BASE - 1)
                serialize_object(buffer, v, settings)
        else:
            buffer.append(DICT_FLAG)
            encode_number(buffer, len(obj))
            for k, v in obj.items():
                if type(k) is tuple:
                    raise SerializationError("Invalid type for dict key: tuple")
                serialize_object(buffer, k, settings)
                serialize_object(buffer, v, settings)
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
        return serialize_object(buffer, obj.isoformat(), settings)
    elif (custom_type := settings.custom_types.get(typ)) is not None:
        custom_type: CustomType
        buffer.append(CUSTOM_TYPE_FLAG)
        serialize_object(buffer, custom_type.identifier, settings)
        serialize_object(buffer, custom_type.converter(obj), settings)
    else:
        if typ is datetime and not settings.serialize_dates:
            raise SerializationError(f"Unexpected type: datetime, with flag serialize_dates disabled")
        raise SerializationError(f"Unexpected type: {typ}")


def serialize_normal_list(buffer: bytearray, obj: Union[List, Tuple], settings: Settings) -> None:
    buffer.append(LIST_FLAG)
    encode_number(buffer, len(obj))
    for item in obj:
        serialize_object(buffer, item, settings)


def is_consistent_type_list(obj: Union[List, Tuple]) -> bool:
    if len(obj) <= 1:
        return False
    first_type = type(obj[0])
    if first_type in [type(None), bool, dict, bytes, float]:
        return all(type(x) is first_type for x in obj[1:])
    return False


def serialize_str_without_type(buffer: bytearray, obj: ObjType, settings: Settings, base: int = NUMBER_BASE) -> None:
    try:
        encoded_str = obj.encode(encoding="ascii")
        encode_number(buffer, 1 + len(encoded_str), base=base)
        buffer.append(INVALID_UTF_8_START_BYTE_COMPACT_ASCII)
    except UnicodeEncodeError:
        encoded_str = obj.encode()
        encode_number(buffer, len(encoded_str), base=base)
    settings.pointers[obj] = settings.str_count
    settings.str_count += 1
    buffer.extend(encoded_str)
