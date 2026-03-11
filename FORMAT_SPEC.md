# Pinch

## Limitations
- Dictionary keys can't be lists

Yup, that's it :)

## Supported Types
- List
- Dictionary (HashMap/Objects/...)
- Integer (up to infinit sizes)
- Float
- String
- Bytes
- Boolean
- Null

## Basic Format
The basic idea is rather simple. Have a byte which represents what type the next object is, then sometimes a number 
representing the length of the object (depending on the type). And finally, the data of the serialized object.
Additionaly each pinch object starts with te header `<o>`.

For example, if I wanted to serialize `[1, "hello"]` it would look like this:
```
<o><<ListFlag>><<len of list (2)>><<IntFlag>><<serialized 1>><<StringFlag>><<len of string (5)>><<serialized "hello">>
^^^ 
Pinch header
```
Which would look like this:
```
0x3c 0x6f 0x3e 0x0e 0x02 0x1f 0x13 0x05 0x68 0x65 0x6c 0x6c 0x6f
```
```0x3c 0x6f 0x3e``` -> Pinch header `<o>`
<br/>
```0x0e``` -> List flag
<br/>
```0x02``` -> Length of the list (2)
<br/>
```0x1f``` -> Flag specifically for the number 1
<br/>
```0x13``` -> String flag
<br/>
```0x05``` -> Length of the string (5)
<br/>
```0x68 0x65 0x6c 0x6c 0x6f``` -> Serialized string (utf-8 encoded)

## How numbers are stored
### Integers
An actual number in the input data, which can be any integer, positive or negative.

**If the number is between 0 and 224 (inclusive):**

It is stored as one byte - 30 + the value of the number. e.g.
<br/>
`0` -> `0x1e`
<br/>
`10` -> `0x28`
<br/>
`25` -> `0x37`
<br/>
`224` -> `0xfe` 

**Otherwise**

A flag is used to show whether it is a positive or negative number. `0x07` for positive and `0x08` 
for negative ([flags](#Flags)). 
<br/>
Then the number is stored like [non-integer numbers](#Non-Integers) (described below) in base 255.

### Non-Integers
These numbers are usually "metadata", e.g. the length of a string or list. 

These numbers are always positive and are stored in either base 255 or base 254 depending on the context.
* If the number is smaller than the base it's being stored in, it will be stored simply as a byte of that value.
* Otherwise, it will be stored as following:
<br/>
First subtract the base from the number. We know to add this back because if it was smaller than the base it would have
been stored in the previous method. And in order to distinguish between the methods, in this method we will add a `0xff`
at the start.
<br/>
Next, encode the number into the base. E.g. if the base is 255 and the number is 257, it will be `0x01 0x02` 
<br/>
Add a `0xff` so that we know where the number ends (Sort of like `\0`)

The reason it is done like this is that it allows us to store small numbers extremely compactly, while still allowing 
us to store numbers up to any size with no limitations.


## Flags
Note that when an object can be serialized by one of multiple flags, there is no assurance as to which flag will be chosen.
* [empty str flag](#empty-string-flag): `0x00`
* [empty bytes flag](#empty-bytes-flag): `0x01`
* [true flag](#true-flag): `0x02`
* [false flag](#false-flag): `0x03`
* [null flag](#null-flag): `0x04`
* [empty list flag](#empty-list-flag): `0x05`
* [empty dict flag](#empty-dict-flag): `0x06`
* [positive int flag](#positive-int-flag): `0x07`
* [negative int flag](#negative-int-flag): `0x08`
* [float flag](#float-flag): `0x09`
* [str flag](#str-flag): `0x0a`
* [bytes flag](#bytes-flag): `0x0b`
* [bool flag](#bool-flag): `0x0c`
* [list flag](#list-flag): `0x0d`
* [consistent type list flag](#consistent-type-list-flag): `0x0e`
* [dict flag](#dict-flag): `0x0f`
* [str key dict flag](#str-key-dict-flag): `0x10`
* [pointer flag](#pointer-flag): `0x11`
* [ascii str flag](#ascii-str-flag): `0x12`
* [list of structured dicts flag](#list-of-structured-dicts-flag): `0x13`

### Empty String Flag
As it sounds, it represents an empty string.

### Empty Bytes Flag
As it sounds, it represents an empty bytes object.

### True Flag
As it sounds, it represents a boolean `true`.

### False Flag
As it sounds, it represents a boolean `false`.

### Null Flag
As it sounds, it represents a null value.

### Empty List Flag
As it sounds, it represents an empty list.

### Empty Dict Flag
As it sounds, it represents an empty dictionary.

### Positive Int Flag
Described in [Numbers](#Integers)

### Negative Int Flag
Described in [Numbers](#Integers)

### Float Flag
A double stored in big-endian IEEE 754 binary64

### Str Flag
First, [the length of the string is stored](#Non-Integers) (base 255), then the string itself encoded in utf-8.

### Bytes Flag
First, [the length of the bytes is stored](#Non-Integers) (base 255), then the bytes themselves.

### Bool Flag
Used only in [consistent type lists](#Consistent-Type-Lists) to show that the list is of booleans.
<br/> 
Usually, [true flag](#true-flag) or [false flag](#false-flag) are used.

### List Flag
First, [the length of the list is stored](#Non-Integers) (base 255), then all the elements of the list, one after 
the other.

### Consistent Type List Flag
Consistent type lists are lists where all the elements are of the same type.
Sometimes this lets us do some optimizations. 
<br/>
There are only some supported types of "consistent type lists".

First, [the length of the list is stored](#Non-Integers) (base 255), and then the type of the elements:

#### Boolean ([bool flag](#bool-flag): `0x0c`)
Each element is stored as a single bit, 1 for true and 0 for false. With right padding to fill the last byte.
#### None ([null flag](#null-flag): `0x04`)
That's it. If we know the length and the type (null) there is nothing else that needs to be stored.

### Dict Flag
First, [the length of the dict is stored](#Non-Integers) (base 255). This is the amount of keys (or values).
<br/>
Then for each key/value pair, first the key is stored, then the value (including their types).

### Str Key Dict Flag
Same as [dict flag](#dict-flag), except we know all the keys are strings, so when serializing the key, we don't store the type.
<br/>
However, in order to allow the use of [pointers](#pointer-flag), the keys' length is stored in base 254. This way we
can store a key as a pointer by starting with 254 (0xfe) and then the position we want to point to.
<br/>
In a similar fashion, keys might also start with 0xfe (after the length, as part of the data), this signals that the key
is [ASCII](#ascii-str-flag). In these cases, the extra byte will be counted in the length (so the actual length of the
key is one less than stated) 

### Pointer Flag
Instead of storing the same string twice, a pointer can be used which points at which string it's a duplicate of.
<br/>
A pointer is simply a [number](#Non-Integers) (base 255) that represents the index of the string which it is a duplicat of.
<br/>
The index is not the position in the encoded bytes but rather just a counter which is incremented each time a unique string 
is serialized. 
<br/>
A pointer cannot point at a string which appears after itself in the encoded bytes. It will always point backwards.

### ASCII Str Flag
Same as [str flag](#str-flag), but indicates that the string is plain ASCII, and not the normal UTF-8. (This is used as an optimization)

### List of Structured Dicts Flag
This is used to store a list of dictionaries, where all the dictionaries have the same keys, and all the keys are strings.
<br/>
First, [the length of the dict is stored](#Non-Integers) (base 255). Then the first dict is stored normally, but each 
consecutive dict is stored without its length or its keys, just the values, one after the other.

