from io import BytesIO
from typing import Iterable, BinaryIO
from . import _devnull, File, Dict, List, Text, Comment, Value, Key, Encoded

__all__ = ["YAML"]


class YAML:
    """Produces YAML that is not particularly aesthetically pleasing.

    This class prioritizes simple code that preserves all the input. No attempt is made
    to make the output look nice. A load+dump cycle using `ruamel.yaml` round-trip (so
    comments are preserved) can clean things up.
    """

    def __init__(self):
        self._blank = Comment()
        self._write: BinaryIO = _devnull

    def encode(self, file: File, into: BytesIO | None = None) -> BytesIO:
        try:
            self._write = into or BytesIO()
            self._comment(b"", b"!", file.hashbang)
            self._dict(b"", False, file)
            return self._write
        finally:
            self._write = _devnull

    def _utf8(self, indent: bytes, prefix: bytes, utf8: Iterable[Encoded]) -> None:
        for line in utf8:
            self._write.write(indent)
            self._write.write(prefix)
            self._write.write(line)
            self._write.write(b"\n")

    def _comment_lines(self, marked: bytes, prefix: bytes, comment: Comment) -> None:
            self._utf8(marked, prefix, comment.lines())

    def _comment(self, indent: bytes, prefix: bytes, comment: Comment | None) -> None:
        if comment is not None:
            marked = b"#" if prefix == b"!" else b"#%d" % len(indent)
            self._comment_lines(marked, prefix, comment)

    def _value(self, indent: bytes, key: Key | bool, value: Value) -> None:
        match value:
            case Text():
                self._comment(indent, b"a:", value.comment_after)
                self._text(indent, key, value)
            case List():
                self._comment(indent, b"a:", value.comment_after)
                self._list(indent, key, value)
            case Dict():
                self._comment(indent, b"a:", value.comment_after)
                self._dict(indent, key, value)
            case _:
                raise ValueError(f"unexpected type: {type(value)}")

    def _text(self, indent: bytes, key: Key | bool, text: Text) -> None:
        value = list(text.lines())
        if len(value) == 1 and not value[0]:
            value.clear()
        if not value:
            self._key(indent, key, b'""')
        elif value[-1]:
            self._key(indent, key, b"|2-")
            self._utf8(indent, b"  ", value)
        else:
            self._key(indent, key, b"|2+")
            self._utf8(indent, b"  ", value[:-1])

    def _list(self, indent: bytes, key: Key | bool, items: List) -> None:
        if not items:
            self._key(indent, key, b"[]#")  # comment is to help ruamel.yaml
        else:
            assert key is not False
            self._key(indent, key, b"")
            indent = indent + b" "
            self._comment(indent, b"i:", items.comment_intro)
            for value in items:
                self._value(indent, True, value)

    def _dict(self, indent: bytes, key: Key | bool, entries: Dict | File) -> None:
        if not entries:
            self._key(indent, key, b"{}#")  # comment is to help ruamel.yaml
        else:
            if key is not False:
                self._key(indent, key, b"")
                indent = indent + b" "
            self._comment(indent, b"i:", entries.comment_intro)
            for key, value in entries.items():
                if key.blank_line_before:
                    self._comment(indent, b"b", self._blank)
                self._comment(indent, b"k:", key.comment_before)
                self._value(indent, key, value)

    def _key(self, indent: bytes, key: Key | bool, end: bytes) -> None:
        self._write.write(indent)
        match key:
            case False:
                self._write.write(end)
            case True:
                self._write.write(b"-")
                if end:
                    self._write.write(b" ")
                    self._write.write(end)
            case Key():
                self._write.write(b'"')
                self._write.write(
                    key.replace("\\", r"\\")
                    .replace('"', r"\"")
                    .replace("\t", r"\t")
                    .encode()
                )
                self._write.write(b'":')
                if end:
                    self._write.write(b" ")
                    self._write.write(end)
            case _:
                raise ValueError(f"unexpected type: {type(key)}")
        self._write.write(b"\n")
