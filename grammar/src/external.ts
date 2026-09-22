import * as terms from "./generated.terms.ts"
import { parser } from "./generated.ts"
import { PeekTabs } from "./context.ts"
import { ExternalTokenizer, InputStream, Stack } from "@lezer/lr"

let output: Console | null = null
export function debugExternal(console: Console | null) { output = console }

class LeftEdge {
    static TERMS = [
        terms.peek, terms.no_epi, terms.indent, terms.dedent, terms.epi_mar, terms.margin
    ]
    readonly name: string | number
    readonly len: number
    readonly canShift: boolean
    readonly allowed: boolean
    constructor(term: number, input: InputStream, stack: Stack, noisy?: boolean) {
        this.name = parser.getName(term) || term
        this.len = 0
        this.canShift = stack.canShift(term)
        this.allowed = false
        let state: PeekTabs = stack.context
        switch (term) {
            case terms.peek:
            case terms.no_epi:
                this.allowed = true
                break
            case terms.indent:
                if (!state.surfeit()) {
                    if (output && noisy)
                        output.debug(`edge: no surfeit so deny indent ${state}`)
                    break
                }
                this.allowed = true
                break
            case terms.dedent:
                if (!state.deficit()) {
                    if (output && noisy)
                        output.debug(`edge: no deficit so deny dedent ${state}`)
                    break
                }
                this.allowed = true
                break
            case terms.epi_mar:
                if (!state.epilog) {
                    if (output && noisy)
                        output.debug(`edge: !epilog so deny epi_mar ${state}`)
                    break
                }
                this.allowed = true
                this.len = state.tabs
                break
            case terms.margin:
                if (state.deficit()) {
                    if (output && noisy)
                        output.debug(`edge: deficit so deny margin ${state}`)
                    break
                }
                if (input.pos < 1 && input.next == 35 && input.peek(1) == 33) {
                    if (output && noisy)
                        output.debug(`edge: at shebang so deny margin ${state}`)
                    break
                }
                if (state.depth < 1 && (input.next == 10 || input.next == -1)) {
                    if (output && noisy)
                        output.debug(`edge: outermost EOL so deny margin ${state}`)
                    break
                }
                this.allowed = true
                this.len = state.tabs
                break
        }
    }
    debug(): string {
        return `${this.allowed ? "+" : "!"}${this.name}`
    }
}
function canShiftReport(input: InputStream, stack: Stack): string {
    let canShift = []
    for (const term of Object.values(terms)) {
        if (stack.canShift(term))
            if (LeftEdge.TERMS.includes(term))
                canShift.push(new LeftEdge(term, input, stack).debug())
            else
                canShift.push(parser.getName(term) || term)
    }
    return `pos=${input.pos} state=${stack.context} canShift=${canShift}`
}

export const leftEdge = new ExternalTokenizer((input, stack) => {
    output?.debug(`edge? ${canShiftReport(input, stack)}`)
    for (const term of LeftEdge.TERMS) {
        if (!stack.canShift(term)) continue
        let check = new LeftEdge(term, input, stack)
        if (!check.allowed) continue
        output?.debug(`edge: accept ${check.debug()} pos=${input.pos} len=${check.len}`)
        input.acceptToken(term, check.len)
        return
    }
    output?.debug(`edge: nothing accepted`)
})
