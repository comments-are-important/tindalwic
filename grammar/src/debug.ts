import type { LRParser } from "@lezer/lr"

export let parser: LRParser | undefined = undefined
export let output: Console | undefined = undefined

export function disable() {
    parser = undefined
    output = undefined
}

export function enable(lrParser: LRParser, console: Console) {
    parser = lrParser
    output = console
}
