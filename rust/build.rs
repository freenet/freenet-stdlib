//! Build script for `freenet-stdlib`.
//!
//! The flatbuffers bindings in `src/generated/` are **checked in**, and an
//! ordinary build does not rebuild them. Regeneration happens only when
//! `FREENET_REGEN_FLATBUFFERS=1` is set, and only with the pinned `flatc`.
//!
//! It used to work the other way: every build shelled out to whatever `flatc`
//! happened to be on `PATH` and rewrote the bindings in place, silently doing
//! nothing at all when there was no `flatc` to find. Both halves caused real
//! problems.
//!
//! Different `flatc` releases format their output differently, so a contributor
//! whose compiler did not match the one the bindings were generated with got
//! thousands of lines of unrelated churn mixed into their diff, on every build,
//! whether or not they had touched a schema. That is not hypothetical: commit
//! 8070e93, "revert: undo accidental flatbuffers regen from #72", exists because
//! it reached `main`. Worse, the rewrite also happened inside the crates.io
//! registry checkout on *downstream* builds, where nobody would ever see it.
//!
//! The silent fallback is the more dangerous half. CI does not install `flatc`,
//! so CI never regenerated anything — it compiled the checked-in bindings and
//! reported success. A schema edit that was never regenerated therefore passed
//! CI while the bindings on disk still described the *old* schema, and nothing
//! anywhere compared the two.
//!
//! So: regeneration is explicit, and `ci.yml`'s `flatbuffers_drift` job is what
//! compares schemas to bindings, by regenerating with the pinned compiler and
//! failing if the result differs from what is committed. Note that job is only
//! advisory until it is added to branch protection — `main` currently requires
//! no status checks, so it reports drift rather than preventing it from merging.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The `flatc` release the checked-in bindings are generated with.
///
/// Matches the `flatbuffers` runtime crate this package resolves to, which is
/// what makes the pairing meaningful: upstream ships `flatc` and the Rust crate
/// from the same tag. Note `Cargo.toml` declares `flatbuffers = "24.3"`, a caret
/// range rather than an exact version, and `Cargo.lock` is not tracked — so the
/// runtime floats within 24.x and can drift away from this pin without anything
/// noticing. Nothing here detects that; the drift job compares generated code to
/// the *schemas*, never to the runtime.
const PINNED_FLATC: &str = "24.12.23";

/// Where the `.fbs` files live, relative to this package.
///
/// Outside the packaged crate on purpose — the schemas are shared with the
/// TypeScript SDK. That is why nothing below may reference this path during an
/// ordinary build; see `main`.
const SCHEMA_DIR: &str = "../schemas/flatbuffers";

fn main() {
    // Deliberately the ONLY directive emitted on the ordinary build path, and
    // deliberately not a `rerun-if-changed` on the schemas.
    //
    // `cargo package` ships only files under `rust/`, so in a registry checkout
    // `../schemas/` does not exist — and cargo treats a `rerun-if-changed` path
    // that does not exist as permanently dirty. Declaring the schemas here would
    // therefore rebuild `freenet-stdlib`, and everything downstream of it, on
    // every single `cargo build` in every consumer. Measured at ~1s per no-op
    // build for this crate alone, before its reverse-dependency cone.
    println!("cargo:rerun-if-env-changed=FREENET_REGEN_FLATBUFFERS");

    if std::env::var_os("FREENET_REGEN_FLATBUFFERS").is_none() {
        warn_if_schemas_look_newer();
        return;
    }

    let schemas = schema_files();

    // Safe here: regeneration is only ever requested from a source checkout, so
    // the directory exists and the always-dirty problem above does not apply.
    for schema in &schemas {
        println!("cargo:rerun-if-changed={}", schema.display());
    }

    // Everything below panics rather than warning. Setting the variable is an
    // explicit request to regenerate, so a run that quietly regenerates nothing
    // is the same silent no-op this script exists to remove -- and it is worse
    // here, because cargo caches a *successful* build-script run keyed on the
    // declared inputs. A warn-and-return would be recorded as success, so
    // installing the right compiler and re-running the documented command could
    // legitimately not execute this script again, leaving the bindings stale
    // while the developer believes they regenerated. Failing loudly is not
    // cached and cannot be missed.
    match installed_flatc_version() {
        Some(version) if version == PINNED_FLATC => {}
        Some(version) => panic!(
            "FREENET_REGEN_FLATBUFFERS is set but flatc {version} is installed; the bindings \
             are pinned to {PINNED_FLATC}. Regenerating with a different compiler rewrites \
             every generated file. Install flatc {PINNED_FLATC} \
             (https://github.com/google/flatbuffers/releases/tag/v{PINNED_FLATC})."
        ),
        None => panic!(
            "FREENET_REGEN_FLATBUFFERS is set but no flatc was found on PATH. Install flatc \
             {PINNED_FLATC} (https://github.com/google/flatbuffers/releases/tag/v{PINNED_FLATC})."
        ),
    }

    let mut cmd = Command::new("flatc");
    cmd.arg("--rust").arg("-o").arg("src/generated");
    for schema in &schemas {
        cmd.arg(schema);
    }
    match cmd.status() {
        Ok(status) if status.success() => {}
        Ok(status) => panic!("flatc exited with {status}"),
        Err(err) => panic!("failed to run flatc: {err}"),
    }

    // The checked-in bindings are formatted, so the regenerated ones must be too
    // or the drift check compares formatting rather than content.
    match Command::new("cargo").arg("fmt").status() {
        Ok(status) if status.success() => {}
        Ok(status) => panic!("cargo fmt exited with {status} after regenerating"),
        Err(err) => panic!("failed to run cargo fmt after regenerating: {err}"),
    }
}

/// Every `.fbs` in [`SCHEMA_DIR`], sorted so the flatc invocation is stable.
///
/// Discovered rather than listed. The TypeScript side globs the same directory,
/// so a hardcoded list here would silently cover one language and not the other:
/// a new schema would generate TypeScript bindings and no Rust ones, and the
/// drift job would report success having compared a file set that did not
/// include it.
fn schema_files() -> Vec<PathBuf> {
    let dir = Path::new(SCHEMA_DIR);
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("cannot read {SCHEMA_DIR}: {err}"))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "fbs"))
        .collect();
    if files.is_empty() {
        panic!("no .fbs schemas found in {SCHEMA_DIR}");
    }
    files.sort();
    files
}

/// Best-effort nudge when a schema looks newer than the bindings built from it.
///
/// Regeneration is opt-in, which means the failure mode for someone editing a
/// schema locally is that their new field simply does not exist and the compile
/// error says nothing about flatbuffers. This is the one moment the build script
/// runs with a chance to say so. Advisory only, and mtime-based, so it stays a
/// warning and never a hard failure.
fn warn_if_schemas_look_newer() {
    let Ok(entries) = std::fs::read_dir(SCHEMA_DIR) else {
        return;
    };
    let newest_generated = std::fs::read_dir("src/generated")
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok()?.metadata().ok()?.modified().ok())
        .max();
    let Some(newest_generated) = newest_generated else {
        return;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "fbs") {
            continue;
        }
        let Ok(modified) = entry.metadata().and_then(|m| m.modified()) else {
            continue;
        };
        if modified > newest_generated {
            println!(
                "cargo:warning={} is newer than the generated bindings. If you edited it, \
                 regenerate with: FREENET_REGEN_FLATBUFFERS=1 cargo build (needs flatc \
                 {PINNED_FLATC}), and in typescript/: npm run flatc-schemas",
                path.display()
            );
        }
    }
}

/// The `x.y.z` of the `flatc` on `PATH`, or `None` when it cannot be run.
///
/// `flatc --version` prints a line like `flatc version 24.12.23`; the version is
/// the last whitespace-separated token.
fn installed_flatc_version() -> Option<String> {
    let out = Command::new("flatc").arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8(out.stdout).ok()?;
    Some(stdout.split_whitespace().last()?.to_string())
}
