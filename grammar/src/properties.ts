import { styleTags, tags } from "@lezer/highlight"

export const highlight = styleTags({
    '# "#!" "//" @': tags.keyword,
    "< <> >": tags.angleBracket,
    "{ {} }": tags.brace,
    "[ [] ]": tags.squareBracket,
    "Shebang/Line": tags.documentMeta,
    "Prolog/Line Epilog/Line Comment/Line": tags.comment,
    "Text/Line": tags.string,
    "Key/Line": tags.namespace,
})
