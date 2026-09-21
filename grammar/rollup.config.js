import { lezer } from "@lezer/generator/rollup"
import typescript from "@rollup/plugin-typescript"

export default {
    input: "./src/generated.ts",
    output: [{
        format: "es",
        file: "../target/tindalwic-lezer.js"
    }, {
        format: "cjs",
        file: "../target/tindalwic-lezer.cjs"
    }],
    external: ["@lezer/lr", "@lezer/highlight"],
    plugins: [
        lezer(),
        typescript({ declaration: false, sourceMap: false })
    ]
}
