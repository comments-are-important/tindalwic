import * as terms from "./generated.terms.ts"
import { parser } from "./generated.ts"
import { ContextTracker, InputStream } from "@lezer/lr"
import { displayChar, reserved, EOF, TAB, LF, HASH } from "./ascii.ts"

let output: Console | null = null
export function debugContext(console: Console | null) { output = console }

export class PeekTabs {
    readonly depth: number
    readonly tabs: number
    readonly next: number
    constructor(depth: number, tabs: number, next: number) {
        this.depth = depth
        this.tabs = tabs
        this.next = next
    }
    toString(): string {
        return `PeekTabs%${this.depth}:${this.tabs}*TAB+${displayChar(this.next)}`
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
            case terms.short_text:
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
    peek(input: InputStream): PeekTabs {
        if (input.next == EOF) {
            let result = (this.tabs === 0 && this.next === EOF) ? this
                : new PeekTabs(this.depth, 0, EOF)
            output?.debug(`peek=> EOF ${(this === result) ? "keep" : "new"} ${result}`)
            return result
        }
        let prev = input.peek(-1)
        if (prev != LF && prev != EOF)
            output?.warn(`peek: not at column 0? prev=${displayChar(prev)}`)
        let tabs = 0
        for (; input.next == TAB; input.advance())
            ++tabs
        let next = input.next
        if (tabs) input.advance(-tabs)
        let result = (this.tabs === tabs && this.next === next) ? this
            : new PeekTabs(this.depth, tabs, next)
        output?.debug(`peek=> ${(this === result) ? "keep" : "new"} ${result}`)
        return result
    }
    indent(): PeekTabs {
        let result = new PeekTabs(this.depth + 1, this.tabs, this.next)
        output?.debug(`indent=> ${this.surfeit() ? "actual" : "virtual"} ${result}`)
        return result
    }
    dedent(): PeekTabs {
        if (this.depth > 0) {
            let result = new PeekTabs(this.depth - 1, this.tabs, this.next)
            output?.debug(`dedent=> ${result}`)
            return result
        }
        output?.error(`dedent=> UNDERFLOW prevented ${this}`)
        return this
    }
    consume(tabs: number): PeekTabs {
        if (tabs < this.tabs)
            return new PeekTabs(this.depth, this.tabs - tabs, TAB)
        if (tabs == this.tabs)
            return new PeekTabs(this.depth, 0, this.next)
        output?.error(`consume tabs UNDERFLOW prevented`)
        return new PeekTabs(this.depth, 0, EOF)
    }
    epilog(): PeekTabs {
        output?.assert(!this.notEpilog(),
            `epilog: ERROR wrong ${(this.next != HASH) ? "next" : "tabs"}`)
        let result = this.consume(this.depth - 1)
        output?.debug(`epilog=> ${result}`)
        return result
    }
    margin(): PeekTabs {
        output?.assert(!this.deficit(),
            `margin: ERROR: deficit`)
        let result = this.consume(this.depth)
        output?.debug(`margin=> ${result}`)
        return result
    }
}

export const peekTabs = new ContextTracker({
    start: new PeekTabs(0, 0, EOF),
    strict: true,
    hash: (state: PeekTabs) => state.hash(),
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
            case terms.short_text:
                return state.margin()
        }
        output?.debug(`shift: no action`)
        return state
    },
})
