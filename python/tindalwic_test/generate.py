from random import randrange, choice, choices
from tindalwic import Comment, Text, List, Dict, Value, File, Key
from tindalwic.pointer import Indent

bools = (False, True)
ascii = b"\t" + bytes(it for it in range(32, 127))


class Random:
    "single thread only"

    def __init__(self, *, deepest=6, widest=8, empties=True) -> None:
        self.deepest = deepest
        self.widest = widest
        self.empties = empties
        self.key = ascii
        self.comment = ascii
        self.text = ascii
        self.indent = Indent(b"")

    def _comment(self, kind: str, indent: Indent) -> Comment:
        result = [f"{self.indent.path().getvalue()} {kind}".encode()]
        for loop in range(randrange(3)):
            result.append(bytes(choices(self.comment, k=randrange(80))))
        if len(result) == 1:
            return Comment(result[0], -1)
        else:
            sep = b"\n" + indent._bytes
            return Comment(sep.join(result), len(indent))

    def _text(self) -> Text:
        text = list[bytes]()
        for loop in range(randrange(3)):
            text.append(bytes(choices(self.text, k=randrange(80))))
        match len(text):
            case 0:
                return Text()
            case 1:
                return Text(text[0], -1)
            case _:
                sep = b"\n" + self.indent._bytes
                return Text(sep.join(text), len(self.indent))

    def _list(self, array: List) -> List:
        depth = len(self.indent)
        if depth < randrange(self.deepest):
            self.indent = self.indent.more()
            for key in range(randrange(self.widest)):
                self.indent.key = key
                array.append(self._value(depth))
            self.indent = self.indent.less()
        if not array and not self.empties:
            array.append(self._text())
        if choice(bools):
            array.comment_after = self._comment("after", self.indent)
        if choice(bools):
            array.comment_intro = self._comment("intro", self.indent.more())
        return array

    def _dict_entries(self, array: Dict | File) -> None:
        depth = len(self.indent)
        if depth < randrange(self.deepest):
            self.indent = self.indent.more()
            for loop in range(randrange(self.widest)):
                key = Key(bytes(choices(self.key, k=randrange(20))).decode())
                self.indent.key = key
                if choice(bools):
                    key.blank_line_before = True
                if choice(bools):
                    key.comment_before = self._comment("before", self.indent)
                array[key] = self._value(depth)
            self.indent = self.indent.less()
        if not array and not self.empties:
            key = Key(bytes(choices(self.key, k=randrange(20))).decode())
            array[key] = self._text()
        if choice(bools):
            array.comment_intro = self._comment("intro", self.indent.more())

    def _dict(self, array: Dict) -> Dict:
        self._dict_entries(array)
        if choice(bools):
            array.comment_after = self._comment("after", self.indent)
        return array

    def _value(self, depth: int) -> Value:
        match randrange(3):
            case 0:
                return self._dict(Dict())
            case 1:
                return self._list(List())
            case _:
                return self._text()

    def file(self) -> File:
        self.indent = self.indent.zero()
        file = File()
        if choice(bools):
            file.hashbang = self._comment("hashbang", self.indent)
        self._dict_entries(file)
        return file
