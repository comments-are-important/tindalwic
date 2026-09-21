import { ContextTracker } from "@lezer/lr"
import * as terms from "./generated.terms.ts"
import * as level from "./indentation.ts"
import { output, parser } from "./debug.ts"

const EOF = -1

export const margin = new ContextTracker({
    start: new level.File() as level.Indentation,
    hash: context => context.hash,
    shift(context, term, stack, input) {
        let move = undefined

        if (term === terms.dedent)
            move = context.parent ?? context

        else if (term === terms.indentT)
            move = new level.Text(context)

        else if (term === terms.indentD)
            move = new level.Dict(context)

        else if (term === terms.indentL)
            move = new level.List(context)

        if (input.peek(stack.pos - input.pos) === EOF)
            move = (move ?? context).seal()

        if (output) {
            let name = parser?.getName(term)
            const chars = []
            for (let pos = input.pos; pos < stack.pos; ++pos)
                chars.push(String.fromCodePoint(input.peek(pos - input.pos)))
            let token = JSON.stringify(chars.join(''))
            if (token != name) name += ` ${token}`
            let result = (move === undefined) ? "" : `->${move}`
            output?.debug(`shift ${name} ${context}${result}`)
        }

        return move ?? context
    }
})
