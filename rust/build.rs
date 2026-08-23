use std::process::Command;

fn main() {
    // The checked-in files in src/generated are the source of truth; regenerating
    // them on every build with the local flatc dirtied the tree (freenet-core#4747).
    // After editing a schema, regenerate with `REGEN_FLATBUFFERS=1 cargo build`.
    println!("cargo:rerun-if-env-changed=REGEN_FLATBUFFERS");
    let regen = std::env::var("REGEN_FLATBUFFERS").unwrap_or_default();
    if regen.is_empty() || regen == "0" {
        return;
    }

    println!("cargo:rerun-if-changed=../schemas/flatbuffers/common.fbs");
    println!("cargo:rerun-if-changed=../schemas/flatbuffers/client_request.fbs");
    println!("cargo:rerun-if-changed=../schemas/flatbuffers/host_response.fbs");

    // Regeneration was requested explicitly, so fail loudly rather than
    // succeed with stale files.
    let status = Command::new("flatc")
        .arg("--rust")
        .arg("-o")
        .arg("src/generated")
        .arg("../schemas/flatbuffers/common.fbs")
        .arg("../schemas/flatbuffers/client_request.fbs")
        .arg("../schemas/flatbuffers/host_response.fbs")
        .status()
        .unwrap_or_else(|err| {
            panic!(
                "REGEN_FLATBUFFERS is set but flatc could not be run: {err}\n\
                 refer to https://github.com/google/flatbuffers to install the flatc compiler"
            )
        });
    assert!(status.success(), "flatc failed with {status}");
    let _ = Command::new("cargo").arg("fmt").status();
}
