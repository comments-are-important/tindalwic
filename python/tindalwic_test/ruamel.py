from io import BytesIO
from typing import Any, TypeVar, NamedTuple

from ruamel.yaml import YAML as RuamelYAML
from ruamel.yaml.comments import CommentedMap, CommentedSeq, Comment as CommentObject
from ruamel.yaml.tokens import CommentToken
from ruamel.yaml.scalarstring import ScalarString

from tindalwic.yaml import YAML as TindalwicYAML
from . import FileSeparated, Timer, TimedTindalwic

T = TypeVar("T")


class RuamelSeparated(NamedTuple):
    ruamel: CommentedMap
    python: Any
    comments: list[str]


class StealComments:
    def __init__(self):
        self.comments = list[CommentToken]()
        self.empties = 0

    def tokens(self, any: Any) -> None:
        match any:
            case None | str():
                pass
            case CommentToken():
                self.comments.append(any)
            case CommentObject():
                self.tokens(any.pre)
                self.tokens(any.comment)
                self.tokens(any.end)
                self.tokens(any.items)
            case list():
                for value in any:
                    self.tokens(value)
            case dict():
                for value in any.values():
                    self.tokens(value)
            case _:
                raise ValueError(f"unexpected type: {type(any)}")

    def value(self, any: Any) -> Any:
        match any:
            case str():
                return any
            case CommentedSeq():
                if len(any) == 0:
                    self.empties += 1
                self.tokens(any.ca)
                return [self.value(it) for it in any]
            case CommentedMap():
                if len(any) == 0:
                    self.empties += 1
                self.tokens(any.ca)
                return {k: self.value(v) for k, v in any.items()}
            case ScalarString():
                return str(any)
            case BaseException():
                return str(any)
            case _:
                raise ValueError(f"unexpected type: {type(any)}")


class TimedRuamel:
    def __init__(self, memory: TimedTindalwic):
        self.trans_timer = Timer()
        self.dump_timer = Timer(memory.encode_timer)
        self.load_timer = Timer(memory.decode_timer)
        self.buffer = b""
        self.ruamel = RuamelYAML(typ="rt")
        if b"comment" in self.dump(self.load(b"{}\n#comment")):
            raise ValueError("ruamel fixed after empty bug")

    def load(self, yaml: bytes) -> Any:
        return self.ruamel.load(BytesIO(yaml))

    def dump(self, any: Any) -> bytes:
        buffer = BytesIO()
        self.ruamel.dump(any, buffer)
        return buffer.getvalue()

    def _load_file(self) -> CommentedMap:
        with self.load_timer:
            try:
                result = self.load(self.buffer)
            except BaseException as error:
                return error  # type: ignore
        if not isinstance(result, CommentedMap):
            raise AssertionError(f"expected CommentedMap, got: {type(result)}")
        return result

    def roundtrip(self, file: RuamelSeparated) -> RuamelSeparated | None:
        with self.dump_timer:
            self.buffer = self.dump(file.ruamel)
        return self.separated(self._load_file())

    def translate(self, file: FileSeparated) -> RuamelSeparated | None:
        with self.trans_timer:
            self.buffer = TindalwicYAML().encode(file.file).getvalue()
        assert self.buffer == file.yaml
        return self.separated(self._load_file())

    def timers(self, empties:int, loops:int) -> None:
        print("   ruamel.yaml RT")
        if empties:
            print(f"\t  skip = {empties} of {loops} or {round(empties*100.0/loops, 2)}%")
        print(f"\t  dump = {self.dump_timer.avg}  ({self.dump_timer.mul})")
        print(f"\t  load = {self.load_timer.avg}  ({self.load_timer.mul})")
        print(f"\t trans = {self.trans_timer.avg}")

    @staticmethod
    def separated(file: CommentedMap) -> RuamelSeparated | None:
        steal = StealComments()
        python = steal.value(file)
        if steal.empties:
            # ruamel drops comments that follow empty seq/map
            # https://sourceforge.net/p/ruamel-yaml/tickets/search/?q=empty
            return None
        steal.comments.sort(key=lambda it: it.start_mark.line)
        index = len(steal.comments) - 1
        while index > 0:
            if steal.comments[index] is steal.comments[index - 1]:
                # not sure why duplicates are stored, but it seems to be intentional.
                # detecting and correcting the situation is super easy and 100% safe...
                del steal.comments[index]
            index -= 1
        lines = list[str]()
        for comment in steal.comments:
            for line in comment.value.splitlines():
                line = line.lstrip()
                if line and line != "#":
                    lines.append(line)
        return RuamelSeparated(file, python, lines)
