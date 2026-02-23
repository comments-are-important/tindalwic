from io import BytesIO
from typing import Iterable, TypeAlias
from . import File, Dict, List, Text, Comment, Value, Key, Encoded

__all__ = ["YAML"]

LINES: TypeAlias = Iterable[Encoded]


class YAML:
    """Produces YAML that is not particularly aesthetically pleasing.

    This class prioritizes simple code that preserves all the input. No attempt is made
    to make the output look nice. A few load+dump cycles using `ruamel.yaml` round-trip
    can clean things up while preserving comments. More than one cycle might be needed
    if you care about reliability. If you take the output of `ruamel.yaml` and cycle it
    through another load+dump, it can change - typically as lists of lists slowly get
    squished into a single line and their intro comments get moved up. The test code
    includes a "cycle until it settles" method to accommodate this behavior.
    """

    def __init__(self):
        self._stream = BytesIO()

    def _write(self, *values: Encoded) -> None:
        for value in values:
            self._stream.write(value)

    def encode(self, file: File) -> BytesIO:
        try:
            self._comment(b"", b"!", file.hashbang)
            self._dict(b"", False, file)
            return self._stream
        finally:
            self._stream = BytesIO()

    def _utf8(self, indent: bytes, prefix: bytes, utf8: LINES) -> None:
        for line in utf8:
            self._write(indent, prefix, line, b"\n")

    def _comment_depth(self, indent: bytes) -> bytes:
        return b"#%d" % len(indent)

    def _comment_lines(self, marked: bytes, prefix: bytes, utf8: LINES) -> None:
        self._utf8(marked, prefix, utf8)

    def _comment(self, indent: bytes, prefix: bytes, comment: Comment | None) -> None:
        if comment is not None:
            marked = b"#" if prefix == b"!" else self._comment_depth(indent)
            self._comment_lines(marked, prefix, comment.lines())

    def _value(self, indent: bytes, key: Key | bool, value: Value) -> None:
        match value:
            case Text():
                self._text(indent, key, value)
            case List():
                self._list(indent, key, value)
            case Dict():
                self._dict(indent, key, value)
            case _:
                raise ValueError(f"unexpected type: {type(value)}")
        self._comment(indent, b"a:", value.comment_after)

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
                    self._comment_lines(self._comment_depth(indent), b"b", (b"",))
                self._comment(indent, b"k:", key.comment_before)
                self._value(indent, key, value)

    def _key(self, indent: bytes, key: Key | bool, end: bytes) -> None:
        self._write(indent)
        match key:
            case False:
                self._write(end)
            case True:
                self._write(b"-")
                if end:
                    self._write(b" ", end)
            case Key():
                safe = key.replace("\\", r"\\").replace('"', r"\"").replace("\t", r"\t")
                self._write(b'"', safe.encode(), b'":')
                if end:
                    self._write(b" ", end)
            case _:
                raise ValueError(f"unexpected type: {type(key)}")
        self._write(b"\n")
