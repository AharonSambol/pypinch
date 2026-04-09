// use crate::deserializing::deserialize::deserialize_object;
// use crate::deserializing::utils::{decode_large_number, decode_number_usize, skip_number, DESERIALIZATION_ERROR_TYPE};
// use crate::{raise_mem_error_if_null, safe_get, safe_new_py_dict, safe_new_py_list};
// use crate::utils::consts::{CONSISTENT_TYPE_LIST_FLAG, EMPTY_DICT_FLAG, EMPTY_LIST_FLAG, LIST_FLAG, LIST_OF_STRUCTURED_DICTS_FLAG, DICT_FLAG, STR_KEY_DICT_FLAG, POSITIVE_INT_FLAG, NEGATIVE_INT_FLAG, FLOAT_FLAG, STR_FLAG, ASCII_STR_FLAG, TRUE_FLAG, FALSE_FLAG, NULL_FLAG, POINTER_FLAG, POINTER_FLAG_1BYTE, POINTER_FLAG_2BYTE, POINTER_FLAG_3BYTE, POINTER_FLAG_4BYTE, BYTES_FLAG, EMPTY_BYTES_FLAG, EMPTY_STR_FLAG, CUSTOM_TYPE_FLAG, AMOUNT_OF_USED_FLAGS};
// use crate::utils::consts::{NUMBER_BASE};
// use crate::utils::py_dict_key::{PyHashMap, PyKey};
// use crate::utils::py_helpers::{pretty_type, py_str_to_rust_str, ToPyErr};
// use pyo3_ffi::{PyObject, PyObject_CallObject, PyObject_Str, PyTuple_New, Py_DECREF, Py_INCREF};
// use crate::deserializing::compound_types::{decode_dict, decode_list, decode_list_of_structured_dicts, decode_str_key_dict, deserialize_dict_key};
// use crate::deserializing::consistent_typed_list::decode_consistent_type_list;
// use crate::deserializing::pointer_holders::pointer_holder::PointerHolder;
// use crate::deserializing::primitives::{decode_bytes, decode_f64, decode_false, decode_negative_int, decode_null, decode_pointer, decode_sized_pointer, decode_string, decode_true};
// use crate::serializing::settings::Settings;
// use crate::serializing::utils::{EMPTY_BYTES, EMPTY_STRING, EMPTY_TUPLE, SERIALIZATION_ERROR_TYPE};
// use crate::utils::wrappers::tuple_set_item;
// 
// enum PathPart {
//     Index(usize),
//     Key(*mut PyObject),
// }
// 
// macro_rules! index_out_of_range_template {
//     () => {
//         "Index out of range, index is `{}` but list is of len `{}`"
//     };
// }
// macro_rules! key_not_in_dict_template {
//     () => {
//         "Key not found, key: `{}` (type `{}`)"
//     };
// }
// 
// pub fn lazy_deserialize(
//     buf: &[u8],
//     ptr: &mut usize,
//     pointers: &mut PointerHolder,
//     use_tuples: bool,
//     str_count: &mut usize,
//     custom_types: &Option<PyHashMap<*mut PyObject>>,
//     dont_load: bool,
//     mut path_to_load: &[PathPart],
// ) -> Result<*mut PyObject, *mut PyObject> {
//     if path_to_load.is_empty() {
//         if dont_load {
//             return Ok(std::ptr::null_mut());
//         }
//         return deserialize_object(buf, ptr, pointers, use_tuples, str_count, custom_types);
//     }
//     let indexer = &path_to_load[0];
//     path_to_load = &path_to_load[1..];
//     let flag = *safe_get!(buf, *ptr);
//     *ptr += 1;
//     match indexer {
//         PathPart::Index(index) => match flag {
//             EMPTY_LIST_FLAG => Err(format!(index_out_of_range_template!(), index, 0)
//                 .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE })),
//             LIST_FLAG => {
//                 let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
//                 if *index >= len {
//                     return Err(format!(index_out_of_range_template!(), index, len)
//                         .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }));
//                 }
//                 for _ in 0..len {
//                     skip_object(buf, ptr, pointers, str_count)?;
//                 }
//                 lazy_deserialize(
//                     buf,
//                     ptr,
//                     pointers,
//                     use_tuples,
//                     str_count,
//                     custom_types,
//                     dont_load,
//                     path_to_load,
//                 )
//             }
//             CONSISTENT_TYPE_LIST_FLAG => lazy_deserialize_consistent_type_list(
//                 buf,
//                 ptr,
//                 index,
//                 path_to_load,
//                 pointers,
//                 str_count,
//             ),
//             LIST_OF_STRUCTURED_DICTS_FLAG => {
//                 todo!()
//             }
//             _ => Err(format!(
//                 "Invalid path, expected `list` but found `{}`",
//                 flag_to_type_name(flag)
//             )
//             .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE })),
//         },
//         PathPart::Key(key) => {
//             match flag {
//                 EMPTY_DICT_FLAG => {
//                     Err(format!(
//                         key_not_in_dict_template!(),
//                         py_str_to_rust_str(&unsafe { PyObject_Str(*key) })?,
//                         pretty_type(*key)
//                     ).to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }))
//                 },
//                 DICT_FLAG => {
//                     let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
//                     for _ in 0..len {
//                         let dict_key = deserialize_object(buf, ptr, pointers, use_tuples, str_count, custom_types)?;
//                         if dict_key == *key {
//                             return lazy_deserialize(
//                                 buf,
//                                 ptr,
//                                 pointers,
//                                 use_tuples,
//                                 str_count,
//                                 custom_types,
//                                 dont_load,
//                                 path_to_load,
//                             )
//                         }
//                         skip_object(buf, ptr, pointers, str_count)?;
//                     }
//                     Err(format!(
//                         key_not_in_dict_template!(),
//                         py_str_to_rust_str(&unsafe { PyObject_Str(*key) })?,
//                         pretty_type(*key)
//                     ).to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }))
//                 }
//                 STR_KEY_DICT_FLAG => {
//                     let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
//                     for _ in 0..len {
//                         // TODO: handle DECREF this and other places?
//                         let dict_key = deserialize_dict_key(buf, ptr, pointers, str_count)?;
//                         if dict_key == *key {
//                             return lazy_deserialize(
//                                 buf,
//                                 ptr,
//                                 pointers,
//                                 use_tuples,
//                                 str_count,
//                                 custom_types,
//                                 dont_load,
//                                 path_to_load,
//                             )
//                         }
//                         skip_object(buf, ptr, pointers, str_count)?;
//                     }
//                     Err(format!(
//                         key_not_in_dict_template!(),
//                         py_str_to_rust_str(&unsafe { PyObject_Str(*key) })?,
//                         pretty_type(*key)
//                     ).to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }))
//                 }
//                 _ => {
//                     Err(
//                         format!(
//                             "Invalid path, expected `dict` but found `{}`",
//                             flag_to_type_name(flag)
//                         ).to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE })
//                     )
//                 }
//             }
//         }
//     }
// }
// 
// 
// fn skip_object(buf: &[u8], ptr: &mut usize, pointers: &mut usize, str_count: &mut usize) -> Result<(), *mut PyObject> {
//     let flag = *safe_get!(buf, *ptr);
//     *ptr += 1;
//     match flag {
//         POSITIVE_INT_FLAG | NEGATIVE_INT_FLAG => skip_number(buf, ptr),
//         FLOAT_FLAG => { *ptr += 8; Ok(()) },
//         STR_FLAG => todo!(),
//         ASCII_STR_FLAG => todo!(),
//         TRUE_FLAG => todo!(),
//         FALSE_FLAG => todo!(),
//         NULL_FLAG => todo!(),
//         POINTER_FLAG => todo!(),
//         POINTER_FLAG_1BYTE => todo!(),
//         POINTER_FLAG_2BYTE => todo!(),
//         POINTER_FLAG_3BYTE => todo!(),
//         POINTER_FLAG_4BYTE => todo!(),
//         BYTES_FLAG => todo!(),
//         CONSISTENT_TYPE_LIST_FLAG => todo!(),
//         DICT_FLAG => todo!(),
//         STR_KEY_DICT_FLAG => todo!(),
//         EMPTY_BYTES_FLAG => todo!(),
//         EMPTY_DICT_FLAG => todo!(),
//         EMPTY_LIST_FLAG => todo!(),
//         EMPTY_STR_FLAG => todo!(),
//         LIST_FLAG => todo!(),
//         LIST_OF_STRUCTURED_DICTS_FLAG => todo!(),
//         CUSTOM_TYPE_FLAG => todo!(),
//         _ => todo!(),
//     }
// }
// 
// 
// // def skip_object(buffer: bytes, pointer: int, settings: Settings) -> int:
// //     flag = buffer[pointer]
// //     pointer += 1
// //
// //     if flag < len(FIRST_FLAGS_LIST):
// //         return pointer
// //     elif flag == POSITIVE_INT_FLAG:
// //         return skip_number(buffer, pointer)
// //     elif flag == STR_KEY_DICT_FLAG:
// //         length, pointer = decode_number(buffer, pointer)
// //         for _ in range(length):
// //             if buffer[pointer] == NUMBER_BASE - 1:
// //                 pointer = skip_number(buffer, pointer + 1)
// //             else:
// //                 pointer = skip_string(buffer, pointer, settings, base=NUMBER_BASE - 1)
// //             pointer = skip_object(buffer, pointer, settings)
// //         return pointer
// //     elif flag == ASCII_STR_FLAG:
// //         return skip_string(buffer, pointer, settings)
// //     elif flag == STR_FLAG:
// //         return skip_string(buffer, pointer, settings)
// //     elif flag == DICT_FLAG:
// //         length, pointer = decode_number(buffer, pointer)
// //         for _ in range(length):
// //             if buffer[pointer] == STR_FLAG:
// //                 # fast path
// //                 pointer = skip_string(buffer, pointer + 1, settings)
// //             else:
// //                 pointer = skip_object(buffer, pointer, settings)
// //             pointer = skip_object(buffer, pointer, settings)
// //         return pointer
// //     elif flag == EMPTY_DICT_FLAG:
// //         return pointer
// //     elif flag == LIST_FLAG:
// //         length, pointer = decode_number(buffer, pointer)
// //         for _ in range(length):
// //             pointer = skip_object(buffer, pointer, settings)
// //         return pointer
// //     elif flag == EMPTY_LIST_FLAG:
// //         return pointer
// //     elif flag == CONSISTENT_TYPE_LIST_FLAG:
// //         typ_flag = buffer[pointer]
// //         length, pointer = decode_number(buffer, pointer + 1)
// //         if typ_flag == NULL_FLAG:
// //             return pointer
// //         elif typ_flag == BOOL_FLAG:
// //             length_in_bytes = (length + 7) >> 3
// //             return pointer + length_in_bytes
// //         elif typ_flag == BYTES_FLAG:
// //             for _ in range(length):
// //                 bytes_length, pointer = decode_number(buffer, pointer)
// //                 pointer += bytes_length
// //             return pointer
// //         elif typ_flag == STR_FLAG:
// //             for _ in range(length):
// //                 pointer = skip_string(buffer, pointer, settings)
// //             return pointer
// //         elif typ_flag == FLOAT_FLAG:
// //             return pointer + BYTES_IN_DOUBLE * length
// //         else:
// //             raise DeserializationError(f"Unexpected type flag: {typ_flag}")
// //     elif flag == NEGATIVE_INT_FLAG:
// //         return skip_number(buffer, pointer)
// //     elif flag == FLOAT_FLAG:
// //         return pointer + BYTES_IN_DOUBLE
// //     elif flag == BYTES_FLAG:
// //         length, pointer = decode_number(buffer, pointer)
// //         return pointer + length
// //     elif flag == POINTER_FLAG:
// //         return skip_number(buffer, pointer)
// //     elif flag == POINTER_FLAG_1BYTE:
// //         return pointer + 1
// //     elif flag == POINTER_FLAG_2BYTE:
// //         return pointer + 2
// //     elif flag == POINTER_FLAG_3BYTE:
// //         return pointer + 3
// //     elif flag == POINTER_FLAG_4BYTE:
// //         return pointer + 4
// //     elif flag == LIST_OF_STRUCTURED_DICTS_FLAG:
// //         list_length, pointer = decode_number(buffer, pointer)
// //         dict_length, pointer = decode_number(buffer, pointer)
// //         # first dict:
// //         for _ in range(dict_length):
// //             pointer = skip_object(buffer, pointer, settings)
// //             pointer = skip_object(buffer, pointer, settings)
// //         # rest of the dicts:
// //         for list_idx in range(1, list_length):
// //             for _ in range(dict_length):
// //                 pointer = skip_object(buffer, pointer, settings)
// //         return pointer
// //     elif flag == CUSTOM_TYPE_FLAG:
// //         pointer = skip_object(buffer, pointer, settings)
// //         pointer = skip_object(buffer, pointer, settings)
// //         return pointer
// //     elif flag < AMOUNT_OF_USED_FLAGS:
// //         raise DeserializationError("unexpected flag")
// //     else:
// //         return pointer