import * as terms from "./generated.terms.ts"
import { parser } from "./generated.ts"
import { ContextTracker, InputStream } from "@lezer/lr"
import { EOF, TAB, LF, HASH, show, reserved } from "./ascii.ts"

let output: Console | null = null
export function debugContext(console: Console | null) { output = console }

export class Margin {
    readonly depth: number
    readonly tabs: number
    readonly next: number
    constructor(depth: number, tabs: number, next: number) {
        this.depth = depth
        this.tabs = tabs
        this.next = next
    }
    toString(): string {
        return `Margin%${this.depth}:${this.tabs}*TAB+${show(this.next)}`
    }
    surfeit(): boolean {
        return this.tabs > this.depth
    }
    deficit(): boolean {
        return this.tabs < this.depth
    }
    notEpilog(): boolean {
        return this.next != HASH || this.tabs != this.depth - 1
    }
    canShift(term: number): boolean {
        switch (term) {
            case terms.peek:
            case terms.indent:
                return true
            case terms.dedent:
                return this.deficit()
            case terms.epilog:
                return !this.notEpilog()
            case terms.margin:
                return !this.deficit()
            case terms.weird:
                return this.tabs == this.depth && !reserved(this.next)
        }
        return false;
    }
    hash(): number {
        let hash = 17 // emulate java.util.Objects.hash()
        hash = (31 * hash + this.depth) | 0
        hash = (31 * hash + this.tabs) | 0
        hash = (31 * hash + this.next) | 0
        return hash
    }
    peek(input: InputStream): Margin {
        if (input.next == EOF) {
            let result = (this.tabs === 0 && this.next === EOF) ? this
                : new Margin(this.depth, 0, EOF)
            output?.debug(`peek=> EOF ${(this === result) ? "keep" : "new"} ${result}`)
            return result
        }
        let prev = input.peek(-1)
        if (prev != LF && prev != EOF)
            output?.warn(`peek: not at column 0? prev=${show(prev)}`)
        let tabs = 0
        for (; input.next == TAB; input.advance())
            ++tabs
        let next = input.next
        if (tabs) input.advance(-tabs)
        let result = (this.tabs === tabs && this.next === next) ? this
            : new Margin(this.depth, tabs, next)
        output?.debug(`peek=> ${(this === result) ? "keep" : "new"} ${result}`)
        return result
    }
    indent(): Margin {
        let result = new Margin(this.depth + 1, this.tabs, this.next)
        output?.debug(`indent=> ${this.surfeit() ? "actual" : "virtual"} ${result}`)
        return result
    }
    dedent(): Margin {
        if (this.depth > 0) {
            let result = new Margin(this.depth - 1, this.tabs, this.next)
            output?.debug(`dedent=> ${result}`)
            return result
        }
        output?.error(`dedent=> UNDERFLOW prevented ${this}`)
        return this
    }
    consume(tabs: number): Margin {
        if (tabs < this.tabs)
            return new Margin(this.depth, this.tabs - tabs, TAB)
        if (tabs == this.tabs)
            return new Margin(this.depth, 0, this.next)
        output?.error(`consume tabs UNDERFLOW prevented`)
        return new Margin(this.depth, 0, EOF)
    }
    epilog(): Margin {
        output?.assert(!this.notEpilog(),
            `epilog: ERROR wrong ${(this.next != HASH) ? "next" : "tabs"}`)
        let result = this.consume(this.depth - 1)
        output?.debug(`epilog=> ${result}`)
        return result
    }
    margin(): Margin {
        output?.assert(!this.deficit(),
            `margin: ERROR: deficit`)
        let result = this.consume(this.depth)
        output?.debug(`margin=> ${result}`)
        return result
    }
}

export const margin = new ContextTracker({
    start: new Margin(0, 0, EOF),
    strict: true,
    hash: (state: Margin) => state.hash(),
    shift(state, term, _stack, input) {
        output?.debug(`shift? ${parser.getName(term)} pos=${input.pos} state=${state}`)
        switch (term) {
            case terms.peek:
                return state.peek(input)
            case terms.indent:
                return state.peek(input).indent()
            case terms.dedent:
                return state.dedent()
            case terms.epilog:
                return state.epilog()
            case terms.margin:
            case terms.weird:
                return state.margin()
        }
        output?.debug(`shift: no action`)
        return state
    },
})
