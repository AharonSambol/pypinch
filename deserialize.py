from typing import Tuple

from consts import NUMBER_BASE, ENDING_FLAG, ByteLike


def decode_number(num: ByteLike, pointer: int) -> Tuple[int, int]:
    power = res = 0
    while num[pointer] != ENDING_FLAG:
        res += num[pointer] * NUMBER_BASE ** power
        power += 1
        pointer += 1
    return res, pointer + 1
