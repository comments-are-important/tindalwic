import { styleTags, tags } from "@lezer/highlight"

export const highlight = styleTags({
    '"#!" @': tags.keyword,
    "< <> >": tags.angleBracket,
    "{ {} }": tags.brace,
    "[ [] ]": tags.squareBracket,
    "Shebang!": tags.documentMeta,
    "Prolog! Epilog! Comment!": tags.blockComment, // lineComment if only one Line?
    "Line": tags.string,
    "KeyText! KeyDict! KeyList! KeyLong! KeyEquals!": tags.namespace,
})
