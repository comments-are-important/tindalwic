import { ExternalTokenizer } from "@lezer/lr"
import * as terms from "./generated.terms.ts"
import * as level from "./indentation.ts"
import { output, parser } from "./debug.ts"

const EOF = -1, TAB = 9, LF = 10, HASH = 35, SLASH = 47
const EQ = 61, A_KET = 62, S_KET = 93, C_KET = 125
const RESERVED = [HASH, SLASH, EQ, 64/*@*/, 60, A_KET, 91, S_KET, 12, C_KET]

const tokenizer = new ExternalTokenizer((input, stack) => {
    const context = stack.context
    const depth = context.depth
    const previous = input.peek(-1)
    const edge = previous === LF || previous === EOF
    scan: {
        // some `break scan` are intentional dead code acting as documentation
        // doing way more than most tokenizers, but I guess the grammar is atypical
        // could push some of this up into the grammar, and arguably should,
        // but doing so makes it harder to read and thus much less pedagogical

        if (edge && depth !== 0 && input.next !== TAB) {
            if (stack.canShift(terms.dedent))
                input.acceptTokenTo(terms.dedent, stack.pos) // zero-width
            break scan
        }

        if (edge && depth !== 0 && input.next === TAB) {
            for (input.advance(); input.pos < stack.pos + depth; input.advance())
                if (input.next !== TAB) {
                    if (stack.canShift(terms.dedent))
                        input.acceptTokenTo(terms.dedent, stack.pos) // zero-width
                    break scan
                }
            if (stack.canShift(terms.margin))
                input.acceptToken(terms.margin)
            // next call can indent if TAB follows margin
            break scan
        }

        if (input.next === TAB) {
            if (stack.canShift(terms.indentT)) {
                input.acceptToken(terms.indentT, 1)
                break scan
            }
            if (stack.canShift(terms.indentD)) {
                input.acceptToken(terms.indentD, 1)
                break scan
            }
            if (stack.canShift(terms.indentL)) {
                input.acceptToken(terms.indentL, 1)
                break scan
            }
            // could also be Line...
        }

        if (!edge && !context.sealed && stack.canShift(terms.Line)) {
            // TODO consider allowing SLASH as long as it isn't SLASH SLASH
            // makes a lot of sense here, less sense in KeyEquals but maybe there too
            if (context instanceof level.List && RESERVED.includes(input.next)) {
                output?.debug(`reject: List Line[0] ${String.fromCharCode(input.next)}`)
                break scan
            }
            for (; ; input.advance())
                if (input.next === LF) {
                    input.acceptToken(terms.Line, 1)
                    break scan
                } else if (input.next === EOF) {
                    input.acceptToken(terms.Line)
                    break scan
                }
            break scan
        }

        if ((edge && depth !== 0 || input.next === EOF) && stack.canShift(terms.dedent)) {
            input.acceptToken(terms.dedent)
            break scan
        }

        if (input.next === HASH && stack.canShift(terms.Hash)) {
            input.acceptToken(terms.Hash, 1)
            break scan
        }
        if (input.next === SLASH && stack.canShift(terms.Slashes)
            && input.peek(1) === SLASH) {
            input.acceptToken(terms.Slashes, 2)
            break scan
        }
        if (edge && (stack.canShift(terms.Hash) || stack.canShift(terms.Slashes))) {
            while (input.next === LF)
                input.advance()
            if (input.next === EOF) {
                if (stack.canShift(terms.eof))
                    input.acceptToken(terms.eof)
                break scan
            }
            let need = depth
            for (; need !== 0; --need, input.advance())
                if (input.next !== TAB) break
            if (need === 0) {
                if (input.next === HASH && stack.canShift(terms.Hash)) {
                    input.acceptToken(terms.Hash, 1)
                    break scan
                }
                if (input.next === SLASH && stack.canShift(terms.Slashes)
                    && input.peek(1) === SLASH) {
                    input.acceptToken(terms.Slashes, 2)
                    break scan
                }
            }
            input.advance(stack.pos - input.pos)
            output?.debug(`failed gap+comment rewinding ${stack.pos - input.pos}`)
        }

        if (stack.canShift(terms.KeyText)) {
            for (let prev = false; ; input.advance())
                if (input.next === LF || input.next === EOF) {
                    input.acceptToken(terms.KeyText, prev ? -1 : 0)
                    break scan
                } else prev = (input.next === A_KET)
            break scan
        }

        if (stack.canShift(terms.KeyDict)) {
            for (let prev = false; ; input.advance())
                if (input.next === LF || input.next === EOF) {
                    input.acceptToken(terms.KeyDict, prev ? -1 : 0)
                    break scan
                } else prev = (input.next === C_KET)
            break scan
        }

        if (stack.canShift(terms.KeyList)) {
            for (let prev = false; ; input.advance())
                if (input.next === LF || input.next === EOF) {
                    input.acceptToken(terms.KeyList, prev ? -1 : 0)
                    break scan
                } else prev = (input.next === S_KET)
            break scan
        }

        if (stack.canShift(terms.eol)) {
            if (input.next === LF)
                input.acceptToken(terms.eol, 1)
            else if (input.next === EOF)
                // grammar wants an eol before the eof
                input.acceptToken(terms.eol)
            // could also be eof...
        }

        if (stack.canShift(terms.eof)) {
            while (input.next === LF)
                input.advance()
            if (input.next === EOF) {
                input.acceptToken(terms.eof)
                break scan
            }
            input.advance(stack.pos - input.pos)
            output?.debug(`failed eof rewinding ${stack.pos - input.pos}`)
        }

        if (stack.canShift(terms.KeyEquals)) {
            if (input.next !== EQ && RESERVED.includes(input.next)) {
                output?.debug(`reject: KeyEquals[0] ${String.fromCharCode(input.next)}`)
                break scan
            }
            for (; input.next !== EOF; input.advance())
                if (input.next === EQ) {
                    input.acceptToken(terms.KeyEquals, 1)
                    break scan
                }
            break scan
        }

    } // end scan
    if (output) {
        const term = (input as any)?.token?.value
        const name = (term >= 0) ? parser?.getName(term) : edge ? "|none" : "none"
        let end = (input as any)?.token?.end
        if (typeof end !== "number") end = undefined
        const stop = (end === undefined) ? 0 : end - input.pos
        const chars = []
        for (let offset = stack.pos - input.pos; offset < stop; ++offset)
            chars.push(String.fromCodePoint(input.peek(offset)))
        let token = (chars.length === 0) ? "" : JSON.stringify(chars.join(''))
        if (token.length > 0)
            token += (end !== undefined) ? " " : "~ "
        const others = []
        for (let other = 0; ; ++other) {
            if (term === other) continue
            const name = parser?.getName(other)
            if (name === undefined) break
            if (stack.canShift(other))
                others.push(name)
        }
        let not = others.length ? `could: ${others.join(" ")}` : ''
        output.debug(`accept ${name}.${context} ${token}${not}`)
    }
})

export { tokenizer as terms }
