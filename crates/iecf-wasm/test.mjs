// The module as a browser gets it: node test.mjs path/to/iecf_wasm.wasm
import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import { load } from "./iecf.js";

const iecf = await load(readFile(process.argv[2]));
assert.match(iecf.version, /^\d+\.\d+\.\d+$/);

const main = { name: "main.st", text: "PROGRAM main\nVAR x : INT; END_VAR\nx := g; (* ünïcode *)\nEND_PROGRAM\n" };
const gvl = { name: "gvl.st", text: "VAR_GLOBAL g : INT; END_VAR\n" };

const strict = iecf.check([main, gvl]);
assert.equal(strict.length, 1);
assert.deepEqual(
  { code: strict[0].code, file: strict[0].file, line: strict[0].line, column: strict[0].column },
  { code: "E0010", file: "main.st", line: 3, column: 6 },
);
assert.deepEqual(iecf.check([main, gvl], { extensions: ["implicit-externals"] }), []);
assert.throws(() => iecf.check([], { extensions: ["nope"] }), /no extension 'nope'/);

// A large file, to grow the module's memory between calls.
const big = { name: "big.st", text: main.text.replace("x := g;", "x := g;\n".repeat(20000)) };
assert.equal(iecf.check([big, gvl]).length, 20000);
assert.deepEqual(iecf.check([main, gvl], { extensions: ["implicit-externals"] }), []);

console.log(`iecf ${iecf.version}: the wasm module checks as the command line does`);
