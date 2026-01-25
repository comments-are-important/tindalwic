from io import StringIO


class Indent:
    __slots__ = ("_bytes", "_more", "_less", "key")

    def __init__(self, value: bytes):
        if value.count(b"\t") != len(value):
            raise AssertionError("indent must be tab chars only")
        self._bytes = value
        self._more: Indent | None = None
        self._less: Indent | None = None
        self.key: str | int | None = None

    def more(self) -> "Indent":
        result = self._more
        if result is None:
            result = self._more = Indent(self._bytes + b"\t")
            result._less = self
        else:
            result.key = None
        return result

    def less(self) -> "Indent":
        result = self._less
        if result is None:
            raise AssertionError("indent can't go negative")
        return result

    def zero(self) -> "Indent":
        result = self
        while result:
            result = result.less()
        indent = result
        while indent is not None:
            indent.key = None
            indent = indent._more
        return result

    def __len__(self) -> int:
        return len(self._bytes)

    def __repr__(self) -> str:
        return f"<Indent {len(self)} @{self.path().getvalue()}>"

    def path(self, into: StringIO | None = None) -> StringIO:
        if self._less is not None:
            into = self._less.path(into)
        elif self.key is None:
            # often the zeroth key is None and the File key is in 1st indent...
            return into or StringIO()
        elif into is None:
            into = StringIO()
        into.write("/")
        match self.key:
            case str(key):
                into.write(key)
            case key if key is ...:
                into.write("~...") # for testing purposes
            case key:
                into.write(str(key).replace("~","~0").replace("/","~1"))
        return into
