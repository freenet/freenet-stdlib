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

/// Records the schema content the committed bindings were generated from.
///
/// Written by the regeneration path and committed alongside the bindings, so an
/// ordinary build can tell whether the two still correspond without needing
/// flatc. The drift job regenerates it like everything else, so a stale one is
/// itself caught as drift.
const SCHEMA_FINGERPRINT: &str = "src/generated/.schema-fingerprint";

fn main() {
    println!("cargo:rerun-if-env-changed=FREENET_REGEN_FLATBUFFERS");

    // Declared only when the schemas are actually present, and that condition
    // is the whole point.
    //
    // `cargo package` ships only files under `rust/`, so in a registry checkout
    // `../schemas/` does not exist — and cargo treats a `rerun-if-changed` path
    // that does not exist as permanently dirty. Declaring them unconditionally
    // rebuilt `freenet-stdlib`, and everything downstream of it, on every single
    // `cargo build` in every consumer (~1s per no-op build for this crate alone,
    // before its reverse-dependency cone).
    //
    // Declaring nothing at all is equally wrong in the other direction: cargo
    // then re-runs this script only when the env var changes, so editing a
    // schema would not re-run it, and the stale-schema warning below could never
    // fire in the one situation it exists for.
    //
    // So existing paths get the directive. Where they are absent no path
    // directive is emitted, and since `rerun-if-env-changed` above is
    // unconditional the script has still opted out of cargo's default
    // "any packaged file" scan — it re-runs only on the env var. That is right
    // for a registry checkout, whose sources cannot change anyway.
    let schemas = discover_schemas();
    if let Some(schemas) = &schemas {
        for schema in schemas {
            println!("cargo:rerun-if-changed={}", schema.display());
        }
    }

    if std::env::var_os("FREENET_REGEN_FLATBUFFERS").is_none() {
        warn_if_schemas_stale();
        return;
    }

    let schemas = schemas.unwrap_or_else(|| {
        panic!("FREENET_REGEN_FLATBUFFERS is set but {SCHEMA_DIR} does not exist")
    });
    if schemas.is_empty() {
        panic!("no .fbs schemas found in {SCHEMA_DIR}");
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

    // Last, so it records only schemas that actually made it through.
    let Some(fingerprint) = fingerprint(&schemas) else {
        panic!("could not read the schemas to fingerprint them");
    };
    if let Err(err) = std::fs::write(SCHEMA_FINGERPRINT, format!("{fingerprint}\n")) {
        panic!("failed to write {SCHEMA_FINGERPRINT}: {err}");
    }
}

/// Every `.fbs` in [`SCHEMA_DIR`], sorted so the flatc invocation is stable, or
/// `None` when the directory is not there at all.
///
/// Discovered rather than listed. The TypeScript side globs the same directory,
/// so a hardcoded list here would silently cover one language and not the other:
/// a new schema would generate TypeScript bindings and no Rust ones, and the
/// drift job would report success having compared a file set that did not
/// include it.
///
/// Returns `None` rather than panicking on a missing directory because that is
/// the normal state of a published crate — this runs on every downstream build,
/// where a panic would be far worse than the problem it reports.
fn discover_schemas() -> Option<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(Path::new(SCHEMA_DIR))
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "fbs"))
        .collect();
    files.sort();
    Some(files)
}

/// Nudge when the schemas no longer match the bindings built from them.
///
/// Regeneration is opt-in, which means the failure mode for someone editing a
/// schema locally is that their new field simply does not exist and the compile
/// error says nothing about flatbuffers. This is the one moment the build script
/// runs with a chance to say so. Advisory only: it warns and never fails.
///
/// Compares CONTENT, not timestamps. An earlier version of this compared
/// schema mtimes against the generated files and fired on every fresh clone,
/// because git writes checkout files in path order — `rust/src/generated/`
/// sorts before `schemas/`, so every schema was always strictly newer. It
/// emitted fifteen warnings across five jobs in a CI run where the drift job
/// simultaneously proved the bindings matched exactly. A warning that is
/// already going off is not a warning; by the time it was true nobody would
/// have been able to tell it apart from the noise.
fn warn_if_schemas_stale() {
    let Some(schemas) = discover_schemas() else {
        return;
    };
    let Some(current) = fingerprint(&schemas) else {
        return;
    };
    // Absent on a checkout predating the fingerprint; silence beats nagging
    // about something the developer cannot act on.
    let Ok(recorded) = std::fs::read_to_string(SCHEMA_FINGERPRINT) else {
        return;
    };
    if recorded.trim() != current {
        println!(
            "cargo:warning=the flatbuffers schemas do not match the committed bindings. \
             If you edited a schema, regenerate with: FREENET_REGEN_FLATBUFFERS=1 cargo \
             build (needs flatc {PINNED_FLATC}), and in typescript/: npm run flatc-schemas"
        );
    }
}

/// A content fingerprint of `schemas`, or `None` if any cannot be read.
///
/// FNV-1a over each file's name and bytes, in the sorted order
/// [`discover_schemas`] returns. Hand-rolled rather than pulled from a crate
/// because a build dependency for this would be absurd, and deliberately not
/// `DefaultHasher`, whose output is explicitly not stable across Rust releases
/// — that would reintroduce the false positive this replaced, keyed on the
/// contributor's toolchain instead of their clone order.
fn fingerprint(schemas: &[PathBuf]) -> Option<String> {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
    };
    for schema in schemas {
        eat(schema.file_name()?.to_str()?.as_bytes());
        eat(&std::fs::read(schema).ok()?);
    }
    Some(format!("{hash:016x}"))
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
