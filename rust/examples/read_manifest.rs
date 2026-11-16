//! Print the manifest embedded in a delegate WASM module, and optionally check
//! it. Used by CI to prove the section survives a real release build:
//!
//! ```text
//! cargo run --example read_manifest -- <module.wasm> '<expected JSON>'
//! ```
//!
//! Exits non-zero if the module has no manifest, or if it differs from the
//! expected JSON.

use freenet_stdlib::prelude::DelegateManifest;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: read_manifest <module.wasm> [expected JSON]");
    let module = std::fs::read(&path).expect("read module");
    let manifest = match DelegateManifest::from_wasm(&module) {
        Ok(Some(m)) => m,
        Ok(None) => {
            eprintln!("{path}: no manifest section");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("{path}: {e}");
            std::process::exit(1);
        }
    };
    let json = String::from_utf8(manifest.to_bytes()).expect("manifest JSON is UTF-8");
    println!("{json}");
    if let Some(expected) = args.next() {
        let expected = DelegateManifest::from_bytes(expected.as_bytes()).expect("expected JSON");
        if manifest != expected {
            eprintln!("{path}: manifest differs from the expected one");
            std::process::exit(1);
        }
    }
}
