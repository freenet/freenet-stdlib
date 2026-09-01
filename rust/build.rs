//! Build script for `freenet-stdlib`.
//!
//! The flatbuffers bindings in `src/generated/` are **checked in**, and an
//! ordinary build does not rebuild them. Regeneration happens only when
//! `FREENET_REGEN_FLATBUFFERS=1` is set, and only with the pinned `flatc`.
//!
//! It used to work the other way: every build shelled out to whatever `flatc`
//! happened to be on `PATH` and rewrote the bindings in place, silently doing
//! nothing at all when there was no `flatc` to find. Both halves of that caused
//! real problems.
//!
//! Different `flatc` releases format their output differently, so a contributor
//! whose compiler did not match the one the bindings were generated with got
//! thousands of lines of unrelated churn mixed into their diff, on every build,
//! whether or not they had touched a schema. That is not hypothetical: commit
//! 8070e93, "revert: undo accidental flatbuffers regen from #72", exists because
//! it reached `main`.
//!
//! The silent fallback is the more dangerous half. CI does not install `flatc`,
//! so CI never regenerated anything — it compiled the checked-in bindings and
//! reported success. A schema edit that was never regenerated therefore passed
//! CI while the bindings on disk still described the *old* schema, and nothing
//! anywhere compared the two.
//!
//! So: regeneration is explicit, and `ci.yml`'s `flatbuffers-drift` job is what
//! actually enforces the relationship, by regenerating with the pinned compiler
//! and failing if the result differs from what is committed.

use std::process::Command;

/// The `flatc` release whose output matches the checked-in bindings.
///
/// Verified against both languages at the time of pinning: regenerating the
/// TypeScript bindings with this version is a no-op, and the Rust bindings it
/// produces pass the byte-frozen wire-format tests in `client_api::client_events`
/// unchanged. It also matches the `flatbuffers` crate version in `Cargo.toml`;
/// keep the two in step when either moves.
const PINNED_FLATC: &str = "24.3.25";

const SCHEMAS: [&str; 3] = [
    "../schemas/flatbuffers/common.fbs",
    "../schemas/flatbuffers/client_request.fbs",
    "../schemas/flatbuffers/host_response.fbs",
];

fn main() {
    println!("cargo:rerun-if-env-changed=FREENET_REGEN_FLATBUFFERS");
    for schema in SCHEMAS {
        println!("cargo:rerun-if-changed={schema}");
    }

    if std::env::var_os("FREENET_REGEN_FLATBUFFERS").is_none() {
        return;
    }

    // Everything below panics rather than warning. Setting the variable is an
    // explicit request to regenerate, so a run that quietly regenerates nothing
    // is the same silent-no-op this script exists to remove -- and it is worse
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
    for schema in SCHEMAS {
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

/// The `x.y.z` of the `flatc` on `PATH`, or `None` when it cannot be run.
///
/// `flatc --version` prints a line like `flatc version 24.3.25`; the version is
/// the last whitespace-separated token.
fn installed_flatc_version() -> Option<String> {
    let out = Command::new("flatc").arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8(out.stdout).ok()?;
    Some(stdout.split_whitespace().last()?.to_string())
}
