process.env.LOG='parse' // must `await import` so @lezer/lr notices
const { parser } = await import("./generated.ts")
const { debugContext } = await import("./context.ts")
const { debugExternal } = await import("./external.ts")

import { readFileSync } from "node:fs"
let input = (process.argv.length > 2) ? process.argv[2] : ""
if (input == "-") input = readFileSync(0, "utf8")

debugContext(console)
debugExternal(console)

console.log(`input.length=${input.length}`)
console.log(input)
console.log("=======")
let tree = parser.parse(input)
console.log("=======")
console.log(tree.toString())
