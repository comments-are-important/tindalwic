from io import BytesIO
from . import File, Dict, List, Text, Comment, Value, Key, Encoded

__all__ = ["YAML"]


class YAML(BytesIO):
    """Produces YAML that is not particularly aesthetically pleasing.

    This class prioritizes simple code that preserves all the input. No attempt is made
    to make the output look nice. A load+dump cycle using `ruamel.yaml` round-trip (so
    comments are preserved) can clean things up.
    """

    def __init__(self):
        self._scratch = list[Encoded]()
        self._blank = Comment()

    def encode(self, file: File) -> BytesIO:
        self.seek(0)
        self.truncate()
        self.write(b"--- !map\n")
        self._comment(b"", b"!", file.hashbang)
        self._dict(b"",False, file)
        self.write(b"...\n")
        return self

    def _utf8(self, indent: bytes, prefix: bytes, utf8: list[Encoded]) -> None:
        for line in utf8:
            self.write(indent)
            self.write(prefix)
            self.write(line)
            self.write(b"\n")

    def _comment(self, indent: bytes, prefix: bytes, comment: Comment | None) -> None:
        if comment is not None:
            marked = b"#" if prefix == b"!" else b"#%d" % len(indent)
            comment.normalize(self._scratch)
            self._utf8(indent + marked, prefix, comment)

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

    def _text(self, indent: bytes, key: Key | bool, value: Text) -> None:
        value.normalize(self._scratch)
        if value and not value[-1]:
            self._key(indent, key, b"|2+")
            self._utf8(indent, b"  ", value[:-1])
        else:
            self._key(indent, key, b"|2-")
            self._utf8(indent, b"  ", value)

    def _list(self, indent: bytes, key: Key | bool, items: List) -> None:
        if not items:
            self._key(indent, key, b"[]")
        else:
            assert key is not False
            self._key(indent, key, b"")
            indent = indent + b" "
            self._comment(indent, b"i:", items.comment_intro)
            for value in items:
                self._value(indent, True, value)

    def _dict(self, indent: bytes, key: Key | bool, entries: Dict|File) -> None:
        if not entries:
            self._key(indent, key, b"{}")
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
        self.write(indent)
        match key:
            case False:
                self.write(end)
            case True:
                self.write(b"-")
                if end:
                    self.write(b" ")
                    self.write(end)
            case Key():
                self.write(b'"')
                self.write(
                    key.replace("\\", r"\\")
                    .replace('"', r"\"")
                    .replace("\t", r"\t")
                    .encode()
                )
                self.write(b'":')
                if end:
                    self.write(b" ")
                    self.write(end)
            case _:
                raise ValueError(f"unexpected type: {type(key)}")
        self.write(b"\n")
