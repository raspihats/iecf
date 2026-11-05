// Runs a C test built for wasm32-wasi; exits with its status.
// node runtime/tests/run-wasi.mjs test.wasm
import { readFile } from "node:fs/promises";
import { WASI } from "node:wasi";

const wasi = new WASI({ version: "preview1", args: [process.argv[2]], returnOnExit: true });
const { instance } = await WebAssembly.instantiate(await readFile(process.argv[2]), wasi.getImportObject());
const status = wasi.start(instance);
console.log(`${process.argv[2]}: ${status === 0 ? "ok" : `${status} check(s) failed`}`);
process.exit(status);
