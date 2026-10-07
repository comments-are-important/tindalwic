import lldb

# space is premium so very terse summary helps (can expand to see full detail).
#  - if a summary function throws, then the default summary is used (nice) silently.
#  - see some examples use lldb.formatters.Logger, but `print()` actually shows up.
#  - SBValue is a bit too forgiving: ask for child with unknown name, get a SBValue.

USIZE_MAX = 0xFFFFFFFFFFFFFFFF # 64-bit is probably correct

PICTURES = str.maketrans(
    "\u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007\u0008\u0009\u000A\u000B\u000C\u000D\u000E\u000F"
    +"\u0010\u0011\u0012\u0013\u0014\u0015\u0016\u0017\u0018\u0019\u001A\u001B\u001C\u001D\u001E\u001F"
    +"\u00F7",
    "\u2400\u2401\u2402\u2403\u2404\u2405\u2406\u2407\u2408\u2409\u240A\u240B\u240C\u240D\u240E\u240F"
    +"\u2410\u2411\u2412\u2413\u2414\u2415\u2416\u2417\u2418\u2419\u241A\u241B\u241C\u241D\u241E\u241F"
    +"\u2421",
)

NOTIFIED_VALUE = False
def value_summary(obj: lldb.SBValue, _internal_but_required) -> str:
    try:
        slice = None
        indent = None
        for child in obj.children:
            match child.name:
                case 'slice': slice = child
                case 'indent': indent = child
        slice = slice.summary
        indent = indent.unsigned
        if indent == USIZE_MAX: indent = 0
        slice = slice.replace("\n"+indent*"\t", "\u2424").translate(PICTURES)
        if indent == USIZE_MAX or indent == 0: indent = ""
        elif indent <= 20: indent = chr(0x2473 + indent)
        return f"{indent}{slice}"
    except Exception as message:
        global NOTIFIED_VALUE
        if not NOTIFIED_VALUE:
            print(f"need to fix value_summary: {message}")
            NOTIFIED_VALUE = True
        raise

NOTIFIED_COMMENT = False
def comment_summary(obj: lldb.SBValue, _internal_but_required) -> str:
    try:
        gap = None
        value = None
        for child in obj.children:
            match child.name:
                case 'gap': gap = child
                case 'value': value = child
        gap = gap.unsigned
        value = value.summary
        if gap == 0:
            return value
        if gap == 1:
            return "\u2424" if value == "None" else f"\u2424{value[5:-1]}"
        if gap <= 20: gap = chr(0x245F + gap)
        return f"{gap}\u2424" if value == "None" else f"{gap}\u2424{value[5:-1]}"
    except Exception as message:
        global NOTIFIED_COMMENT
        if not NOTIFIED_COMMENT:
            print(f"need to fix comment_summary: {message}")
            NOTIFIED_COMMENT = True
        raise

NOTIFIED_NAME = False
def name_summary(obj: lldb.SBValue, _internal_but_required) -> str:
    try:
        comment = None
        key = None
        for child in obj.children:
            match child.name:
                case 'comment': comment = child
                case 'key': key = child
        comment = comment.summary
        key = key.summary
        if comment == "None":
            return key
        return f"{key}\u2AFB{comment}"
    except Exception as message:
        global NOTIFIED_NAME
        if not NOTIFIED_NAME:
            print(f"need to fix name_summary: {message}")
            NOTIFIED_NAME = True
        raise

NOTIFIED_ENTRY = False
def entry_summary(obj: lldb.SBValue, _internal_but_required) -> str:
    try:
        name = None
        item = None
        for child in obj.children:
            match child.name:
                case 'name': name = child
                case 'item': item = child
        name = name.summary
        item = item.summary
        return f"{name}\u27F6{item}"
    except Exception as message:
        global NOTIFIED_ENTRY
        if not NOTIFIED_ENTRY:
            print(f"need to fix entry_summary: {message}")
            NOTIFIED_ENTRY = True
        raise

class CellArray:
    # https://github.com/rust-lang/rust/blob/main/src/etc/lldb_providers.py
    #  + L1188 StdSliceSyntheticProvider L1631 StdCellSyntheticProvider
    # mashup to elide the Cell.value hop: clone the value with the cell name.
    def __init__(self, obj: lldb.SBValue, _internal_but_required):
        self.obj = obj
        self.data_ptr = None
        self.length = 0
    def update(self):
        self.data_ptr = self.obj.children[0]
        self.length = self.obj.children[1].unsigned
    def num_children(self) -> int:
        return self.length
    def get_child_at_index(self, index: int) -> lldb.SBValue | None:
        kind = self.data_ptr.type.GetPointeeType()
        address = self.data_ptr.unsigned + index * kind.size
        cell = self.data_ptr.CreateValueFromAddress("[%s]" % index, address, kind)
        return cell.children[0].Clone(cell.name)

class EnumAllOne:
    # TODO elide the enum .0 hop: clone the value with the variant name.
    def __init__(self, obj: lldb.SBValue, _internal_but_required):
        self.obj = obj
        self.zero = None
    def update(self):
        self.zero = self.obj.children[0]
    def num_children(self) -> int:
        return 1
    def get_child_at_index(self, index: int) -> lldb.SBValue | None:
        return self.zero.children[0].Clone(self.zero.type.name)


def __lldb_init_module(debugger: lldb.SBDebugger, _internal_but_required):

    target = debugger.GetDummyTarget()
    if target.IsValid():
        ptr_size = target.GetAddressByteSize()
        if ptr_size > 0:
            global USIZE_MAX
            USIZE_MAX = (1 << (ptr_size * 8)) - 1

    debugger.HandleCommand(f'type filter add tindalwic::value::parse::Input --child line --child current --child empties')
    debugger.HandleCommand(f'type summary add -F {__name__}.value_summary   tindalwic::value::Value')
    debugger.HandleCommand(f'type summary add -F {__name__}.comment_summary tindalwic::Comment')
    debugger.HandleCommand(f'type summary add -F {__name__}.name_summary    tindalwic::Name')
    debugger.HandleCommand(f'type summary add -F {__name__}.entry_summary   tindalwic::Entry')
    debugger.HandleCommand(f'type synthetic add -l {__name__}.CellArray &[core::cell::Cell<tindalwic::Entry>] &[core::cell::Cell<tindalwic::Item>]')
    #debugger.HandleCommand(f'type synthetic add -l {__name__}.EnumAllOne tindalwic::Item')
