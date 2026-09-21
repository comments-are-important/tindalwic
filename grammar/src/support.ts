import {

    start, // epsilon, triggers peeking
    eol, // epsilon at EOF or a LF char, triggers peeking
    line, // zero or more (greedy) non-LF chars + `eol`
    indent, // epsilon iff state.surfeit(), triggers ++depth
    dedent, // epsilon iff state.deficit(), triggers --depth
    margin, // the TAB chars iff state.exact(), triggers tabs=0
    epitabs // the TAB chars iff state.epilog, triggers tabs=0; epilog=false

} from "./generated.terms.ts"
import { ContextTracker, ExternalTokenizer, InputStream } from "@lezer/lr"

let debug: object | null = null
export function setDebug(console: object | null) { debug = console }

class State {
    parent: State | null
    depth: number
    tabs: number
    epilog: boolean
    hash: number
    constructor(parent: State | null, tabs: number, epilog: any) {
        this.parent = parent
        this.depth = !parent ? 0 : 1 + parent.depth
        this.tabs = tabs
        this.epilog = epilog
        let hash = 17 // emulate java.util.Objects.hash()
        hash = (31 * hash + (parent ? parent.hash : 0)) | 0
        hash = (31 * hash + this.depth) | 0
        hash = (31 * hash + tabs) | 0
        hash = (31 * hash + (epilog ? 1231 : 1237)) | 0
        this.hash = hash
    }
    peek(input: InputStream) {
        let prev = input.peek(-1)
        if (prev != LF && prev != -1) {
            debug?.error?.("not at column 0", prev)
            return this
        }
        let column = 0
        for (; column + 1 < this.depth; input.advance(), ++column)
            if (input.next != TAB) {
                input.advance(-column)
                debug?.log?.("not TAB", column, this.depth)
                return new State(this.parent, column, false)
            }
        if (input.next == HASH) {
            input.advance(-column)
            debug?.log?.("epilog", column)
            return new State(this.parent, column, true)
        }
        if (input.next != TAB) {
            input.advance(-column)
            debug?.log?.("not TAB", column, this.depth)
            return new State(this.parent, column, false)
        }
        let peek = (input.peek(1) == TAB) ? 1 : 0
        input.advance(-column)
        debug?.log?.(peek ? "TAB" : "not TAB", column + 1, this.depth)
        return new State(this.parent, column + peek, false)
    }
    surfeit() {
        return this.tabs > this.depth && !this.epilog
    }
    indent() {
        if (!this.surfeit()) debug?.error?.("!surfeit")
        return new State(this, this.tabs, false)
    }
    deficit() {
        return this.tabs < this.depth && !this.epilog
    }
    dedent() {
        if (!this.deficit()) debug?.error?.("!deficit")
        if (this.parent) return this.parent
        debug?.error?.("at outermost context")
        return new State(null, this.tabs, false)
    }
    exact() {
        return this.tabs == this.depth && !this.epilog
    }
    margin() {
        if (!this.exact()) debug?.error?.("!exact")
        return new State(this.parent, 0, false)
    }
    epitabs() {
        if (!this.epilog) debug?.error?.("!epilog")
        if (this.tabs + 1 != this.depth) debug?.error?.("epilog incorrect")
        return new State(this.parent, 0, false)
    }
}

const LF = 10, TAB = 9, HASH = 35

export const state = new ContextTracker({
    strict: true,
    hash: (state: State | null) => state ? state.hash : 0,
    start: null,
    shift(state, term, stack, input) {
        if (!state) state = new State(null, 0, false)
        if (term == start) return state.peek(input)
        if (!stack.context) debug?.warn?.("1st shift != start", term)
        if (term == eol) return state.peek(input)
        if (term == line) return state.peek(input)
        if (term == indent) return state.indent()
        if (term == dedent) return state.dedent()
        if (term == margin) return state.margin()
        if (term == epitabs) return state.epitabs()
        return state
    },
})

export const chars = new ExternalTokenizer((input, stack) => {
    let accept = -1, offset = 0, state: State | null = stack.context
    if (input.next == -1 || !state) {
        if (stack.canShift(start))
            accept = start
        else if (stack.canShift(eol))
            accept = eol
        else
            debug?.warn?.("!state nothing to accept?")
    } else if (input.next == LF) {
        if (stack.canShift(eol)) {
            accept = eol
            offset = 1
        }
        else if (stack.canShift(line)) {
            accept = line
            offset = 1
        }
        else
            debug?.warn?.("LF nothing to accept?")
    } else if (stack.canShift(indent) && state.surfeit())
        accept = indent
    else if (stack.canShift(dedent) && state.deficit())
        accept = dedent
    else if (stack.canShift(margin) && state.exact()) {
        accept = margin
        offset = state.tabs
    }
    else if (stack.canShift(epitabs) && state.epilog) {
        accept = epitabs
        offset = state.tabs
    }
    else
        debug?.warn?.("tried everything nothing to accept?")
    debug?.log?.("acceptToken(", accept, offset, ")", input.pos, "state", state?.depth, state?.tabs, state?.epilog)
    if (accept != -1) {
        input.acceptToken(accept, offset)
    }
})
