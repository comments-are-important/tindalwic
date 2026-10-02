import { styleTags, tags } from "@lezer/highlight"

export const highlight = styleTags({
    '"#!" "//" "///" "@" "="': tags.processingInstruction,
    '"<" "<>" ">"': tags.angleBracket,
    '"{" "{}" "}"': tags.brace,
    '"[" "[]" "]"': tags.squareBracket,
    "Shebang/Line": tags.documentMeta,
    "Interpreter/Line": tags.monospace,
    "GFM/Line": tags.comment, // fallback in case !parseMixed
    "Text/Line": tags.string,
    "Key/Line": tags.namespace,
})
const markdownHighlighting = styleTags({
    "Blockquote/...": tags.quote,
    HorizontalRule: tags.contentSeparator,
    "ATXHeading1/... SetextHeading1/...": tags.heading1,
    "ATXHeading2/... SetextHeading2/...": tags.heading2,
    "ATXHeading3/...": tags.heading3,
    "ATXHeading4/...": tags.heading4,
    "ATXHeading5/...": tags.heading5,
    "ATXHeading6/...": tags.heading6,
    "Comment CommentBlock": tags.comment,
    Escape: tags.escape,
    Entity: tags.character,
    "Emphasis/...": tags.emphasis,
    "StrongEmphasis/...": tags.strong,
    "Link/... Image/...": tags.link,
    "OrderedList/... BulletList/...": tags.list,
    "BlockQuote/...": tags.quote,
    "InlineCode CodeText": tags.monospace,
    "URL Autolink": tags.url,
    "HeaderMark HardBreak QuoteMark ListMark LinkMark EmphasisMark CodeMark": tags.processingInstruction,
    "CodeInfo LinkLabel": tags.labelName,
    LinkTitle: tags.string,
    Paragraph: tags.content
})
