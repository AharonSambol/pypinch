import struct
import typing
from typing import List, Union, Any, Dict, Callable

from pypinch.consts import NUMBER_BASE, ObjType, POSITIVE_INT_FLAG, NULL_FLAG, BYTES_FLAG, \
    LIST_FLAG, \
    DICT_FLAG, STR_KEY_DICT_FLAG, FLOAT_FLAG, STR_FLAG, NEGATIVE_INT_FLAG, \
    EMPTY_LIST_FLAG, EMPTY_DICT_FLAG, CONSISTENT_TYPE_LIST_FLAG, BOOL_FLAG, POINTER_FLAG, \
    ByteLike, HEADER, BIG_ENDIAN_DOUBLE_FORMAT, NUMBER_OF_BITS_IN_BYTE, \
    LEFTMOST_BIT_MASK, BYTES_IN_DOUBLE, FIRST_FLAGS_LIST, AMOUNT_OF_USED_FLAGS, \
    ASCII_STR_FLAG, LIST_OF_STRUCTURED_DICTS_FLAG, EMPTY_STR_FLAG, \
    EMPTY_BYTES_FLAG, TRUE_FLAG, FALSE_FLAG, INVALID_UTF_8_START_BYTE_COMPACT_ASCII, POINTER_FLAG_1BYTE, \
    POINTER_FLAG_4BYTE, POINTER_FLAG_3BYTE, POINTER_FLAG_2BYTE, CUSTOM_TYPE_FLAG
from pypinch.deserialize.deserialize import deserialize_object, deserialize_str
from pypinch.deserialize.settings import Settings
from pypinch.deserialize.utils import decode_number, skip_number
from pypinch.exceptions import DeserializationError

INDEX_OUT_OF_RANGE_TEMPLATE = "Index out of range, index is `{}` but list is of len `{}`"
KEY_NOT_IN_DICT_TEMPLATE = "Key not found, key: `{}` (type `{}`)"


class Idx:
    def __init__(self, index: int):
        self.index = index


class PointersHolder:
    def __init__(self, buffer: bytes):
        self.buffer = buffer
        self.str_posses = []

    def __getitem__(self, item):
        if type(self.str_posses[item]) is str:
            return self.str_posses[item]

        pointer = self.str_posses[item]
        length, pointer = decode_number(self.buffer, pointer >> 1, base=NUMBER_BASE - (pointer & 1))
        return self.buffer[
            #          Skip 1 char if buffer starts with INVALID_UTF_8_START_BYTE_COMPACT_ASCII
            pointer + (self.buffer[pointer] == INVALID_UTF_8_START_BYTE_COMPACT_ASCII)
            :pointer + length
        ].decode()

    def append(self, string: str) -> None:
        self.str_posses.append(string)


def lazy_load_bytes(
        buffer: ByteLike,
        path_to_load: List[Union[Any, Idx]],
        *,
        custom_types: Dict[Any, Callable[[bytes], Any]] = None,
        # use_tuples: bool = False,
        # stop_gc: bool = False,
) -> ObjType:
    try:
        return lazy_deserialize_object(
            buffer,
            len(HEADER),
            path_to_load,
            Settings(use_tuples=False, pointers=PointersHolder(buffer), custom_types=custom_types),
            dont_load=False
        )
    except DeserializationError:
        raise
    except MemoryError:
        raise
    except Exception as e:
        raise DeserializationError() from e


def bytes_check_if_contains(
        buffer: ByteLike,
        path_to_load: List[Union[Any, Idx]],
        *,
        custom_types: Dict[Any, Callable[[bytes], Any]] = None,
        # stop_gc: bool = False,
) -> ObjType:
    try:
        return lazy_deserialize_object(
            buffer,
            len(HEADER),
            path_to_load,
            Settings(use_tuples=False, pointers=PointersHolder(buffer), custom_types=custom_types),
            dont_load=True
        )
    except DeserializationError:
        return False
    except MemoryError:
        raise
    except Exception as e:
        raise DeserializationError() from e


def lazy_deserialize_object(buffer: bytes, pointer: int, path_to_load: List[Any], settings: Settings,
                            dont_load: bool) -> ObjType:
    if not path_to_load:
        if dont_load:
            return True
        return deserialize_object(buffer, pointer, settings)[0]
    indexer, path_to_load = path_to_load[0], path_to_load[1:]
    flag = buffer[pointer]
    pointer += 1

    if isinstance(indexer, Idx):
        index = indexer.index
        if flag == EMPTY_LIST_FLAG:
            raise DeserializationError(INDEX_OUT_OF_RANGE_TEMPLATE.format(index, 0))
        elif flag == LIST_FLAG:
            length, pointer = decode_number(buffer, pointer)
            if index not in range(length):
                raise DeserializationError(INDEX_OUT_OF_RANGE_TEMPLATE.format(index, length))
            for _ in range(index):
                pointer = skip_object(buffer, pointer, settings)
            return lazy_deserialize_object(buffer, pointer, path_to_load, settings, dont_load=dont_load)
        elif flag == CONSISTENT_TYPE_LIST_FLAG:
            return lazy_deserialize_consistent_type_list(buffer, index, path_to_load, pointer, settings)
        elif flag == LIST_OF_STRUCTURED_DICTS_FLAG:
            list_length, pointer = decode_number(buffer, pointer)
            dict_length, pointer = decode_number(buffer, pointer)
            if index not in range(list_length):
                raise DeserializationError(INDEX_OUT_OF_RANGE_TEMPLATE.format(index, list_length))

            if path_to_load:    # in this case we don't need to load the whole dict, only the specific value
                next_indexer, path_to_load = path_to_load[0], path_to_load[1:]
                if isinstance(next_indexer, Idx):
                    raise DeserializationError(f"Invalid path, expected `list` but found `dict`")

                key_index = None
                checking_keys = True
                for i in range(dict_length):
                    if not checking_keys:
                        pointer = skip_object(buffer, pointer, settings)    # skip key
                        pointer = skip_object(buffer, pointer, settings)    # skip value
                        continue
                    key, pointer = deserialize_object(buffer, pointer, settings)
                    if key == next_indexer:
                        if index == 0:
                            return lazy_deserialize_object(buffer, pointer, path_to_load, settings, dont_load=dont_load)
                        key_index = i
                        checking_keys = False   # no more need to deserialize the keys for comparing them
                    pointer = skip_object(buffer, pointer, settings)

                if key_index is None:
                    raise DeserializationError(KEY_NOT_IN_DICT_TEMPLATE.format(next_indexer, type(next_indexer)))

                for _ in range((index - 1) * dict_length + key_index):
                    pointer = skip_object(buffer, pointer, settings)

                return lazy_deserialize_object(buffer, pointer, path_to_load, settings, dont_load=dont_load)
            else:
                if dont_load:
                    return True
                if index == 0:
                    dct = {}
                    for i in range(dict_length):
                        k, pointer = deserialize_object(buffer, pointer, settings)
                        v, pointer = deserialize_object(buffer, pointer, settings)
                        dct[k] = v
                    return dct
                else:
                    keys = typing.cast(List[str], [None] * dict_length)
                    for i in range(dict_length):
                        k, pointer = deserialize_object(buffer, pointer, settings)
                        pointer = skip_object(buffer, pointer, settings)
                        keys[i] = k

                    for _ in range((index - 1) * dict_length):
                        pointer = skip_object(buffer, pointer, settings)

                    dct = {}
                    for i, key in enumerate(keys):
                        dct[key], pointer = deserialize_object(buffer, pointer, settings)
                    return dct
        else:
            raise DeserializationError(f"Invalid path, expected `list` but found `{flag_to_type_name(flag)}`")
    else:
        if flag == EMPTY_DICT_FLAG:
            raise DeserializationError(KEY_NOT_IN_DICT_TEMPLATE.format(indexer, type(indexer)))
        elif flag == DICT_FLAG:
            length, pointer = decode_number(buffer, pointer)
            for _ in range(length):
                # TODO: check char char if it matches indexer and the moment it doesnt skip all the rest of the chars
                key, pointer = deserialize_object(buffer, pointer, settings)
                if key == indexer:
                    return lazy_deserialize_object(buffer, pointer, path_to_load, settings, dont_load=dont_load)
                pointer = skip_object(buffer, pointer, settings)
            raise DeserializationError(KEY_NOT_IN_DICT_TEMPLATE.format(indexer, type(indexer)))
        elif flag == STR_KEY_DICT_FLAG:
            length, pointer = decode_number(buffer, pointer)
            for _ in range(length):
                if buffer[pointer] == NUMBER_BASE - 1:
                    position, pointer = decode_number(buffer, pointer + 1)
                    key = settings.pointers[position]
                else:
                    key, pointer = deserialize_str(buffer, pointer, settings, base=NUMBER_BASE - 1)
                if key == indexer:
                    return lazy_deserialize_object(buffer, pointer, path_to_load, settings, dont_load=dont_load)
                pointer = skip_object(buffer, pointer, settings)
            raise DeserializationError(KEY_NOT_IN_DICT_TEMPLATE.format(indexer, type(indexer)))
        else:
            raise DeserializationError(f"Invalid path, expected `dict` but found `{flag_to_type_name(flag)}`")


def skip_object(buffer: bytes, pointer: int, settings: Settings) -> int:
    flag = buffer[pointer]
    pointer += 1

    if flag < len(FIRST_FLAGS_LIST) or flag in [EMPTY_DICT_FLAG, EMPTY_LIST_FLAG]:
        return pointer
    elif flag == [NEGATIVE_INT_FLAG, POINTER_FLAG, POSITIVE_INT_FLAG]:
        return skip_number(buffer, pointer)
    elif flag == STR_KEY_DICT_FLAG:
        length, pointer = decode_number(buffer, pointer)
        for _ in range(length):
            if buffer[pointer] == NUMBER_BASE - 1:
                pointer = skip_number(buffer, pointer + 1)
            else:
                pointer = skip_string(buffer, pointer, settings, base=NUMBER_BASE - 1)
            pointer = skip_object(buffer, pointer, settings)
        return pointer
    elif flag == ASCII_STR_FLAG or flag == STR_FLAG:
        return skip_string(buffer, pointer, settings)
    elif flag == DICT_FLAG:
        length, pointer = decode_number(buffer, pointer)
        for _ in range(length):
            if buffer[pointer] == STR_FLAG:
                # fast path
                pointer = skip_string(buffer, pointer + 1, settings)
            else:
                pointer = skip_object(buffer, pointer, settings)
            pointer = skip_object(buffer, pointer, settings)
        return pointer
    elif flag == LIST_FLAG:
        length, pointer = decode_number(buffer, pointer)
        for _ in range(length):
            pointer = skip_object(buffer, pointer, settings)
        return pointer
    elif flag == CONSISTENT_TYPE_LIST_FLAG:
        typ_flag = buffer[pointer]
        length, pointer = decode_number(buffer, pointer + 1)
        if typ_flag == NULL_FLAG:
            return pointer
        elif typ_flag == BOOL_FLAG:
            length_in_bytes = (length + 7) >> 3
            return pointer + length_in_bytes
        elif typ_flag == BYTES_FLAG:
            for _ in range(length):
                bytes_length, pointer = decode_number(buffer, pointer)
                pointer += bytes_length
            return pointer
        elif typ_flag == STR_FLAG:
            for _ in range(length):
                pointer = skip_string(buffer, pointer, settings)
            return pointer
        elif typ_flag == FLOAT_FLAG:
            return pointer + BYTES_IN_DOUBLE * length
        else:
            raise DeserializationError(f"Unexpected type flag: {typ_flag}")
    elif flag == FLOAT_FLAG:
        return pointer + BYTES_IN_DOUBLE
    elif flag == BYTES_FLAG:
        length, pointer = decode_number(buffer, pointer)
        return pointer + length
    elif flag == POINTER_FLAG_1BYTE:
        return pointer + 1
    elif flag == POINTER_FLAG_2BYTE:
        return pointer + 2
    elif flag == POINTER_FLAG_3BYTE:
        return pointer + 3
    elif flag == POINTER_FLAG_4BYTE:
        return pointer + 4
    elif flag == LIST_OF_STRUCTURED_DICTS_FLAG:
        list_length, pointer = decode_number(buffer, pointer)
        dict_length, pointer = decode_number(buffer, pointer)
        # first dict:
        for _ in range(dict_length):
            pointer = skip_object(buffer, pointer, settings)
            pointer = skip_object(buffer, pointer, settings)
        # rest of the dicts:
        for list_idx in range(1, list_length):
            for _ in range(dict_length):
                pointer = skip_object(buffer, pointer, settings)
        return pointer
    elif flag == CUSTOM_TYPE_FLAG:
        pointer = skip_object(buffer, pointer, settings)
        pointer = skip_object(buffer, pointer, settings)
        return pointer
    elif flag < AMOUNT_OF_USED_FLAGS:
        raise DeserializationError("unexpected flag")
    else:
        return pointer


def lazy_deserialize_consistent_type_list(buffer: bytes, index: int, path_to_load: List[Any], pointer: int,
                                          settings: Settings) -> Any:
    typ_flag = buffer[pointer]
    length, pointer = decode_number(buffer, pointer + 1)
    if index not in range(length):
        raise DeserializationError(INDEX_OUT_OF_RANGE_TEMPLATE.format(index, length))

    if path_to_load:
        got_type = flag_to_type_name(typ_flag)
        raise DeserializationError(
            f"Invalid path, expected `{'list' if isinstance(path_to_load[0], Idx) else 'dict'}` but found `{got_type}`"
        )

    if typ_flag == NULL_FLAG:
        return None
    elif typ_flag == BOOL_FLAG:
        return lazy_load_bool_list(buffer, index, pointer, length)
    elif typ_flag == BYTES_FLAG:
        return lazy_load_bytes_list(buffer, index, pointer)
    elif typ_flag == STR_FLAG:
        return lazy_load_str_list(buffer, index, pointer, settings)
    elif typ_flag == FLOAT_FLAG:
        return lazy_load_float_list(buffer, index, pointer)
    else:
        raise DeserializationError(f"Unexpected type flag: {typ_flag}")


def flag_to_type_name(flag: int) -> str:
    if flag >= AMOUNT_OF_USED_FLAGS:
        return "int"
    if res := {
        NULL_FLAG: "None",
        BOOL_FLAG: "bool",
        BYTES_FLAG: "bytes",
        STR_FLAG: "str",
        FLOAT_FLAG: "float",
        EMPTY_STR_FLAG: "str",
        EMPTY_BYTES_FLAG: "bytes",
        TRUE_FLAG: "bool",
        FALSE_FLAG: "bool",
        EMPTY_LIST_FLAG: "list",
        EMPTY_DICT_FLAG: "dict",
        POSITIVE_INT_FLAG: "int",
        NEGATIVE_INT_FLAG: "int",
        LIST_FLAG: "list",
        CONSISTENT_TYPE_LIST_FLAG: "list",
        DICT_FLAG: "dict",
        STR_KEY_DICT_FLAG: "dict",
        POINTER_FLAG: "str",
        ASCII_STR_FLAG: "str",
        LIST_OF_STRUCTURED_DICTS_FLAG: "list"
    }.get(flag):
        return res
    raise DeserializationError("Corrupt data: unexpected flag")


def lazy_load_int_list(buffer: bytes, index: int, pointer: int) -> int:
    for _ in range(index):
        if buffer[pointer] == NUMBER_BASE - 1:
            pointer = skip_number(buffer, pointer + 1)
        else:
            pointer = skip_number(buffer, pointer)
    if buffer[pointer] == NUMBER_BASE - 1:
        num, pointer = decode_number(buffer, pointer + 1, base=NUMBER_BASE - 1)
        return -num
    else:
        num, pointer = decode_number(buffer, pointer, base=NUMBER_BASE - 1)
        return num


def lazy_load_bool_list(buffer: bytes, index: int, pointer: int, length: int) -> bool:
    # same as: math.ceil(length / NUMBER_OF_BITS_IN_BYTE)
    # the `>> 3` is like dividing by 8 (8 is `1000` in binary)
    # the + 7 is like rounding up
    length_in_bytes = (length + 7) >> 3
    for i in range(length_in_bytes):
        byte = buffer[pointer + i]
        for j in range(NUMBER_OF_BITS_IN_BYTE):
            if i * NUMBER_OF_BITS_IN_BYTE + j == index:
                return (byte & LEFTMOST_BIT_MASK) == LEFTMOST_BIT_MASK
            byte <<= 1
    # TODO: unless data is malformed? (also in rust err)
    raise DeserializationError("this should be unreachable")


def lazy_load_bytes_list(buffer: bytes, index: int, pointer: int) -> bytes:
    for _ in range(index):
        bytes_length, pointer = decode_number(buffer, pointer)
        pointer += bytes_length

    bytes_length, pointer = decode_number(buffer, pointer)
    return buffer[pointer:pointer + bytes_length]


def lazy_load_str_list(buffer: bytes, index: int, pointer: int, settings: Settings) -> str:
    for _ in range(index):
        pointer = skip_string(buffer, pointer, settings)
    res, _ = deserialize_str(buffer, pointer, settings)
    return res


def lazy_load_float_list(buffer: bytes, index: int, pointer: int) -> float:
    pointer += BYTES_IN_DOUBLE * index
    return struct.unpack(BIG_ENDIAN_DOUBLE_FORMAT, buffer[pointer:pointer + BYTES_IN_DOUBLE])[0]


def skip_string(buffer: bytes, pointer: int, settings: Settings, base: int = NUMBER_BASE) -> int:
    settings.pointers.str_posses.append((pointer << 1) + (base == NUMBER_BASE))
    length, pointer = decode_number(buffer, pointer, base=base)
    return pointer + length
