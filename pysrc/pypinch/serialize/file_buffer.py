from typing import IO


class FileBuffer(bytearray):
    def __init__(self, source, writer: IO[bytes], flush_threshold: int, direct_write_threshold: int):
        self.writer = writer
        self.flush_threshold = flush_threshold
        self.direct_write_threshold = direct_write_threshold
        super().__init__(source)

    def append(self, __item):
        super().append(__item)
        if super().__len__() >= self.flush_threshold:
            self.flush()

    def extend(self, __iterable_of_ints: bytes):
        if len(__iterable_of_ints) >= self.flush_threshold:
            self.flush()
            self.writer.write(__iterable_of_ints)
        else:
            super().extend(__iterable_of_ints)
            if super().__len__() >= self.flush_threshold:
                self.flush()

    def flush(self) -> None:
        self.writer.write(self)
        super().clear()
