import os
from abc import ABC
from collections import UserString
from collections.abc import Mapping, Sequence
from io import BytesIO, StringIO
from typing import Any, BinaryIO, ClassVar, Generator, TypeAlias

from .pointer import Indent

_devnull: BinaryIO = open(os.devnull, "wb")

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
    "RAM",
]

Encoded: TypeAlias = bytes | bytearray | memoryview


class UTF8:
    encoded: Encoded
    dedent: int
    __slots__ = ("encoded", "dedent")

    def __init__(self, encoded: Encoded | str | UserString = b"", dedent: int = 0):
        match encoded:
            case str() | UserString():
                encoded = encoded.encode()
        if dedent == 0:
            # parser never sends us a zero, gotta be from the default
            if 10 not in encoded:
                dedent = -1
        elif dedent < 0:
            assert 10 not in encoded, "dedent says single line, but has newline"
        else:
            assert 10 in encoded, "dedent should be -1, this is a single line"
        self.encoded = encoded
        self.dedent = dedent

    def lines(self) -> Generator[Encoded]:
        limit = len(self.encoded)
        if self.dedent < 0 or limit == 0:
            yield self.encoded
            return
        start = index = 0
        while index < limit:
            if self.encoded[index] != 10:
                index += 1
            else:
                yield self.encoded[start:index]
                index += 1
                if index < limit:
                    tabs = self.dedent
                    while tabs > 0:
                        if index >= limit or self.encoded[index] != 9:
                            raise ValueError(f"dedent=={self.dedent} tabs=={tabs}")
                        index += 1
                        tabs -= 1
                start = index
        yield self.encoded[start:index]

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, UTF8):
            return False
        if self.dedent == other.dedent and self.encoded == other.encoded:
            return True
        return list(self.lines()) == list(other.lines())

    def __bytes__(self) -> bytes:
        return b"\n".join(self.lines())

    def __str__(self) -> str:
        return bytes(self).decode()

    def _repr_args(self, args: list[str]) -> None:
        if self.encoded:
            args.append(repr(bytes(self)))

    def __repr__(self) -> str:
        args = list[str]()
        self._repr_args(args)
        return f"{self.__class__.__name__}({','.join(args)})"


# =====================================================================================


class Comment(UTF8):
    __slots__ = ()

    def __init__(self, encoded: Encoded | str | UserString = b"", dedent: int = 0):
        super().__init__(encoded, dedent)


class Value(ABC):
    comment_after: Comment | None  # = None
    __slots__ = ()


class Text(UTF8, Value):
    __slots__ = ("comment_after",)

    def __init__(
        self,
        encoded: Encoded | str | UserString = b"",
        dedent: int = 0,
        after: Comment | None = None,
    ):
        super().__init__(encoded, dedent)
        self.comment_after = after

    def _repr_args(self, args: list[str]) -> None:
        super()._repr_args(args)  # lines
        if self.comment_after is not None:
            args.append(f"after={self.comment_after!r}")


class List(list[Value], Value):
    comment_intro: Comment | None  # = None
    __slots__ = ("comment_intro", "comment_after")

    def __init__(
        self,
        *values: Value,
        intro: Comment | None = None,
        after: Comment | None = None,
    ):
        super().__init__(values)
        self.comment_intro = intro
        self.comment_after = after

    def _repr_args(self, args: list[str]) -> None:
        for value in self:
            args.append(repr(value))
        if self.comment_intro is not None:
            args.append(f"intro={self.comment_intro!r}")
        if self.comment_after is not None:
            args.append(f"after={self.comment_after!r}")

    def __repr__(self) -> str:
        args = list[str]()
        self._repr_args(args)
        return f"{self.__class__.__name__}({','.join(args)})"


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

    def __init__(
        self,
        intro: "Comment | File | None" = None,
        after: Comment | None = None,
        /,
        **values: Value,
    ):
        if isinstance(intro, File):
            super().__init__(intro)
            self.comment_intro = intro.comment_intro
            self.update((Key(k), v) for k, v in values.items())
        else:
            super().__init__((Key(k), v) for k, v in values.items())
            self.comment_intro = intro
        self.comment_after = after

    def _repr_args(self, args: list[str]) -> None:
        if self.comment_after is not None:
            args.append(repr(self.comment_intro))
            args.append(repr(self.comment_after))
        elif self.comment_intro is not None:
            args.append(repr(self.comment_intro))
        for key, value in self.items():
            args.append(f"{key}={value!r}")

    def __repr__(self) -> str:
        args = list[str]()
        self._repr_args(args)
        return f"{self.__class__.__name__}({','.join(args)})"


class File(dict[Key, Value]):
    hashbang: Comment | None  # = None
    comment_intro: Comment | None  # = None
    __slots__ = ("hashbang", "comment_intro")

    def __init__(
        self,
        hashbang: Comment | None = None,
        intro: Comment | Dict | None = None,
        /,
        **values: Value,
    ):
        if isinstance(intro, Dict):
            super().__init__(intro)
            self.comment_intro = intro.comment_intro
            self.update((Key(k), v) for k, v in values.items())
        else:
            super().__init__((Key(k), v) for k, v in values.items())
            self.comment_intro = intro
        self.hashbang = hashbang

    def _repr_args(self, args: list[str]) -> None:
        if self.comment_intro is not None:
            args.append(repr(self.hashbang))
            args.append(repr(self.comment_intro))
        elif self.hashbang is not None:
            args.append(repr(self.hashbang))
        for key, value in self.items():
            args.append(f"{key}={value!r}")

    def __repr__(self) -> str:
        args = list[str]()
        self._repr_args(args)
        return f"{self.__class__.__name__}({','.join(args)})"


# ========================================================================= ThreadLocal


class RAM:
    empty: ClassVar[memoryview] = memoryview(b"")
    __slots__ = (
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
        self._errors = list[str]()
        self._indent: Indent = Indent(b"")
        self._count: int = 0
        self._write: BinaryIO = _devnull
        self._parse = RAM.empty
        self._next: int = -1
        self._line = RAM.empty
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
                    return File(None, result)
                case other:
                    raise AssertionError(f"impossible: got {type(other)}")
        finally:
            self._errors.clear()
            self._indent = self._indent.zero()

    def _value(self, any: Any) -> Value | None:
        match any:
            case None:
                return Text(b"", -1)
            case str() | UserString():
                return Text(any.encode(), 0)
            case bytes() | bytearray() | memoryview():
                return Text(any, 0)
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

    def encode(self, file: File, into: BytesIO | None = None) -> BytesIO:
        """uses and returns either `into` or (if None) a freshly allocated BytesIO"""
        self._errors.clear()
        self._indent = self._indent.zero()
        try:
            self._count = 0
            self._write = into or BytesIO()
            self._writeComment(b"#!", file.hashbang)
            self._writeDict(file)
            if self._errors:
                raise ValueError(self._error("illegal non-`Value` data"))
            return self._write
        finally:
            self._errors.clear()
            self._indent = self._indent.zero()
            self._write = _devnull

    def _writeIndent(self) -> None:
        if self._count:
            self._write.write(b"\n")
            self._count += 1
        else:
            self._count = 1
        self._write.write(self._indent._bytes)

    def _writeUTF8(self, utf8: UTF8) -> None:
        if len(self._indent) == utf8.dedent:
            self._write.write(utf8.encoded)
        elif utf8.encoded:
            lines = utf8.lines()
            self._write.write(next(lines))
            for line in lines:
                self._write.write(b"\n")
                self._write.write(self._indent._bytes)
                self._write.write(line)

    def _writeComment(self, marker: bytes, comment: Comment | None) -> None:
        if comment is not None:
            self._writeIndent()
            self._write.write(marker)
            self._indent = self._indent.more()
            self._writeUTF8(comment)
            self._indent = self._indent.less()

    def _shortList(self, text: Text) -> bool:
        """check if encoding should use short List item syntax."""
        if text.dedent >= 0:
            assert 10 in text.encoded, "dedent should be -1"
            return False
        if not text.encoded:
            return True
        assert 10 not in text.encoded, "dedent says single line, but has newline"
        return text.encoded[0] not in b"\t#<>[]{}/="

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
                        self._writeIndent()
                        self._writeUTF8(value)
                        self._indent = self._indent.less()
                    elif value.encoded:
                        self._write.write(value.encoded)
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
        if text.dedent >= 0:
            assert 10 in text.encoded, "dedent should be -1"
            return False
        assert 10 not in text.encoded, "dedent says single line, but has newline"
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
                self._count += 1  # because _writeIdent never called
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
                        self._writeIndent()
                        self._writeUTF8(value)
                        self._indent = self._indent.less()
                    else:
                        self._write.write(key.encode())
                        self._write.write(b"=")
                        self._write.write(value.encoded)
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

    def decode(self, buffer: Encoded) -> File:
        self._errors.clear()
        self._indent = self._indent.zero()
        try:
            self._count = 0
            self._parse = memoryview(buffer)
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
            self._errors.clear()
            self._indent = self._indent.zero()
            self._parse = self._line = RAM.empty

    def _readln(self) -> bool:
        index = self._next
        limit = len(self._parse)
        if index >= limit:
            if self._line is not RAM.empty:
                self._count += 1
                self._line = RAM.empty
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
            self._count = starting_line  # so error message has the right line
            if count > 1:
                self._errors_add(f"{count} lines excess indentation")
            else:
                self._errors_add("excess indentation")
            self._count += count

    def _readComment(self, start: int) -> Comment:
        indent = len(self._indent) + 1
        start += self._next - len(self._line) - 1
        newline = False
        while self._readln() and self._tabs >= indent:
            newline = True
        if self._tabs < 0:
            end = self._next
        else:
            end = self._next - len(self._line) - 2
        return Comment(self._parse[start:end], indent if newline else -1)

    def _readText(self) -> Text:
        indent = len(self._indent) + 1
        start = self._next + indent
        newline = -1
        while self._readln() and self._tabs >= indent:
            newline += 1
        if self._tabs < 0:
            end = self._next
        else:
            end = self._next - len(self._line) - 2
        return Text(self._parse[start:end], indent if newline > 0 else -1)

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
                    value = Text(b"", -1)
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
                    value = Text(self._line[indent:], -1)
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
                        value = Text(self._line[self._assign + 1 :], -1)
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
