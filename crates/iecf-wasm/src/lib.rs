//! iecf's checker as a WebAssembly module with no JavaScript glue
//! generated for it and no dependency: a handful of exported functions
//! that pass bytes through one buffer in the module's memory. `iecf.js`
//! next to this crate wraps them in one call:
//!
//! ```js
//! const iecf = await load(fetch("iecf.wasm"));
//! const diagnostics = iecf.check([{ name: "main.st", text }], { extensions: [] });
//! ```
//!
//! The protocol, for another host:
//!
//! 1. `iecf_buffer(n)` makes the input buffer `n` bytes long and returns
//!    where it is; the host writes UTF-8 there.
//! 2. `iecf_add_file(name_len, text_len)`: the buffer holds a file's name
//!    then its text. Returns 0, or 1 if either is not UTF-8.
//! 3. `iecf_extension(name_len)`: the buffer holds an extension's name
//!    (docs/extensions.md). Returns 0, or 1 if there is no such extension.
//! 4. `iecf_check()` checks the files added since the last check, forgets
//!    them and the extensions, and returns the length of its result: the
//!    JSON array `iecf check --format json` prints, at `iecf_result()`.
//! 5. `iecf_version()` puts the version in the result, for the host to
//!    check it loaded the iecf its build server runs (§2a, point 2).
//!
//! The host reads and writes the module's memory only between calls,
//! never during one, so the buffers are never in use when it does.

use iecf_driver::{Extension, Options};
use std::cell::RefCell;

#[derive(Default)]
struct State {
    input: Vec<u8>,
    files: Vec<(String, String)>,
    options: Options,
    result: Vec<u8>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// Makes the input buffer `len` bytes long and returns where it starts.
pub fn buffer(len: usize) -> *mut u8 {
    STATE.with_borrow_mut(|s| {
        s.input.clear();
        s.input.resize(len, 0);
        s.input.as_mut_ptr()
    })
}

/// Adds the file whose name and text the input buffer holds.
pub fn add_file(name_len: usize, text_len: usize) -> u32 {
    STATE.with_borrow_mut(|s| {
        let Some(bytes) = s.input.get(..name_len + text_len) else { return 1 };
        let (name, text) = bytes.split_at(name_len);
        match (std::str::from_utf8(name), std::str::from_utf8(text)) {
            (Ok(name), Ok(text)) => {
                s.files.push((name.to_string(), text.to_string()));
                0
            }
            _ => 1,
        }
    })
}

/// Switches on the extension the input buffer names.
pub fn extension(name_len: usize) -> u32 {
    STATE.with_borrow_mut(|s| {
        let name = s.input.get(..name_len).and_then(|b| std::str::from_utf8(b).ok());
        match name.and_then(Extension::from_name) {
            Some(e) => {
                s.options = std::mem::take(&mut s.options).with(e);
                0
            }
            None => 1,
        }
    })
}

/// Checks the files added so far; the result is the diagnostics as JSON.
pub fn check() -> usize {
    STATE.with_borrow_mut(|s| {
        let files = std::mem::take(&mut s.files);
        let options = std::mem::take(&mut s.options);
        s.result = iecf_driver::check(files, &options).to_json().into_bytes();
        s.result.len()
    })
}

pub fn version() -> usize {
    STATE.with_borrow_mut(|s| {
        s.result = env!("CARGO_PKG_VERSION").as_bytes().to_vec();
        s.result.len()
    })
}

pub fn result() -> *const u8 {
    STATE.with_borrow(|s| s.result.as_ptr())
}

/// The exports, under the names the host calls. wasm32 only: natively
/// the functions above are the API, and the tests call them.
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code, reason = "no_mangle exports a fixed name; no pointer is dereferenced")]
mod exports {
    #[unsafe(no_mangle)]
    pub extern "C" fn iecf_buffer(len: u32) -> *mut u8 {
        super::buffer(len as usize)
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn iecf_add_file(name_len: u32, text_len: u32) -> u32 {
        super::add_file(name_len as usize, text_len as usize)
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn iecf_extension(name_len: u32) -> u32 {
        super::extension(name_len as usize)
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn iecf_check() -> u32 {
        super::check() as u32
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn iecf_version() -> u32 {
        super::version() as u32
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn iecf_result() -> *const u8 {
        super::result()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the host does, without the host: write into the buffer
    /// through `buffer`'s slice, as the host writes into memory.
    fn put(bytes: &[u8]) {
        buffer(bytes.len());
        STATE.with_borrow_mut(|s| s.input.copy_from_slice(bytes));
    }

    fn result_text() -> String {
        STATE.with_borrow(|s| String::from_utf8(s.result.clone()).unwrap())
    }

    #[test]
    fn the_protocol() {
        let (name, text) = ("main.st", "PROGRAM main\nVAR x : INT; END_VAR\nx := g;\nEND_PROGRAM\n");
        put(format!("{name}{text}").as_bytes());
        assert_eq!(add_file(name.len(), text.len()), 0);
        put(b"gvl.stVAR_GLOBAL g : INT; END_VAR\n");
        assert_eq!(add_file(6, 28), 0);
        let len = check();
        assert_eq!(result_text().len(), len);
        assert!(result_text().contains("\"code\":\"E0010\""), "{}", result_text());
        // The files and options are forgotten: a second check starts anew.
        put(format!("{name}{text}").as_bytes());
        assert_eq!(add_file(name.len(), text.len()), 0);
        put(b"gvl.stVAR_GLOBAL g : INT; END_VAR\n");
        assert_eq!(add_file(6, 28), 0);
        put(b"implicit-externals");
        assert_eq!(extension(18), 0);
        check();
        assert_eq!(result_text(), "[]");
        put(b"nope");
        assert_eq!(extension(4), 1);
        put(&[b'a', 0xff]);
        assert_eq!(add_file(1, 1), 1, "not UTF-8");
        assert_eq!(add_file(5, 5), 1, "longer than the buffer");
        version();
        assert_eq!(result_text(), env!("CARGO_PKG_VERSION"));
    }
}
