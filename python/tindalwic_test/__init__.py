from collections.abc import Mapping
from cProfile import Profile
from io import BytesIO
from pathlib import Path
from time import perf_counter_ns
from typing import NamedTuple

from tindalwic import RAM, File, Encoded, Comment
from tindalwic.yaml import YAML

class Timer:
    def __init__(self, denominator: "Timer|None" = None):
        self.total = 0
        self.count = 0
        self.start = 0
        self.denom = denominator

    def __enter__(self):
        self.start = perf_counter_ns()
        return self

    def __exit__(self, exc_type, exc_value, traceback):
        self.total += perf_counter_ns() - self.start
        self.count += 1

    @property
    def avg(self) -> float:
        return self.total / self.count if self.count else 0

    @property
    def mul(self) -> float:
        denom = self.denom
        if denom is None:
            return 0
        denom = denom.avg
        if denom == 0:
            return 0
        return round(self.avg / denom, 2)


class FileSeparated(NamedTuple):
    file: File
    python: dict[str, str | list | dict]
    comments: list[str]
    yaml: bytes


class StealComments(YAML):
    def __init__(self):
        super().__init__()
        self.comments = list[str]()

    def _comment_lines(self, marked: bytes, prefix: bytes, comment: Comment) -> None:
        for line in comment.lines():
            if isinstance(line, memoryview):
                line = line.tobytes()
            self.comments.append(f"{marked.decode()}{prefix.decode()}{line.decode()}")
            super()._utf8(marked, prefix, (line,))


class TimedTindalwic:
    def __init__(self, pstats: Path | None):
        self.python_timer = Timer()
        self.file_timer = Timer()
        self.encode_timer = Timer()
        self.decode_timer = Timer()
        self.memory = RAM()
        self.pstats = pstats
        self.profile = Profile(builtins=False) if pstats else None

    def python(self, file: File) -> dict:
        with self.profile if self.profile else self.python_timer:
            return self.memory.python(file)

    def file(self, mapping: Mapping) -> File:
        with self.profile if self.profile else self.file_timer:
            return self.memory.file(mapping)

    def encode(self, file: File, into: BytesIO | None = None) -> BytesIO:
        with self.profile if self.profile else self.encode_timer:
            return self.memory.encode(file)

    def decode(self, buffer: Encoded) -> File:
        with self.profile if self.profile else self.decode_timer:
            return self.memory.decode(buffer)

    def timers(self) -> None:
        print("   tindalwic")
        if self.pstats and self.profile:
            self.profile.dump_stats(self.pstats)
            print(f"\t(written to {self.pstats})")
        else:
            print(f"\tencode = {self.encode_timer.avg}")
            print(f"\tdecode = {self.decode_timer.avg}")
            print(f"\tpython = {self.python_timer.avg}")
            print(f"\t  file = {self.file_timer.avg}")

    def separated(self, file: File) -> FileSeparated:
        python = self.python(file)
        steal = StealComments()
        yaml = steal.encode(file).getvalue()
        assert yaml == YAML().encode(file).getvalue()
        return FileSeparated(file, python, steal.comments, yaml)
