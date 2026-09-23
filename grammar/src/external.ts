import { ExternalTokenizer, InputStream } from "@lezer/lr"
import { Margin } from "./context.ts"
import { parser } from "./generated.ts"
import * as terms from "./generated.terms.ts"
import * as chars from "./ascii.ts"

let output: Console | null = null
export function debugExternal(console: Console | null) { output = console }

class Key {
    readonly term: number
    readonly prev: number
    readonly halt: number
    readonly peek_eol: boolean
    constructor(term: number, prev: number, halt: number, eol: boolean) {
        this.term = term
        this.prev = prev
        this.halt = halt
        this.peek_eol = eol
    }
    goodStart(input: InputStream): boolean {
        let have = input.peek(-1)
        if (have === this.prev)
            return true
        if (this.prev === chars.EOF)
            // BOF will already have returned, that's fine
            if (have === chars.TAB || have === chars.LF)
                return true
        return false
    }
    badStart(input: InputStream): string {
        let name = parser.getName(this.term)
        let want = (this.prev === chars.EOF) ? "margin" : chars.show(this.prev)
        let have = chars.show(input.peek(-1))
        return `!-PREV ${name} want=${want} have=${have}`
    }
    goodEnd(input: InputStream): boolean {
        if (this.halt !== input.next)
            return false
        if (this.peek_eol) {
            let have = input.peek(1)
            if (have !== chars.LF && have !== chars.EOF)
                return false
        }
        return true
    }
    badEnd(input: InputStream): string {
        let want = `${chars.show(this.halt)}${this.peek_eol ? "$" : ""}`
        let have = `${chars.show(input.next)}(${chars.show(input.peek(1))})`
        return `!-HALT want=${want} have=${have}`
    }
    acceptEnd(len: number, input: InputStream): string {
        let name = parser.getName(this.term)
        return `${name}[${len}]${this.goodEnd(input) ? "" : this.badEnd(input)}`
    }
}
let keys = [
    // these require peeking ahead, so they can't be native tokens:
    new Key(terms.angleKey, chars.BRA_A, chars.A_KET, true),
    new Key(terms.squareKey, chars.BRA_S, chars.S_KET, true),
    new Key(terms.curlyKey, chars.BRA_C, chars.C_KET, true),
    // this one could be native, but is here for symmetry and debugging:
    new Key(terms.equalsKey, chars.EOF, chars.EQ, false),
]

export const str = new ExternalTokenizer((input, stack) => {
    let at = input.pos, len = () => input.pos - at
    if (output) {
        let allowed = []
        for (const term of Object.values(terms))
            if (stack.canShift(term))
                allowed.push(`${parser.getName(term)}`)
        output?.assert(allowed.length < 2, `str@${at}? !-MULTIPLE ${allowed}`)
    }
    for (const key of keys)
        if (stack.canShift(key.term)) {
            output?.assert(key.goodStart(input), `str@${at}? ${key.badStart(input)}`)
            while (input.next !== key.halt
                && input.next !== chars.LF
                && input.next !== chars.EOF)
                input.advance()
            // accept any good chars we found even if the end was bad
            // the subsequent rule should fail, causing error recovery
            output?.debug(`str@${at}-> ${key.acceptEnd(len(), input)}`)
            input.acceptToken(key.term)
            return
        }
})

export const edge = new ExternalTokenizer((input, stack) => {
    let state: Margin = stack.context
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
                    case terms.weird:
                    case terms.Gap:
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
    if (state.canShift(terms.Gap) && state.canShift(terms.Gap)) {
        output?.debug(`edge: accept Gap`)
        input.acceptToken(terms.Gap, 1)
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
    if (stack.canShift(terms.weird) && state.canShift(terms.weird)) {
        output?.debug(`edge: accept weird len=${state.depth}`)
        input.acceptToken(terms.weird, state.depth)
        return
    }
    if (stack.canShift(terms.margin) && state.canShift(terms.margin)) {
        if (input.pos < 1 && input.next == chars.HASH && input.peek(1) == chars.BANG) {
            output?.debug(`edge: at shebang so deny margin ${state}`)
        } else if (state.depth < 1 && (input.next == chars.LF || input.next == chars.EOF)) {
            output?.debug(`edge: outermost EOL so deny margin ${state}`)
        } else {
            output?.debug(`edge: accept margin len=${state.depth}`)
            input.acceptToken(terms.margin, state.depth)
            return
        }
    }
    output?.debug(`edge: nothing accepted`)
})
