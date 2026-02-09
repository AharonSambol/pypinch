from pypinch.consts import NUMBER_BASE, ENDING_FLAG


def encode_number(buffer: bytearray, num: int, base: int = NUMBER_BASE) -> None:
    if num < base:
        buffer.append(num)
    else:
        buffer.append(ENDING_FLAG)
        while num:
            num, remainder = divmod(num, base)
            buffer.append(remainder)
        buffer.append(ENDING_FLAG)
