import { parser } from "./generated.ts"
import { setDebug } from "./support.ts"
import { readFileSync } from "node:fs"

setDebug(console)
let input = readFileSync(0, "utf8")
let tree = parser.parse(input)
console.log(tree.toString())
