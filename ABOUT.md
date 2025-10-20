Strict Encoding
===============

```ini
[*.alacs]
charset = utf-8
end_of_line = lf
indent_style = tab
insert_final_newline = false
max_line_length = off
trim_trailing_whitespace = false
```

ALACS data is required to conform to these [.editorconfig](.editorconfig) settings.
Other formats accommodate various encodings and use of whitespace, but that flexibility
comes at a cost - sometimes subtle, like the impact on content-addressable storage. In
the past it was arguably worth the price to open up more platforms, but today it seems
not to be worth it.


Lexical Analysis is Trivial
===========================

 + Line oriented.
 + String values **only**, no other primitive types.
 + No quotation/escaping mechanism needed.

ALACS reads data one line at a time and examines typically a few chars at the start. Any
chars that follow the prefix, all the way to the end of the line, are used verbatim.
Formats like YAML and TOML acknowledge the burden of quotes in JSON, allowing them to be
omitted in special cases. ALACS takes that idea to the extreme: no quotes at all.

Something like [Pydantic](https://pydantic.dev/) can be used to validate and convert the
strings to appropriate types. There is no reason to duplicate that functionality here.


Indentation Demarks Nested Contexts
===================================

 + Similar to Python, YAML, Scala... except:
 + Tab chars **only**, never spaces.

If a line indicates the opening of a nested context, then all the lines within it will
immediately follow and will be indented by exactly one more tab char than the opening
line. Every context is closed by EOF or by the next line with insufficient indentation.

The four kinds of context are described next. The outermost (zero indentation, closed
by EOF) context is always that of an associative array. This idea borrows from TOML, but
ALACS rejects the complexity of their non-contiguous contexts.


Comment and Text Contexts
=========================

 + Opened when the prefix of a line contains these markers:
   + **`#`** for a comment,
   + **`|`** or **`>`** for text.
 + These contexts are **terminal** (impossible to open any other context inside it).
 + Remainder of the opening line (if not empty) becomes the first line.
 + Each subsequent line in the context (stripped of indentation) adds one more line.

ALACS takes inspiration for these marker chars from YAML, but further specifies: the
text inside `#` and `>` contexts is (CommonMark)[https://commonmark.org/]. Extensions
that are exactly the same in both Github and GitLab flavors may be used. ALACS itself
does not parse any text, this is just verbatim UTF-8 - but software that reads this data
can know how to treat it.

Comments always have exactly one subject of discussion, but that is never another
comment (no meta allowed). A comment for a text element always immediately follows and
closes the text context - the two opening lines will line up vertically, and any
indented text lines will line up at one tab stop more. Comments for the other contexts
will be discussed as those contexts are introdced.


Linear Array
============

 + Opened when the prefix of a line contains the marker: **`[]`**
 + Each value has its own line (which may open a more deeply nested subcontext).
 + Can have two comments (both optional) about the array itself:
   + An introduction at the very top, indented.
   + Following (and closing, like text element comments) the array context.
 + Nulls are not supported.
 + Each line that does not mark a subcontext is a simple string literal.
   + This means a line that has only indentation is an empty string.


Associative Array
=================

 + Opened when the prefix of a line contains the marker: **`:`**
 + Keys must pass Python `str.isidentifier()` predicate.
   + A comment about the key may preceed it.
 + The marker for the value follows the Key (possibly opening a subcontext).
   + The **`=`** marker means the remainder of the line is a simple string literal.
 + Same two comment locations as for linear array.

blank lines

<!-- from "lexical analysis" section: talk about the 3 special values that look like
they are quoted, but are just special prefixes recognized only in this context -->


key`=`value _or_ key` = `value
------------------------------

value must not have any line break chars (including, obviously, newline).

    url = https://example.com?query=recent#fragment


key`>` _or_ key` >`
-------------------

opens a multiline string context (one more indent). can be 0 or 1 line, too.

    pangram >
    	the quick brown fox
    	jumps over the lazy dog.
    empty >
    one >
    	single line is fine but `=` form is probably better

key`:` _or_ key` :`
-------------------

opens a nested associative array context.

    book :
    	title = Hop On Pop
    	author = Seuss

key`[]` _or_ key` []`
---------------------

opens a nested linear array context.

    weekend []
    	Saturday
    	Sunday


Rough Spots
===========

 + No inline linear array? A sequence of short values looks kinda icky.
 + Comments can't be tacked on to the end of some other line.

Please submit ideas for how to implement these. 

