from abc import ABC
from collections import UserString
from collections.abc import Mapping, Sequence
from io import BytesIO, StringIO
from typing import Any, ClassVar, TypeAlias

from .pointer import Indent

# multiple inheritance (from builtins in particular) means that __slots__ and __init__
# must be written as below to avoid TypeError about instance lay-out conflict.

__all__ = [
    "Encoded",
    "UTF8",
    "Comment",
    "Value",
    "Text",
    "List",
    "Key",
    "Dict",
    "File",
    "ALACS",
]

Encoded: TypeAlias = bytes | bytearray | memoryview


class UTF8(list[Encoded]):
    __slots__ = ()

    def __init__(self, *lines: Encoded | str | UserString):
        super().__init__(lines)  # type: ignore
        for index, line in enumerate(self):
            if isinstance(line, (str, UserString)):
                self[index] = line.encode()

    def __bytes__(self) -> bytes:
        return b"\n".join(self)

    def __str__(self) -> str:
        return bytes(self).decode()

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__}={str(self)}>"

    def normalize(self, scratch: list[Encoded] | None) -> None:
        if 1 == len(self) and 0 == len(self[0]):
            self.clear()  # `[]` is more "True"ly empty than `[b'']`
        else:
            if scratch is None:
                scratch = list[Encoded]()
            else:
                scratch.clear()
            for index in range(len(self) - 1, -1, -1):
                chunk = self[index]
                if not isinstance(chunk, memoryview):
                    if b"\n" not in chunk:
                        continue
                    chunk = memoryview(chunk)
                start = len(chunk) - 1
                while 0 <= start and 10 != chunk[start]:
                    start -= 1
                if 0 <= start:
                    scratch.append(chunk[start + 1 :])
                    limit = start
                    start -= 1
                    while start >= 0:
                        if 10 == chunk[start]:
                            scratch.append(chunk[start + 1 : limit])
                            limit = start
                        start -= 1
                    scratch.append(chunk[:limit])
                    scratch.reverse()
                    self[index : index + 1] = scratch
                    scratch.clear()


# =====================================================================================


class Comment(UTF8):
    __slots__ = ()

    def __init__(self, *lines: Encoded | str | UserString):
        super().__init__(*lines)


class Value(ABC):
    comment_after: Comment | None  # = None
    __slots__ = ()


class Text(UTF8, Value):
    __slots__ = ("comment_after")

    def __init__(self, *lines: Encoded | str | UserString):
        super().__init__(*lines)
        self.comment_after = None


class List(list[Value], Value):
    comment_intro: Comment | None  # = None
    __slots__ = ("comment_intro", "comment_after")

    def __init__(self, *values: Value):
        super().__init__(values)
        self.comment_intro = None
        self.comment_after = None


class Key(str):
    blank_line_before: bool  # = False
    comment_before: Comment | None  # = None
    __slots__ = ("blank_line_before", "comment_before")

    def __init__(self, handled_by_str_new_but_here_for_type_hint: Any):
        if "\n" in self:
            raise ValueError("newline in key")
        self.blank_line_before = False
        self.comment_before = None


class Dict(dict[Key, Value], Value):
    comment_intro: Comment | None  # = None
    __slots__ = ("comment_intro", "comment_after")

    def __init__(self, **values: Value):
        super().__init__((Key(k), v) for k, v in values.items())
        self.comment_intro = None
        self.comment_after = None


class File(dict[Key, Value]):
    hashbang: Comment | None  # = None
    comment_intro: Comment | None  # = None
    __slots__ = ("hashbang", "comment_intro")

    def __init__(self, **values: Value):
        super().__init__((Key(k), v) for k, v in values.items())
        self.hashbang = None
        self.comment_intro = None


# ========================================================================= ThreadLocal


class ALACS:
    empty: ClassVar[memoryview] = memoryview(b"")
    __slots__ = (
        "_scratch",
        "_errors",
        "_indent",
        "_count",
        "_write",
        "_parse",
        "_next",
        "_line",
        "_tabs",
        "_assign",
    )

    def __init__(self):
        super().__init__()
        self._scratch = list[Encoded]()
        self._errors = list[str]()
        self._indent: Indent = Indent(b"")
        self._count: int = 0
        self._write = BytesIO()
        self._parse = ALACS.empty
        self._next: int = -1
        self._line = ALACS.empty
        self._tabs: int = -1
        self._assign: int = -1

    # -------------------------------------------------------------------------- errors

    def _error(self, message: str) -> str:
        match self._errors:
            case []:
                return message
            case _:
                return f"{message}:\n\t{'\n\t'.join(self._errors)}"

    def _errors_add(self, *parts: Any) -> None:
        message = StringIO()
        if self._count:
            message.write(f"#{self._count}: ")
        for part in parts:
            message.write(f"{part} ")
        message.write("@")
        self._indent.path(message)
        self._errors.append(message.getvalue())

    # ----------------------------------------------------------------------- to python

    def python(self, file: File) -> dict:
        """Convert a `Value` to simple Python data.

        Returns a deep copy with any `Text` replaced by `str` instances,
        and the arrays replaced by their builtin analogs.
        """
        self._errors.clear()
        self._indent = self._indent.zero()
        try:
            match self._python(file):
                case _ if self._errors:
                    raise ValueError(self._error("illegal non-`Value` data"))
                case None:
                    raise AssertionError("impossible: got None, but no error")
                case result if isinstance(result, dict):
                    return result
                case other:
                    raise AssertionError(f"impossible: got {type(other)}")
        finally:
            self._errors.clear()
            self._indent = self._indent.zero()

    def _python(self, any: Value | File) -> str | list | dict | None:
        match any:
            case Text():
                return str(any)
            case List():
                result = list()
                self._indent = self._indent.more()
                for key, value in enumerate(any):
                    self._indent.key = key
                    result.append(self._python(value))
                self._indent = self._indent.less()
                return result
            case Dict() | File():
                result = dict()
                self._indent = self._indent.more()
                for key, value in any.items():
                    self._indent.key = key
                    match key:
                        case Key():
                            result[str(key)] = self._python(value)
                        case other:
                            self._errors_add("key is", type(other))
                self._indent = self._indent.less()
                return result
        self._errors_add("value is", type(any))
        return None

    # --------------------------------------------------------------------- from python

    def file(self, mapping: Mapping) -> File:
        """Convert a simple Python `dict` (any mapping) to a `File`.

        Returns deep copy except bytes/bytearray/memoryview are shared
        (even though some of those are mutable)."""
        self._errors.clear()
        self._indent = self._indent.zero()
        try:
            match self._value(mapping):
                case _ if self._errors:
                    raise ValueError(self._error("can't be converted to `Value`"))
                case None:
                    raise AssertionError("impossible: got None, but no error")
                case Dict() as result:
                    file = File()
                    file.update(result)
                    file.comment_intro = result.comment_intro
                    return file
                case other:
                    raise AssertionError(f"impossible: got {type(other)}")
        finally:
            self._scratch.clear()
            self._errors.clear()
            self._indent = self._indent.zero()

    def _value(self, any: Any) -> Value | None:
        match any:
            case None:
                return Text()
            case str() | UserString() | bytes() | bytearray() | memoryview():
                result = Text(any)
                result.normalize(self._scratch)
                return result
            case Sequence():
                result = List()
                self._indent = self._indent.more()
                for i, v in enumerate(any):
                    self._indent.key = i
                    x = self._value(v)
                    if x is not None:
                        result.append(x)
                self._indent = self._indent.less()
                return result
            case Mapping():
                result = Dict()
                self._indent = self._indent.more()
                for k, v in any.items():
                    self._indent.key = k
                    if isinstance(k, (str, UserString)):
                        x = self._value(v)
                        if x is not None:
                            result[Key(k)] = x
                    else:
                        self._errors_add("key is", type(k))
                self._indent = self._indent.less()
                return result
        self._errors_add("value is", type(any))
        return None

    # -------------------------------------------------------------------------- encode

    def encode(self, file: File) -> memoryview:
        """result must be `release`d - suggest use `with`."""
        self._errors.clear()
        self._indent = self._indent.zero()
        try:
            self._count = 0
            self._write.seek(0)
            self._write.truncate()
            self._writeComment(b"#!", file.hashbang)
            self._writeDict(file)
            if self._errors:
                raise ValueError(self._error("illegal non-`Value` data"))
            return self._write.getbuffer()
        finally:
            self._scratch.clear()
            self._errors.clear()
            self._indent = self._indent.zero()

    def _writeIndent(self) -> None:
        if self._count:
            self._write.write(b"\n")
            self._count += 1
        else:
            self._count = 1
        self._write.write(self._indent._bytes)

    def _writeComment(self, marker: bytes, comment: Comment | None) -> None:
        if comment is not None:
            self._writeIndent()
            self._write.write(marker)
            comment.normalize(self._scratch)
            if comment:
                self._write.write(comment[0])
                self._indent = self._indent.more()
                for index in range(1, len(comment)):
                    self._writeIndent()
                    self._write.write(comment[index])
                self._indent = self._indent.less()

    def _shortList(self, text: Text) -> bool:
        """check if encoding should use short List item syntax."""
        text.normalize(self._scratch)
        match len(text):
            case 0:
                return True
            case 1:
                return text[0][0] not in b"\t#<>[]{}/="
        return False

    def _writeList(self, array: List) -> None:
        self._writeComment(b"#", array.comment_intro)
        for index, value in enumerate(array):
            self._indent.key = index
            self._writeIndent()
            match value:
                case Text():
                    if not self._shortList(value):
                        self._write.write(b"<>")
                        self._indent = self._indent.more()
                        for line in value:
                            self._writeIndent()
                            self._write.write(line)
                        self._indent = self._indent.less()
                    elif value:
                        self._write.write(value[0])
                case List():
                    self._write.write(b"[]")
                    self._indent = self._indent.more()
                    self._writeList(value)
                    self._indent = self._indent.less()
                case Dict():
                    self._write.write(b"{}")
                    self._indent = self._indent.more()
                    self._writeDict(value)
                    self._indent = self._indent.less()
                case _:
                    self._errors_add("value is", type(value))
                    continue
            self._writeComment(b"#", value.comment_after)

    def _shortDict(self, key: Key, text: Text) -> bool:
        """check if encoding should use short Dict entry syntax."""
        text.normalize(self._scratch)
        if len(text) > 1:
            return False
        if not key:
            return True
        if key[0] in "\t#<>[]{}/":
            return False
        if "=" in key:
            return False
        return True

    def _writeDict(self, array: Dict | File) -> None:
        self._writeComment(b"#", array.comment_intro)
        for key, value in array.items():
            self._indent.key = key
            if not isinstance(key, Key):
                self._errors_add("key is", type(key))
                continue
            if key.blank_line_before:
                self._writeIndent()
            self._writeComment(b"//", key.comment_before)
            self._writeIndent()
            match value:
                case Text():
                    if not self._shortDict(key, value):
                        self._write.write(b"<")
                        self._write.write(key.encode())
                        self._write.write(b">")
                        self._indent = self._indent.more()
                        for line in value:
                            self._writeIndent()
                            self._write.write(line)
                        self._indent = self._indent.less()
                    else:
                        self._write.write(key.encode())
                        self._write.write(b"=")
                        if value:
                            self._write.write(value[0])
                case List():
                    self._write.write(b"[")
                    self._write.write(key.encode())
                    self._write.write(b"]")
                    self._indent = self._indent.more()
                    self._writeList(value)
                    self._indent = self._indent.less()
                case Dict():
                    self._write.write(b"{")
                    self._write.write(key.encode())
                    self._write.write(b"}")
                    self._indent = self._indent.more()
                    self._writeDict(value)
                    self._indent = self._indent.less()
                case _:
                    self._errors_add("value is", type(value))
                    continue
            self._writeComment(b"#", value.comment_after)

    # -------------------------------------------------------------------------- decode

    def decode(self, alacs: Encoded) -> File:
        self._errors.clear()
        self._indent = self._indent.zero()
        try:
            self._count = 0
            self._parse = memoryview(alacs)
            self._next = 0
            self._indent.key = ""
            self._readln()
            file = File()
            if len(self._line) > 1 and self._line[0] == 35 and self._line[1] == 33:
                file.hashbang = self._readComment(2)
            self._readDict(file)
            if self._errors:
                raise ValueError(self._error("parse errors"))
            return file
        finally:
            self._scratch.clear()
            self._errors.clear()
            self._indent = self._indent.zero()
            self._parse = self._line = ALACS.empty

    def _readln(self) -> bool:
        index = self._next
        limit = len(self._parse)
        if index >= limit:
            if self._line is not ALACS.empty:
                self._count += 1
                self._line = ALACS.empty
                self._tabs = self._assign = -1
            return False
        byte = self._parse[index]
        while byte == 9:
            index += 1
            if index >= limit:
                byte = 10
                break
            byte = self._parse[index]
        self._tabs = index - self._next
        self._assign = -1
        while byte != 10:
            if byte == 61 and self._assign < 0:
                self._assign = index - self._next
            index += 1
            if index >= limit:
                break
            byte = self._parse[index]
        self._line = self._parse[self._next : index]
        self._count += 1
        self._next = index + 1
        return True

    def _readExcess(self) -> None:
        if self._tabs > len(self._indent):
            starting_line = self._count
            count = 1
            while self._readln() and self._tabs > len(self._indent):
                count += 1
            self._count = starting_line # so error message points to this
            if count > 1:
                self._errors_add(f"{count} lines excess indentation")
            else:
                self._errors_add("excess indentation")
            self._count += count

    def _readComment(self, start: int) -> Comment:
        comment = Comment(self._line[start:])
        indent = len(self._indent) + 1
        while self._readln() and self._tabs >= indent:
            comment.append(self._line[indent:])
        return comment

    def _readText(self) -> Text:
        text = Text()
        indent = len(self._indent) + 1
        while self._readln() and self._tabs >= indent:
            text.append(self._line[indent:])
        if len(text) == 1 and len(text[0]) == 0:
            text.clear()
        return text

    def _readList(self, array: List) -> None:
        self._indent.key = ""
        self._readExcess()
        indent = len(self._indent)
        if self._tabs < indent:
            return
        if len(self._line) > indent and self._line[indent] == 35:
            array.comment_intro = self._readComment(indent + 1)
        self._readExcess()
        while self._tabs == indent:
            self._indent.key = len(array)
            size = len(self._line) - indent
            assert size >= 0
            value: Value | None = None
            match 10 if size == 0 else self._line[indent]:
                case 10:
                    value = Text()
                    self._readln()
                case 35:
                    self._errors_add("unattached comment")
                    self._readComment(indent)
                case 47:
                    self._errors_add("key comment in list context")
                    self._readComment(indent)
                case 60:
                    if size != 2 or self._line[-1] != 62:
                        self._errors_add("malformed text opening")
                        self._readln()
                    else:
                        value = self._readText()
                case 91:
                    if size != 2 or self._line[-1] != 93:
                        self._errors_add("malformed linear array opening")
                        self._readln()
                    else:
                        value = List()
                        self._readln()
                        self._indent = self._indent.more()
                        self._readList(value)
                        self._indent = self._indent.less()
                case 123:
                    if size != 2 or self._line[-1] != 125:
                        self._errors_add("malformed associative array opening")
                        self._readln()
                    else:
                        value = Dict()
                        self._readln()
                        self._indent = self._indent.more()
                        self._readDict(value)
                        self._indent = self._indent.less()
                case _:
                    value = Text(self._line[indent:])
                    self._readln()
            if value is not None:
                if self._tabs == indent:
                    if len(self._line) > indent and self._line[indent] == 35:
                        value.comment_after = self._readComment(indent + 1)
                array.append(value)
            self._readExcess()

    def _readDict(self, array: Dict | File) -> None:
        self._indent.key = ""
        self._readExcess()
        indent = len(self._indent)
        if self._tabs < indent:
            return
        if len(self._line) > indent and self._line[indent] == 35:
            array.comment_intro = self._readComment(indent + 1)
        entries = 0
        blank = 0
        comment: Comment | None = None
        self._readExcess()
        while self._tabs == indent:
            size = len(self._line) - indent
            assert size >= 0
            if size == 0:
                if comment:
                    self._errors_add("blank line must precede key comment")
                elif blank:
                    self._errors_add("more than one blank line")
                else:
                    blank = self._count
                self._readln()
                continue
            key: Key | None = None
            self._indent.key = key
            value: Value | None = None
            match self._line[indent]:
                case 35:
                    self._errors_add("illegal position for comment")
                    self._readComment(indent + 1)
                case 47:
                    if size < 2 or self._line[indent + 1] != 47:
                        self._errors_add("malformed key comment")
                        self._readComment(indent)
                    elif comment:
                        self._errors_add("more than one key comment")
                        self._readComment(indent)
                    else:
                        comment = self._readComment(indent + 2)
                case 60:
                    if size < 2 or self._line[-1] != 62:
                        self._errors_add("malformed text opening")
                        self._readln()
                    else:
                        key = Key(self._line[indent + 1 : -1].tobytes().decode())
                        self._indent.key = key
                        value = self._readText()
                case 91:
                    if size < 2 or self._line[-1] != 93:
                        self._errors_add("malformed linear array opening")
                        self._readln()
                    else:
                        key = Key(self._line[indent + 1 : -1].tobytes().decode())
                        self._indent.key = key
                        self._readln()
                        value = List()
                        self._indent = self._indent.more()
                        self._readList(value)
                        self._indent = self._indent.less()
                case 123:
                    if size < 2 or self._line[-1] != 125:
                        self._errors_add("malformed associative array opening")
                        self._readln()
                    else:
                        key = Key(self._line[indent + 1 : -1].tobytes().decode())
                        self._indent.key = key
                        self._readln()
                        value = Dict()
                        self._indent = self._indent.more()
                        self._readDict(value)
                        self._indent = self._indent.less()
                case _:
                    if self._assign < 0:
                        self._errors_add("malformed `key=value` association")
                        self._readln()
                    else:
                        key = Key(self._line[indent : self._assign].tobytes().decode())
                        self._indent.key = key
                        value = Text(self._line[self._assign + 1 :])
                        self._readln()
            if value is None:
                assert key is None
            else:
                if self._tabs == indent:
                    if len(self._line) > indent and self._line[indent] == 35:
                        value.comment_after = self._readComment(indent + 1)
                assert key is not None
                array[key] = value
                if blank:
                    key.blank_line_before = True
                    blank = False
                if comment:
                    key.comment_before = comment
                    comment = None
                entries += 1
                if len(array) != entries:
                    self._errors_add(f"duplicate key: {key}")
            self._readExcess()
        if comment or blank:
            self._errors_add("unclaimed key comment or blank line")
