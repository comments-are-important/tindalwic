from io import BytesIO
from . import File, Dict, List, Text, Comment, Value, Encoded

__all__ = ["YAML"]


class YAML(BytesIO):
    """Produces YAML that is not particularly aesthetically pleasing.

    This class prioritizes simple code that preserves all the input. No attempt is made
    to make the output look nice. A load+dump cycle using `ruamel.yaml` round-trip will
    clean things up a bit without losing the comments."""

    def __init__(self):
        self._scratch = list[Encoded]()
        self._blank = Comment()

    def encode(self, alacs: File) -> BytesIO:
        self.seek(0)
        self.truncate()
        self._comment(b"", b"!", alacs.hashbang)
        self._dict(b"", alacs)
        self.seek(0)
        return self

    def _utf8(self, indent: bytes, prefix: bytes, alacs: list[Encoded]) -> None:
        for line in alacs:
            self.write(indent)
            self.write(prefix)
            self.write(line)
            self.write(b"\n")

    def _comment(self, indent: bytes, prefix: bytes, alacs: Comment | None) -> None:
        if alacs is not None:
            alacs.normalize(self._scratch)
            mark = b"#" if prefix == b'!' else b"#%d" % len(indent)
            self._utf8(mark, prefix, alacs)

    def _value(self, indent: bytes, alacs: Value) -> None:
        match alacs:
            case Text():
                self._text(indent, alacs)
            case List():
                self._list(indent, alacs)
            case Dict():
                self._dict(indent, alacs)
            case _:
                raise ValueError(f"unexpected type: {type(alacs)}")
        self._comment(indent, b"a:", alacs.comment_after)

    def _text(self, indent: bytes, value: Text) -> None:
        value.normalize(self._scratch)
        if value and not value[-1]:
            self.write(b" |2+\n")
            self._utf8(indent, b" ", value[:-1])
        else:
            self.write(b" |2-\n")
            self._utf8(indent, b" ", value)

    def _list(self, indent: bytes, alacs: List) -> None:
        if not alacs:
            self.write(indent)
            self.write(b" []")
        self.write(b"\n")
        self._comment(indent, b"i:", alacs.comment_intro)
        if alacs:
            more = indent + b" "
            for value in alacs:
                self.write(indent)
                self.write(b"-")
                self._value(more, value)

    def _dict(self, indent: bytes, alacs: Dict | File) -> None:
        if not alacs:
            self.write(indent)
            self.write(b" {}\n")
        elif not isinstance(alacs, File):
            self.write(b"\n")
        self._comment(indent, b"i:", alacs.comment_intro)
        if alacs:
            more = indent + b" "
            for key, value in alacs.items():
                if key.blank_line_before:
                    self._comment(indent, b"b", self._blank)
                self._comment(indent, b"k:", key.comment_before)
                self.write(indent)
                self.write(b'"')
                key = key.replace("\\", r"\\").replace('"', r"\"").replace("\t", r"\t")
                self.write(key.encode())
                self.write(b'":')
                self._value(more, value)
