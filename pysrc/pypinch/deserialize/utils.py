from typing import Tuple

from pypinch.consts import ByteLike, NUMBER_BASE, ENDING_FLAG


def decode_number(buffer: ByteLike, pointer: int, base: int = NUMBER_BASE) -> Tuple[int, int]:
    """ numbers are stored in this format:
    if the number is smaller than ENDING_FLAG:
        just store the number (takes 1 byte), this is most cases so this is the most optimized
    if the number is bigger than ENDING_FLAG:
        first store an ENDING_FLAG (to differentiate from the numbers which are smaller than ENDING_FLAG)
        now, we know the number is at least `base` (so add that to the result)
        now store the number in base `base` up until we reach an ENDING_FLAG which signals the number is over
    """
    if buffer[pointer] != ENDING_FLAG:
        return buffer[pointer], pointer + 1
    power = 1
    res = base
    pointer += 1
    while buffer[pointer] != ENDING_FLAG:
        res += buffer[pointer] * power
        power *= base
        pointer += 1
    return res, pointer + 1


def skip_number(buffer: ByteLike, pointer: int) -> int:
    if buffer[pointer] != ENDING_FLAG:
        return pointer + 1
    pointer += 1
    while buffer[pointer] != ENDING_FLAG:
        pointer += 1
    return pointer + 1
