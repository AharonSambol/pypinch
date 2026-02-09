from typing import Tuple

from pinch_python.consts import ByteLike, NUMBER_BASE, ENDING_FLAG


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
