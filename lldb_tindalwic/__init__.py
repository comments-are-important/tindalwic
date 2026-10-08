from lldb import SBValue, SBDebugger
from typing import Optional

try:
    # https://github.com/rust-lang/rust/blob/main/src/etc/
    import lldb_lookup
    rust = lambda: lldb_lookup
except:
    # this is normal - our script import usually happens first
    lldb_lookup = None
    def late_load_rust():
        import lldb_lookup
        global rust
        rust = lambda: lldb_lookup
        return lldb_lookup
    rust = lambda: late_load_rust()

def rust_type(val: SBValue) -> object:
    is_msvc = not val.GetFrame().GetModule().FindSection(".debug_info").IsValid()
    return rust().classify_rust_type(val.GetType(), is_msvc)
def helpful_info(msg: str, val: SBValue) -> str:
    return f"{msg}: {val.type.name};{val.GetTypeSummary()};{rust_type(val)};{val.IsSynthetic()}{[it.name for it in val.children]}"

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
def value_summary(obj: SBValue, _internal_but_required) -> str:
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
def comment_summary(obj: SBValue, _internal_but_required) -> str:
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
def name_summary(obj: SBValue, _internal_but_required) -> str:
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
def entry_summary(obj: SBValue, _internal_but_required) -> str:
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

NOTIFIED_TEXT = False
def text_summary(obj: SBValue, _internal_but_required) -> str:
    try:
        value = None
        epilog = None
        for child in obj.children:
            match child.name:
                case 'value': value = child
                case 'epilog': epilog = child
        value = value.num_children
        epilog = epilog.summary
        return f"Text[{value}]\u2AFD{epilog}"
    except Exception as message:
        global NOTIFIED_TEXT
        if not NOTIFIED_TEXT:
            print(f"need to fix text_summary: {message}")
            NOTIFIED_TEXT = True
        raise

NOTIFIED_DICT = False
def dict_summary(obj: SBValue, _internal_but_required) -> str:
    try:
        prolog = None
        entries = None
        epilog = None
        for child in obj.children:
            match child.name:
                case 'prolog': prolog = child
                case 'entries': entries = child
                case 'epilog': epilog = child
        prolog = prolog.summary
        entries = entries.num_children
        epilog = epilog.summary
        return f"Dict[{entries}]\u2AFD{prolog}\u2AFD{epilog}"
    except Exception as message:
        global NOTIFIED_DICT
        if not NOTIFIED_DICT:
            print(f"need to fix dict_summary: {message}")
            NOTIFIED_DICT = True
        raise

NOTIFIED_LIST = False
def list_summary(obj: SBValue, _internal_but_required) -> str:
    try:
        prolog = None
        items = None
        epilog = None
        for child in obj.children:
            match child.name:
                case 'prolog': prolog = child
                case 'items': items = child
                case 'epilog': epilog = child
        prolog = prolog.summary
        items = items.num_children
        epilog = epilog.summary
        return f"List[{items}]\u2AFD{prolog}\u2AFD{epilog}"
    except Exception as message:
        global NOTIFIED_LIST
        if not NOTIFIED_LIST:
            print(f"need to fix list_summary: {message}")
            NOTIFIED_LIST = True
        raise

class DelegatingProvider:
    # dynamic import prevents directly subclassing rust providers, so...
    def __init__(self, obj, internal):
        self.obj = obj
        self.internal = internal
        self.delegate = None
    def constructor(self):
        raise NotImplementedError(f"not implemented: {type(self)}.constructor")
    def synthetic(self, val: SBValue) -> SBValue:
        raise NotImplementedError(f"not implemented: {type(self)}.synthetic")
    def update(self):
        try:
            self.delegate = self.constructor()(self.obj, self.internal)
            self.delegate.update()
        except Exception as message:
            print(f"DelegatingProvider.update: {message}")
    def has_children(self) -> bool:
        try:
            return self.delegate.has_children()
        except Exception as message:
            print(f"DelegatingProvider.has_children: {message}")
    def num_children(self) -> int:
        try:
            return self.delegate.num_children()
        except Exception as message:
            print(f"DelegatingProvider.num_children: {message}")
    def get_child_index(self, name: str) -> int:
        try:
            return self.delegate.get_child_index(name)
        except Exception as message:
            print(f"DelegatingProvider.get_child_index: {message}")
    def get_child_at_index(self, index: int) -> Optional[SBValue]:
        try:
            child = self.delegate.get_child_at_index(index)
            return None if child is None else self.synthetic(child)
        except Exception as message:
            print(f"DelegatingProvider.get_child_at_index: {message}")

class EntrySynthetic(DelegatingProvider):
    def constructor(self):
        return rust().StructSyntheticProvider
    def synthetic(self, val: SBValue) -> SBValue:
        if val.name != 'item': return val
        enum = rust().ClangEncodedEnumProvider(val, self.internal)
        # their __init__ calls self.update(), contrary to docs, but in case they change:
        if not all(hasattr(enum, it) for it in enum.__slots__): enum.update()
        if enum.num_children() != 1:
            return val
        zero = enum.get_child_at_index(0)
        return None if zero is None else zero.Clone('item')

class EntriesSynthetic(DelegatingProvider):
    def constructor(self):
        return rust().StdSliceSyntheticProvider
    def synthetic(self, val: SBValue) -> SBValue:
        # every child in this slice is a Cell we want to elide
        return val.children[0].Clone(val.name) # Cell.value field

def __lldb_init_module(debugger: SBDebugger, _internal_but_required):

    target = debugger.GetDummyTarget()
    if target.IsValid():
        ptr_size = target.GetAddressByteSize()
        if ptr_size > 0:
            global USIZE_MAX
            USIZE_MAX = (1 << (ptr_size * 8)) - 1

    debugger.HandleCommand(f'type summary add -F {__name__}.value_summary   tindalwic::value::Value')
    debugger.HandleCommand(f'type summary add -F {__name__}.comment_summary tindalwic::Comment')
    debugger.HandleCommand(f'type summary add -F {__name__}.name_summary    tindalwic::Name')
    debugger.HandleCommand(f'type summary add -F {__name__}.entry_summary   tindalwic::Entry')
    debugger.HandleCommand(f'type summary add -F {__name__}.text_summary   tindalwic::Text')
    debugger.HandleCommand(f'type summary add -F {__name__}.dict_summary   tindalwic::Dict')
    debugger.HandleCommand(f'type summary add -F {__name__}.list_summary   tindalwic::List')
    debugger.HandleCommand(f'type synthetic add -l {__name__}.EntrySynthetic tindalwic::Entry')
    debugger.HandleCommand(f'type synthetic add -l {__name__}.EntriesSynthetic &[core::cell::Cell<tindalwic::Entry>]')
