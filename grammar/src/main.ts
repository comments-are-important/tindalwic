import { parser } from "./generated.ts"
import { debugContext } from "./context.ts"
import { debugExternal } from "./external.ts"
import { readFileSync } from "node:fs"
import { env } from "node:process"

let input = (process.argv.length > 2) ? process.argv[2] : readFileSync(0, "utf8")

let verbose = (env.LOG == "parse")

if (verbose) {
    debugContext(console)
    debugExternal(console)
    console.log(`input.length=${input.length}`)
    console.log(input)
    console.log("=======")
}
let tree = parser.parse(input)
if (verbose)
    console.log("=======")
console.log(tree.toString())

