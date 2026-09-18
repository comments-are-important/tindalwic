/// <reference types="tree-sitter-cli/dsl" />
// @ts-check
export default grammar({

    name: "tindalwic", // text in nested dictionaries and lists with important comments

    // the official Rust parser should be used if possible. this parser exists:
    //   + to provide syntax highlighting for the various editors to use.
    //   + as an educational aid for people comfortable reading this kind of grammar.
    //     the Rust file is 600 lines, and the format syntax is somewhat obscured by
    //     concerns (like zero-copy). this file is 100 lines and works at a higher level
    //     of abstraction (the tricky details are out of the way in the scanner code).
    // biggest difference from the Rust is the shape of the tree, particularly how
    // $.entry flattens the epilog and the Name fields into a single node. it's a minor
    // annoyance that is acceptable given the intended purposes of this grammar.

    rules: {

        // outermost context is a dictionary after an optional `#!` but without an epilog:
        file: $ => seq(optional($.shebang), optional($.prolog), repeat($.entry)),

        // the three data types (one text primitive and two nested array contexts):
        text: $ => seq($._INDENT, optional($._text_block), $._DEDENT, optional($.epilog)),
        dict: $ => seq($._INDENT, optional($.prolog), repeat($.entry), $._DEDENT, optional($.epilog)),
        list: $ => seq($._INDENT, optional($.prolog), repeat($._item), $._DEDENT, optional($.epilog)),

        // comments are text except 1st line is before the block, flowing into it:
        _text_flow: $ => seq($.line, $._INDENT, repeat($._another_line), $._DEDENT),

        // the three data types have distinctive (hopefully familiar) markers
        _value: $ => choice(
            seq($._left_margin, '<>', $.text),
            seq($._left_margin, '{}', $.dict),
            seq($._left_margin, '[]', $.list),
        ),

        // text is a contiguous block of equally indented lines:
        line: $ => /[^\n]*/,
        _text_block: $ => seq($._first_line, repeat($._another_line)),

        // comments are fully nodes in the parse and each has a topic it is about:
        shebang: $ => seq($._left_margin, '#!', $._text_flow), // file
        prolog: $ => seq($._left_margin, '#', $._text_flow),   // file dict list
        epilog: $ => seq($._left_margin, '#', $._text_flow),   // value
        comment: $ => seq($._left_margin, '//', $._text_flow), // key

        // lists hold a linear array of data values:
        _item: $ => choice(
            $._value, // the three distinctive markers indicate the value type
            // in a list context a single line text value can skip the marker...
            seq($._left_margin, alias($.short_text, $.text))
        ),
        short_text: $ => alias($._SHORT_STR, $.line),

        // dicts hold an associative array of key+value pairs:
        entry: $ => seq(optional($.gap), optional($.comment), $._key_value),
        key: $ => alias($._SHORT_KEY, $.line),
        _key_value: $ => choice(
            seq($._left_margin, '@', alias($.text, $.key), $._value),
            seq($._left_margin, '<', alias($.text_key, $.key), '>', $.text),
            seq($._left_margin, '{', alias($.dict_key, $.key), '}', $.dict),
            seq($._left_margin, '[', alias($.list_key, $.key), ']', $.list),
            seq($._left_margin, $.key, '=', alias($.short_line, $.text)),
        ),
        text_key: $ => alias($._TEXT_KEY, $.line),
        dict_key: $ => alias($._DICT_KEY, $.line),
        list_key: $ => alias($._LIST_KEY, $.line),
        short_line: $ => $.line,

        // some rules need to peek ahead a few chars
        gap: $ => seq($._PEEK_EMPTY, $._NEW_LINE),
        _left_margin: $ => seq($._PEEK_MARGIN, $._NEW_LINE, $._MARGIN),
        _first_line: $ => seq($._PEEK_MARGIN, /\n/, $._MARGIN, $.line),
        _another_line: $ => seq($._PEEK_MARGIN, '\n', $._MARGIN, $.line),
    },

    externals: $ => [
        // most tokens are mutually exclusive: grammar rules must
        // never ask for more than one from any single scanner call.
        $._NEW_LINE,    // LF or zero-width beginning of file
        $._MARGIN,      // the expected number of TABs starting at column 0
        $._SHORT_STR,   // empty or /[^#/@=<>{}\[\]\n\t][^\n]*/
        $._SHORT_KEY,   // empty or /[^#/@=<>{}\[\]\n\t][^=\n]*/ if peek('=')
        $._TEXT_KEY,    // rest of line if peek('>', EOF or LF)
        $._DICT_KEY,    // rest of line if peek('}', EOF or LF)
        $._LIST_KEY,    // rest of line if peek(']', EOF or LF)
        $._INDENT,      // ++margin zero-width
        // remaining tokens help the rules determine the structure: scanner
        // will be asked to select from among more than one of them.
        // until issue 5929 gets done all these must be zero-width
        $._DEDENT,      // --margin if EOF or peek(LF, not enough TABs)
        $._PEEK_EMPTY,  // if peek(NEW_LINE, EOF or LF)
        $._PEEK_MARGIN, // if peek(NEW_LINE, margin TABS)
    ],

    conflicts: $ => [[$.text], [$.dict], [$.list]],

    extras: $ => [
        // empty to disable the builtin ignore whitespace stuff.
        // note "insert_final_newline = false" in `.editorconfig`: newlines here aren't
        // typical line termination chars. tindalwic does not use quotation for strings,
        // so a trailing (CR)LF at EOF can't be ignored. instead it uses things like
        // RegExp `/[^\n]*/` to finish lines without consuming any termination chars.
        // think of _NEW_LINE as: "nope, not done yet, here's another line to parse".
    ],

});
