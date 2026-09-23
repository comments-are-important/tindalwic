import * as terms from "./generated.terms.ts"
import { parser } from "./generated.ts"
import { PeekTabs } from "./context.ts"
import { ExternalTokenizer } from "@lezer/lr"
import { EOF, LF, BANG, HASH } from "./ascii.ts"

let output: Console | null = null
function unexpectedTermName(term: number, expected: string): boolean {
    let got = parser.getName(term)
    output?.assert(expected == got, `mismatch: our ${expected} != parser ${got}`)
    return expected != got
}
export function debugExternal(console: Console | null) {
    output = console
    if (output != null)
        if (unexpectedTermName(terms.peek, "peek")
            || unexpectedTermName(terms.indent, "indent")
            || unexpectedTermName(terms.dedent, "dedent")
            || unexpectedTermName(terms.epilog, "epilog")
            || unexpectedTermName(terms.margin, "margin")
            || unexpectedTermName(terms.short_text, "short_text"))
            output.error(`probably need to regenerate`)
}

export const leftEdge = new ExternalTokenizer((input, stack) => {
    let state: PeekTabs = stack.context
    if (output) {
        let can = []
        for (const term of Object.values(terms))
            if (stack.canShift(term)) {
                let prefix = ""
                switch (term) {
                    case terms.peek:
                    case terms.indent:
                    case terms.dedent:
                    case terms.epilog:
                    case terms.margin:
                    case terms.short_text:
                        prefix = state.canShift(term) ? "+" : "!"
                }
                can.push(`${prefix}${parser.getName(term) || term}`)
            }
        output.debug(`edge? pos=${input.pos} state=${stack.context} canShift=${can}`)
    }
    if (stack.canShift(terms.peek) && state.canShift(terms.peek)) {
        output?.debug(`edge: accept peek epsilon`)
        input.acceptToken(terms.peek, 0)
        return
    }
    if (stack.canShift(terms.indent) && state.canShift(terms.indent)) {
        output?.debug(`edge: accept indent epsilon`)
        input.acceptToken(terms.indent, 0)
        return
    }
    if (stack.canShift(terms.dedent) && state.canShift(terms.dedent)) {
        output?.debug(`edge: accept dedent epsilon`)
        input.acceptToken(terms.dedent, 0)
        return
    }
    if (stack.canShift(terms.epilog) && state.canShift(terms.epilog)) {
        let offset = state.depth - 1
        if (offset < 0) {
            output?.debug(`edge: accept epilog preventing UNDERFLOW`)
            input.acceptToken(terms.epilog)
        } else {
            output?.debug(`edge: accept epilog len=${offset}`)
            input.acceptToken(terms.epilog, offset)
        }
        return
    }
    if (stack.canShift(terms.short_text) && state.canShift(terms.short_text)) {
        output?.debug(`edge: accept short_text len=${state.depth}`)
        input.acceptToken(terms.short_text, state.depth)
        return
    }
    if (stack.canShift(terms.margin) && state.canShift(terms.margin)) {
        if (input.pos < 1 && input.next == HASH && input.peek(1) == BANG) {
            output?.debug(`edge: at shebang so deny margin ${state}`)
        } else if (state.depth < 1 && (input.next == LF || input.next == EOF)) {
            output?.debug(`edge: outermost EOL so deny margin ${state}`)
        } else {
            output?.debug(`edge: accept margin len=${state.depth}`)
            input.acceptToken(terms.margin, state.depth)
            return
        }
    }
    output?.debug(`edge: nothing accepted`)
})
