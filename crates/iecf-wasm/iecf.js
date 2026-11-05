// iecf's checker in the browser, or in Node: one ES module, no
// dependencies, around iecf.wasm (built from crates/iecf-wasm).
//
//   import { load } from "./iecf.js";
//   const iecf = await load(fetch("iecf.wasm"));   // or the bytes
//   iecf.version;                                  // "0.0.1"
//   iecf.check([{ name: "main.st", text }], { extensions: ["implicit-externals"] });
//   // -> [{ code, severity, message, file, start, end, line, column, end_line, end_column }]
//
// The diagnostics are those `iecf check --format json` prints for the
// same files. `start`/`end` are byte offsets in the UTF-8 text; `line`
// and `column` are 1-based, the column counting characters.

const encoder = new TextEncoder();
const decoder = new TextDecoder();

export async function load(source) {
  const resolved = await source;
  const { instance } =
    typeof Response !== "undefined" && resolved instanceof Response
      ? await WebAssembly.instantiateStreaming(resolved, {})
      : await WebAssembly.instantiate(resolved, {});
  const x = instance.exports;

  // Copies bytes into the module's input buffer. The module may grow its
  // memory in the call, so the view is taken after it.
  const put = (bytes) => {
    const at = x.iecf_buffer(bytes.length);
    new Uint8Array(x.memory.buffer, at, bytes.length).set(bytes);
  };
  const result = (len) => decoder.decode(new Uint8Array(x.memory.buffer, x.iecf_result(), len));

  return {
    version: result(x.iecf_version()),

    check(files, { extensions = [] } = {}) {
      for (const { name, text } of files) {
        const n = encoder.encode(name);
        const t = encoder.encode(text);
        const both = new Uint8Array(n.length + t.length);
        both.set(n);
        both.set(t, n.length);
        put(both);
        if (x.iecf_add_file(n.length, t.length) !== 0) throw new Error(`iecf: ${name} is not UTF-8`);
      }
      for (const e of extensions) {
        const n = encoder.encode(e);
        put(n);
        if (x.iecf_extension(n.length) !== 0) throw new Error(`iecf: no extension '${e}'`);
      }
      return JSON.parse(result(x.iecf_check()));
    },
  };
}
