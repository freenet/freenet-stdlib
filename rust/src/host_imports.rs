//! The authoritative list of WASM host imports this crate declares.
//!
//! # Why this exists
//!
//! A Rust `extern "C"` declaration becomes a WASM **import** only if something
//! calls it, and it is resolved **by name at module instantiation**. So a crate
//! can declare an import no host provides, and nothing notices: the SDK
//! compiles, the host compiles, both publish, and CI stays green on both sides.
//! It surfaces only when someone writes a contract or delegate that calls the
//! function and watches it fail to instantiate — by which point the SDK has
//! been telling authors to use it, in released documentation.
//!
//! That is not hypothetical. freenet-stdlib 0.10.0 shipped seven delegate host
//! imports that freenet-core 0.2.136 does not register: three withdrawn by
//! freenet-core#5638, and four that no released node ever provided. One of the
//! four, `__frnt__delegate__subscribe_contract_checked`, was documented as the
//! *preferred* alternative to a function that did work. They accumulated
//! because nothing compared the two sides. See freenet-stdlib#133.
//!
//! # What this gives you
//!
//! [`DECLARED_HOST_IMPORTS`] is a hand-maintained list, and
//! `host_import_manifest_tests` parses this crate's own source to check that
//! the list and the `extern "C"` blocks agree. Adding or removing an import
//! therefore cannot be silent: it fails the build until someone edits this
//! list, which puts the change in the diff a reviewer reads.
//!
//! # The other half lives in freenet-core
//!
//! This guard proves only that the list matches *what the SDK declares*. It
//! cannot see what the host registers. freenet-core should assert its own
//! linker registration set against this constant — it is `pub`, and compiled
//! into the crate freenet-core already depends on, precisely so that check
//! needs no cross-repo file plumbing. Tracked in freenet-core#5655.

/// One WASM host import: the import module it is resolved in, and its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HostImport {
    /// The `wasm_import_module` the host must register this function under.
    pub module: &'static str,
    /// The import's link name.
    pub name: &'static str,
}

impl HostImport {
    const fn new(module: &'static str, name: &'static str) -> Self {
        Self { module, name }
    }
}

/// Every WASM host import declared by this crate, sorted by `(module, name)`.
///
/// A host that runs contracts must register the `freenet_*` entries below that
/// a contract can reach; a host that runs delegates must register the
/// `freenet_delegate_*` entries. An import declared here and absent from the
/// host is a load-time failure for any guest that calls it.
///
/// **Editing this list is the point.** It is checked against the `extern "C"`
/// blocks by `host_import_manifest_tests`, so it is not documentation that can
/// drift — but it is also not generated, so a change here is a deliberate act
/// that appears in review.
pub const DECLARED_HOST_IMPORTS: &[HostImport] = &[
    HostImport::new("freenet_contract_io", "__frnt__fill_buffer"),
    HostImport::new(
        "freenet_delegate_contracts",
        "__frnt__delegate__get_contract_state",
    ),
    HostImport::new(
        "freenet_delegate_contracts",
        "__frnt__delegate__get_contract_state_len",
    ),
    HostImport::new("freenet_delegate_ctx", "__frnt__delegate__ctx_len"),
    HostImport::new("freenet_delegate_ctx", "__frnt__delegate__ctx_read"),
    HostImport::new("freenet_delegate_ctx", "__frnt__delegate__ctx_write"),
    HostImport::new(
        "freenet_delegate_management",
        "__frnt__delegate__create_delegate",
    ),
    HostImport::new("freenet_delegate_secrets", "__frnt__delegate__get_secret"),
    HostImport::new("freenet_delegate_secrets", "__frnt__delegate__get_secret_len"),
    HostImport::new("freenet_delegate_secrets", "__frnt__delegate__has_secret"),
    HostImport::new("freenet_delegate_secrets", "__frnt__delegate__list_secrets"),
    HostImport::new(
        "freenet_delegate_secrets",
        "__frnt__delegate__list_secrets_len",
    ),
    HostImport::new("freenet_delegate_secrets", "__frnt__delegate__remove_secret"),
    HostImport::new("freenet_delegate_secrets", "__frnt__delegate__set_secret"),
    HostImport::new("freenet_log", "__frnt__logger__info"),
    HostImport::new("freenet_rand", "__frnt__rand__rand_bytes"),
    HostImport::new("freenet_time", "__frnt__time__utc_now"),
];

#[cfg(test)]
mod host_import_manifest_tests {
    use super::{HostImport, DECLARED_HOST_IMPORTS};

    /// Every source file in this crate that declares host imports.
    ///
    /// Held as `include_str!` rather than read from disk so the test works from
    /// a packaged crate, and so adding a file here is a compile-time act.
    ///
    /// A new file with an `extern "C"` block and no entry here would be missed.
    /// [`every_extern_c_block_is_in_a_scanned_file`] is the backstop for that:
    /// it walks the whole `src/` tree on disk and fails if any file outside
    /// this list contains a host-import block.
    const SCANNED: &[(&str, &str)] = &[
        ("delegate_host.rs", include_str!("delegate_host.rs")),
        ("host_imports.rs", include_str!("host_imports.rs")),
        ("log.rs", include_str!("log.rs")),
        ("rand.rs", include_str!("rand.rs")),
        ("time.rs", include_str!("time.rs")),
        ("memory/buf.rs", include_str!("memory/buf.rs")),
    ];

    /// Parse `#[link(wasm_import_module = "M")] ... extern "C" { fn NAME(..) }`
    /// out of Rust source.
    ///
    /// Deliberately matches only a `fn` **declaration line inside an extern
    /// block**, never a bare occurrence of the name. This file and
    /// `delegate_host.rs` mention these identifiers dozens of times in prose,
    /// and a check satisfied by its own doc comments is not a check.
    /// Drop everything from the first `#[cfg(test)]` onward.
    ///
    /// A host import is never declared inside a test module, but a *fixture*
    /// for this parser is — the tests below contain `extern "C"` blocks as
    /// string literals, and this very file would otherwise be read as
    /// declaring `__frnt__delegate__real` and friends. Stripping test code
    /// first is what lets the guard scan its own source honestly rather than
    /// carve out an exception for it.
    fn strip_test_modules(src: &str) -> &str {
        match src.find("#[cfg(test)]") {
            Some(i) => &src[..i],
            None => src,
        }
    }

    fn parse_imports(src: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        let mut pending_module: Option<String> = None;
        let mut current_module: Option<String> = None;
        let mut in_extern = false;

        for line in src.lines() {
            let t = line.trim();

            if in_extern {
                if t == "}" || t.starts_with("} ") {
                    in_extern = false;
                    current_module = None;
                    continue;
                }
                if let Some(rest) = t.strip_prefix("fn ").or_else(|| {
                    t.strip_prefix("pub fn ")
                        .or_else(|| t.strip_prefix("pub(crate) fn "))
                }) {
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    if !name.is_empty() {
                        // An extern block with no `#[link]` resolves in "env".
                        // Recording it as such makes an unattributed block show
                        // up as a mismatch rather than vanish.
                        let module = current_module.clone().unwrap_or_else(|| "env".to_string());
                        out.push((module, name));
                    }
                }
                continue;
            }

            if let Some(rest) = t.strip_prefix("#[link(wasm_import_module = \"") {
                if let Some(end) = rest.find('"') {
                    pending_module = Some(rest[..end].to_string());
                }
                continue;
            }

            if t.starts_with("extern \"C\"") {
                in_extern = true;
                current_module = pending_module.take();
                continue;
            }

            // Anything else that is not an attribute clears a dangling
            // `#[link]`, so the module cannot leak onto an unrelated block.
            if !t.starts_with("#[") && !t.is_empty() {
                pending_module = None;
            }
        }

        out
    }

    fn declared_from_source() -> Vec<(String, String)> {
        let mut found: Vec<(String, String)> = SCANNED
            .iter()
            .flat_map(|(_, src)| parse_imports(strip_test_modules(src)))
            .collect();
        found.sort();
        found.dedup();
        found
    }

    /// The guard. The `extern "C"` blocks and [`DECLARED_HOST_IMPORTS`] must
    /// name exactly the same set.
    ///
    /// If this fails, do not "fix" it by editing the list to match. Ask first
    /// whether freenet-core registers the import — an import the host does not
    /// provide is a load-time failure for any guest that calls it, which is the
    /// failure this whole module exists to prevent.
    #[test]
    fn the_declared_manifest_matches_the_extern_blocks() {
        let from_source = declared_from_source();

        let mut from_manifest: Vec<(String, String)> = DECLARED_HOST_IMPORTS
            .iter()
            .map(|i| (i.module.to_string(), i.name.to_string()))
            .collect();
        from_manifest.sort();

        let missing: Vec<_> = from_source
            .iter()
            .filter(|i| !from_manifest.contains(i))
            .collect();
        let extra: Vec<_> = from_manifest
            .iter()
            .filter(|i| !from_source.contains(i))
            .collect();

        assert!(
            missing.is_empty() && extra.is_empty(),
            "host import manifest is out of step with the extern \"C\" blocks.\n\
             Declared in source but absent from DECLARED_HOST_IMPORTS: {missing:?}\n\
             Listed in DECLARED_HOST_IMPORTS but not declared in source: {extra:?}\n\
             \n\
             Adding an entry is only correct if freenet-core registers it. See \
             the module docs."
        );
    }

    /// The manifest must be sorted and free of duplicates, so a diff against it
    /// is readable and two entries cannot disagree.
    #[test]
    fn the_manifest_is_sorted_and_unique() {
        let mut sorted = DECLARED_HOST_IMPORTS.to_vec();
        sorted.sort();
        assert_eq!(
            DECLARED_HOST_IMPORTS,
            sorted.as_slice(),
            "DECLARED_HOST_IMPORTS must be sorted by (module, name)"
        );

        let mut seen = sorted.clone();
        seen.dedup();
        assert_eq!(
            seen.len(),
            DECLARED_HOST_IMPORTS.len(),
            "DECLARED_HOST_IMPORTS contains duplicates"
        );
    }

    /// The seven imports removed in 0.11.0 must stay gone.
    ///
    /// A named regression test rather than a comment, because the way each of
    /// these arrived was a plausible-looking addition to the extern block. The
    /// guard above would catch a re-add as a manifest mismatch; this says, in
    /// the failure message, why it is not simply a list that needs updating.
    #[test]
    fn the_imports_removed_in_0_11_0_have_not_come_back() {
        const REMOVED: &[&str] = &[
            "__frnt__delegate__put_contract_state",
            "__frnt__delegate__update_contract_state",
            "__frnt__delegate__subscribe_contract",
            "__frnt__delegate__subscribe_contract_checked",
            "__frnt__delegate__list_subscriptions_len",
            "__frnt__delegate__list_subscriptions",
            "__frnt__delegate__schedule_wakeup",
        ];

        let from_source = declared_from_source();
        for name in REMOVED {
            assert!(
                !from_source.iter().any(|(_, n)| n == name),
                "`{name}` was removed in 0.11.0 because no released freenet-core \
                 registers it; a delegate calling it fails to instantiate. \
                 Re-adding it needs the host side to exist first."
            );
            assert!(
                !DECLARED_HOST_IMPORTS.iter().any(|i| i.name == *name),
                "`{name}` is back in DECLARED_HOST_IMPORTS; see freenet-stdlib#133"
            );
        }
    }

    /// The parser must not be satisfied by prose.
    ///
    /// `delegate_host.rs` names its imports repeatedly in doc comments, so a
    /// scraper that matched bare occurrences would pass while the extern block
    /// said something else entirely. This pins that it does not.
    #[test]
    fn prose_mentioning_an_import_is_not_read_as_a_declaration() {
        let src = r#"
/// Calls `__frnt__delegate__ghost` under the hood, see fn __frnt__delegate__phantom
// fn __frnt__delegate__commented_out(a: i32) -> i32;
#[cfg(target_family = "wasm")]
#[link(wasm_import_module = "freenet_real")]
extern "C" {
    /// Doc mentioning fn __frnt__delegate__not_this
    fn __frnt__delegate__real(a: i32) -> i32;
}

fn __frnt__delegate__local_definition() -> i64 { 0 }
"#;
        assert_eq!(
            parse_imports(src),
            vec![(
                "freenet_real".to_string(),
                "__frnt__delegate__real".to_string()
            )],
            "only a `fn` declaration line inside an extern block is an import"
        );
    }

    /// A `#[link]` attribute must not leak onto a later, unrelated extern block.
    #[test]
    fn a_link_attribute_does_not_leak_past_intervening_code() {
        let src = r#"
#[link(wasm_import_module = "freenet_first")]
extern "C" {
    fn __frnt__one() -> i32;
}

pub fn something_in_between() {}

extern "C" {
    fn __frnt__two() -> i32;
}
"#;
        assert_eq!(
            parse_imports(src),
            vec![
                ("freenet_first".to_string(), "__frnt__one".to_string()),
                // No `#[link]`, so it resolves in "env" — and shows as a
                // mismatch rather than silently inheriting the module above.
                ("env".to_string(), "__frnt__two".to_string()),
            ]
        );
    }

    /// No source file outside [`SCANNED`] declares host imports.
    ///
    /// Without this, adding a new module with an `extern "C"` block would be
    /// invisible to the guard — the exact silence the guard exists to remove.
    /// Walks `src/` on disk, so it is skipped when the tree is not present
    /// (a packaged-crate build), where the `include_str!` set is fixed anyway.
    #[test]
    fn every_extern_c_block_is_in_a_scanned_file() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        if !root.is_dir() {
            return;
        }

        let known: Vec<String> = SCANNED
            .iter()
            .map(|(p, _)| p.replace('/', &std::path::MAIN_SEPARATOR.to_string()))
            .collect();

        let mut unscanned = Vec::new();
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("read src/") {
                let path = entry.expect("dir entry").path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                    continue;
                }
                let rel = path
                    .strip_prefix(&root)
                    .expect("under src/")
                    .to_string_lossy()
                    .to_string();
                if known.contains(&rel) {
                    continue;
                }
                let src = std::fs::read_to_string(&path).expect("read source");
                if !parse_imports(strip_test_modules(&src)).is_empty() {
                    unscanned.push(rel);
                }
            }
        }

        assert!(
            unscanned.is_empty(),
            "these files declare host imports but are not in SCANNED, so the \
             manifest guard cannot see them: {unscanned:?}"
        );
    }

    /// `HostImport` is reachable on the published API, since freenet-core is
    /// meant to assert its registration set against it.
    #[test]
    fn the_manifest_is_public_api() {
        let one: HostImport = DECLARED_HOST_IMPORTS[0];
        assert!(one.module.starts_with("freenet_"));
        assert!(one.name.starts_with("__frnt__"));
    }
}
