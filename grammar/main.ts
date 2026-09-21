import { parser } from "./src/generated.ts"
import * as debug from "./src/debug.ts"
import { readFileSync } from "node:fs"

process.stdout.on("error", () => process.exit(1))
process.stderr.on("error", () => process.exit(1))

const verbose = typeof process != "undefined" && process.env && /\bparse\b/.test(process.env.LOG!)
// that was copy-n-pasted from line 9 of lr parse.ts so it could (unlikely) become stale
if (verbose) debug.enable(parser, console)

let input = (process.argv.length < 3) ? readFileSync(0, "utf8") : process.argv[2]
if (verbose) {
    console.log(`input.length=${input.length}`)
    console.log("=======")
    console.log(input)
    console.log("=======")
}
let tree = parser.parse(input)
if (verbose)
    console.log("=======")
console.log(tree.toString())

