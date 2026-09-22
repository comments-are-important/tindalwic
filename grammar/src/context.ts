import * as terms from "./generated.terms.ts"
import { parser } from "./generated.ts"
import { ContextTracker, InputStream } from "@lezer/lr"

let output: Console | null = null
export function debugContext(console: Console | null) { output = console }

export class PeekTabs {
    readonly depth: number
    readonly tabs: number
    readonly epilog: boolean
    constructor(depth: number, tabs: number, epilog: boolean) {
        this.depth = depth
        this.tabs = tabs
        this.epilog = epilog
    }
    surfeit(): boolean {
        return this.tabs > this.depth
    }
    deficit(): boolean {
        return this.tabs < this.depth && !this.epilog
    }
    toString(): string {
        let epilog = (!this.epilog) ? ""
            : (this.tabs + 1 == this.depth) ? ",epilog" : ",ERROR"
        let status = (this.tabs == this.depth) ? "exact"
            : this.surfeit() ? "surfeit" : "deficit"
        return `PeekTabs{depth=${this.depth},tabs=${this.tabs}${epilog},${status}}`
    }
    hash(): number {
        let hash = 17 // emulate java.util.Objects.hash()
        hash = (31 * hash + this.depth) | 0
        hash = (31 * hash + this.tabs) | 0
        hash = (31 * hash + (this.epilog ? 1231 : 1237)) | 0
        return hash
    }
    peek(term:number, input: InputStream): PeekTabs {
        const LF = 10, TAB = 9, HASH = 35
        if (input.next == -1) {
            let result = this
            output?.debug(`peek=> at EOF, no change`)
            return result
        }
        let prev = input.peek(-1)
        if (prev != LF && prev != -1) {
            let result = zero_zero_false
            output?.debug(`peek=> ERROR prev=${prev} ${result}`)
            return result
        }
        if (this.depth < 1) {
            if (input.next == TAB) {
                let result = zero_one_false
                output?.debug(`peek=> outermost TAB ${result}`)
                return result
            }
            let result = zero_zero_false
            output?.debug(`peek=> outermost not TAB ${result}`)
            return result
        }
        let column = 0
        for (; column + 1 < this.depth; input.advance(), ++column)
            if (input.next != TAB) {
                if (column) input.advance(-column)
                let result = new PeekTabs(this.depth, column, false)
                output?.debug(`peek=> larger deficit ${result}`)
                return result
            }
        if (input.next == HASH) {
            if (column) input.advance(-column)
            let suppressed = term == terms.no_epi
            let result = new PeekTabs(this.depth, column, !suppressed)
            output?.debug(`peek=> ${suppressed?"suppressed ":""}epilog ${result}`)
            return result
        }
        if (input.next != TAB) {
            if (column) input.advance(-column)
            let result = new PeekTabs(this.depth, column, false)
            output?.debug(`peek=> minimal deficit ${result}`)
            return result
        }
        if (input.peek(1) == TAB) {
            if (column) input.advance(-column)
            // halting here makes it impossible to do .indent().indent()
            let result = new PeekTabs(this.depth, column + 1, false)
            output?.debug(`peek=> surfeit ${result}`)
            return result
        }
        if (column) input.advance(-column)
        let result = new PeekTabs(this.depth, column, false)
        output?.debug(`peek=> exact ${result}`)
        return result
    }
    indent(): PeekTabs {
        output?.assert(this.surfeit(),
            `indent: ERROR: !surfeit`)
        let result = new PeekTabs(this.depth + 1, this.tabs, false)
        output?.debug(`indent=> ${result}`)
        return result
    }
    dedent(): PeekTabs {
        output?.assert(this.deficit(),
            `dedent: ERROR: ${(this.depth < 1) ? "outermost" : "!deficit"}`)
        let result = new PeekTabs(this.depth ? this.depth - 1 : 0, this.tabs, false)
        output?.debug(`dedent=> ${result}`)
        return result
    }
    margin(): PeekTabs {
        output?.assert(this.tabs >= this.depth,
            `margin: ERROR: deficit or epilog`)
        let result = new PeekTabs(this.depth, 0, false)
        output?.debug(`margin=> ${result}`)
        return result
    }
    epi_mar(): PeekTabs {
        output?.assert(this.tabs + 1 == this.depth && this.epilog,
            `epi_mar: ERROR:${(this.tabs + 1 == this.depth) ? "" : " tabs"}${this.epilog ? "" : " !epilog"}`)
        let result = new PeekTabs(this.depth, 0, false)
        output?.debug(`epi_mar=> ${result}`)
        return result
    }
}

const zero_zero_false = new PeekTabs(0, 0, false)
const zero_one_false = new PeekTabs(0, 1, false)

export const peekTabs = new ContextTracker({
    start: zero_zero_false,
    strict: true,
    hash: (state: PeekTabs) => state.hash(),
    shift(state, term, stack, input) {
        output?.debug(`shift? ${parser.getName(term)} pos=${input.pos} state=${state}`)
        switch (term) {
            case terms.peek:
            case terms.no_epi:
                return state.peek(term, input)
            case terms.indent:
                return state.indent()
            case terms.dedent:
                return state.dedent()
            case terms.margin:
                return state.margin()
            case terms.epi_mar:
                return state.epi_mar()
        }
        output?.debug(`shift: no action`)
        return state
    },
})
