//! The delegate manifest: what a delegate asks the node for, declared inside
//! its own WASM.
//!
//! A delegate that wants more than the request/response model it has always
//! had (today: lifecycle events; later: wake-ups, notifications) says so in a
//! manifest. The `#[delegate(manifest(...))]` attribute writes it into a WASM
//! custom section named [`MANIFEST_SECTION_NAME`]. The node reads that section
//! when the delegate is registered, without running any delegate code.
//!
//! # Why a manifest at all
//!
//! Two jobs, both of which need the node to know what a delegate wants
//! *before* it runs:
//!
//! 1. **Backward compatibility of new inbound messages.** An
//!    [`InboundDelegateMsg`](crate::prelude::InboundDelegateMsg) variant added
//!    in a later stdlib is a hard decode error in a delegate built against an
//!    earlier one (see `WIRE-FORMAT.md`). So the node sends a new kind of
//!    inbound message — [`LifecycleEvent`] today — **only** to a delegate
//!    whose manifest lists it. A delegate without a manifest never receives
//!    one, and behaves exactly as it did before manifests existed.
//! 2. **Consent.** Some capabilities are gated by the node on the user's
//!    permission ("run in the background"). The node reads the capabilities
//!    from the manifest and asks the user once, at registration time, for the
//!    ones not yet granted.
//!
//! # Why it cannot be swapped
//!
//! The section is part of the WASM module, and a delegate's key is derived
//! from the hash of that module. Changing the manifest changes the delegate
//! key, exactly as changing any line of code does.
//!
//! # Encoding: JSON, deliberately
//!
//! The payload is UTF-8 JSON, not bincode. The manifest is the one piece of
//! delegate metadata the node must read from delegates built against *any*
//! stdlib, including ones newer than the node. bincode is positional and
//! cannot skip what it does not know (`WIRE-FORMAT.md`), so a newer stdlib
//! adding a field or a capability would make older nodes reject the whole
//! manifest. JSON names its fields, so an older reader ignores fields it does
//! not know, and unknown capability or lifecycle names decode as
//! [`Capability::Unknown`] / [`LifecycleKind::Unknown`] and are ignored rather
//! than rejecting the manifest. (`#[serde(other)]` is safe here because JSON
//! is self-describing; `WIRE-FORMAT.md`'s warning against it is about
//! bincode.)
//!
//! # Emitting a manifest adds a section, and so changes the delegate key
//!
//! Only delegates that write `manifest(...)` get the section. Upgrading stdlib
//! does not add one to delegates that do not ask for it.

use serde::{Deserialize, Serialize};

/// Name of the WASM custom section holding the manifest.
pub const MANIFEST_SECTION_NAME: &str = "freenet-manifest";

/// The manifest format version this stdlib writes.
///
/// Informational only. Readers accept any version `>= 1` and never gate on it,
/// because a reader that refused newer versions would drop every capability it
/// does understand the moment one it does not is added. That works only under
/// two rules, which are permanent:
///
/// - the meaning of an existing field or name never changes; a changed meaning
///   gets a new field or a new name;
/// - `lifecycle` and `capabilities` entries are what a reader looks up by
///   name. A later format that needs parameters for a capability adds a new
///   top-level field for them. (A reader still tolerates a non-string entry:
///   it decodes as `Unknown`, see [`DelegateManifest::from_bytes`].)
pub const MANIFEST_VERSION: u16 = 1;

/// Largest manifest payload a reader accepts, in bytes. A manifest is a
/// handful of short names; anything bigger is not a manifest.
pub const MAX_MANIFEST_BYTES: usize = 4096;

/// What a delegate declares it wants from the node.
///
/// `#[non_exhaustive]` so fields can be added without a source break; build one
/// with [`DelegateManifest::new`].
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DelegateManifest {
    /// Format version; see [`MANIFEST_VERSION`].
    pub manifest_version: u16,
    /// Lifecycle events the delegate wants delivered. The node never sends a
    /// [`LifecycleEvent`] of a kind that is not listed here.
    #[serde(default, deserialize_with = "lenient_list")]
    pub lifecycle: Vec<LifecycleKind>,
    /// Node-enforced capabilities the delegate asks the user for.
    #[serde(default, deserialize_with = "lenient_list")]
    pub capabilities: Vec<Capability>,
}

/// A kind of [`LifecycleEvent`] a delegate can ask to receive.
///
/// Adding a kind later means adding a variant here **and** a matching
/// [`LifecycleEvent`] variant. A delegate built before the addition cannot
/// list the new kind, so it never receives the new event — which is what makes
/// appending a `LifecycleEvent` variant safe for already-deployed delegates.
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleKind {
    /// [`LifecycleEvent::Installed`].
    Installed,
    /// [`LifecycleEvent::NodeStarted`].
    NodeStarted,
    /// A name this stdlib does not know, written by a newer one. Readers
    /// ignore it. The macro never writes it; re-serializing a manifest read
    /// from a newer stdlib does (as `"unknown"`).
    #[serde(other)]
    Unknown,
}

/// A node-enforced capability, granted by the user once per app.
///
/// A node that implements capabilities refuses one until the user has granted
/// it, and remembers the answer per app, so the user is asked once. How a node
/// identifies an app is the node's business, not part of this format.
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Run without an open app: receive lifecycle events. Required by any
    /// manifest that lists a lifecycle kind (the macro enforces it).
    Background,
    /// A name this stdlib does not know, written by a newer one. Readers
    /// ignore it. The macro never writes it; re-serializing a manifest read
    /// from a newer stdlib does (as `"unknown"`).
    #[serde(other)]
    Unknown,
}

/// A lifecycle event, delivered as
/// [`InboundDelegateMsg::Lifecycle`](crate::prelude::InboundDelegateMsg::Lifecycle).
///
/// Only sent to a delegate whose manifest lists the matching
/// [`LifecycleKind`]. A node that implements delivery also requires the user's
/// [`Capability::Background`] grant for the delegate's app. The run gets the
/// delegate's registered parameters and no origin.
///
/// # Wire format
///
/// bincode, nested inside `InboundDelegateMsg`. Variants are appended, never
/// inserted or reordered (pinned by `lifecycle_event_tags_are_pinned`). A
/// variant's fields are frozen once released: `WIRE-FORMAT.md` rule 1 forbids
/// appending a field to a struct already on the wire, so new information
/// arrives as a new variant.
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum LifecycleEvent {
    /// This delegate was installed on this node: its first registration here,
    /// or the first time its app was granted [`Capability::Background`] after
    /// that. Delivered at most once per delegate key per node.
    ///
    /// A delegate typically uses it to subscribe to the contracts it watches
    /// and to do any one-time setup.
    Installed,
    /// The node started. Delivered once per node start, after the node has
    /// finished whatever restore work it does at start-up.
    ///
    /// Contract notifications that arrived while the node was down were not
    /// delivered and are never replayed, so a delegate should re-read any
    /// contract it depends on rather than assume it saw every change.
    NodeStarted {
        /// When the node was last known to be running (milliseconds since the
        /// Unix epoch), if it knows. Everything between this and now was
        /// missed. `None` means the node does not know, not that nothing was
        /// missed.
        down_since_ms: Option<u64>,
    },
}

impl LifecycleEvent {
    /// The manifest kind that must be listed for this event to be delivered.
    pub fn kind(&self) -> LifecycleKind {
        match self {
            LifecycleEvent::Installed => LifecycleKind::Installed,
            LifecycleEvent::NodeStarted { .. } => LifecycleKind::NodeStarted,
        }
    }
}

/// Why a manifest could not be read.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ManifestError {
    #[error("not a WASM module (bad magic or version)")]
    NotWasm,
    #[error("truncated or malformed WASM section structure")]
    Malformed,
    #[error("more than one `{MANIFEST_SECTION_NAME}` custom section")]
    Duplicate,
    #[error("manifest is {0} bytes, over the {MAX_MANIFEST_BYTES}-byte limit")]
    TooLarge(usize),
    #[error("manifest is not valid JSON for this schema: {0}")]
    Decode(String),
    #[error("manifest_version 0 is not a valid version")]
    BadVersion,
}

impl DelegateManifest {
    /// A manifest at the current [`MANIFEST_VERSION`].
    pub fn new(lifecycle: Vec<LifecycleKind>, capabilities: Vec<Capability>) -> Self {
        Self {
            manifest_version: MANIFEST_VERSION,
            lifecycle,
            capabilities,
        }
    }

    /// Whether the manifest asks for lifecycle events of this kind.
    pub fn wants_lifecycle(&self, kind: LifecycleKind) -> bool {
        kind != LifecycleKind::Unknown && self.lifecycle.contains(&kind)
    }

    /// Whether the manifest asks for this capability.
    pub fn wants_capability(&self, cap: Capability) -> bool {
        cap != Capability::Unknown && self.capabilities.contains(&cap)
    }

    /// Known capabilities this manifest asks for, deduplicated, in declaration
    /// order. Unknown names are dropped.
    pub fn known_capabilities(&self) -> Vec<Capability> {
        let mut out = Vec::new();
        for c in &self.capabilities {
            if *c != Capability::Unknown && !out.contains(c) {
                out.push(*c);
            }
        }
        out
    }

    /// Serialize to the section payload.
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("a manifest always serializes")
    }

    /// Parse a section payload.
    ///
    /// Tolerant of manifests written by newer stdlibs: unknown fields are
    /// ignored, a list field that is not an array (`null`, or any other shape)
    /// reads as empty, and a list entry that is not a
    /// name this reader knows (an unknown name, or a non-string value) reads as
    /// `Unknown` rather than failing the manifest, so the known entries next
    /// to it still count.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ManifestError> {
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(ManifestError::TooLarge(bytes.len()));
        }
        let m: DelegateManifest =
            serde_json::from_slice(bytes).map_err(|e| ManifestError::Decode(e.to_string()))?;
        if m.manifest_version == 0 {
            return Err(ManifestError::BadVersion);
        }
        Ok(m)
    }

    /// Read the manifest from a raw WASM module (no version prefix).
    ///
    /// `Ok(None)` means the module has no manifest section: a delegate that
    /// asked for nothing, which is every delegate built before manifests
    /// existed. Only the section headers are walked; nothing is executed or
    /// validated beyond what locating the section needs.
    pub fn from_wasm(module: &[u8]) -> Result<Option<Self>, ManifestError> {
        let mut found: Option<&[u8]> = None;
        for section in custom_sections(module)? {
            let (name, payload) = section?;
            if name == MANIFEST_SECTION_NAME.as_bytes() {
                if found.is_some() {
                    return Err(ManifestError::Duplicate);
                }
                found = Some(payload);
            }
        }
        found.map(Self::from_bytes).transpose()
    }
}

/// Decode a list whose entries are enum names, mapping any entry this reader
/// cannot decode to the enum's `#[serde(other)]` variant.
fn lenient_list<'de, D, T>(d: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned + Unknownable,
{
    // Anything but an array (`null`, or a shape a later format might use)
    // reads as an empty list: asking for nothing is the safe direction.
    let serde_json::Value::Array(raw) = serde_json::Value::deserialize(d)? else {
        return Ok(Vec::new());
    };
    Ok(raw
        .into_iter()
        .map(|v| serde_json::from_value(v).unwrap_or_else(|_| T::unknown()))
        .collect())
}

trait Unknownable {
    fn unknown() -> Self;
}
impl Unknownable for LifecycleKind {
    fn unknown() -> Self {
        LifecycleKind::Unknown
    }
}
impl Unknownable for Capability {
    fn unknown() -> Self {
        Capability::Unknown
    }
}

/// Used by `#[delegate(manifest(...))]` to check, at compile time, that the
/// section name and version it writes are the ones this stdlib reads. Not
/// part of the public API.
#[doc(hidden)]
pub const fn __manifest_macro_agrees(section: &str, version: u16) -> bool {
    let (a, b) = (section.as_bytes(), MANIFEST_SECTION_NAME.as_bytes());
    if a.len() != b.len() || version != MANIFEST_VERSION {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// A custom section's `(name, payload)`.
type CustomSection<'a> = (&'a [u8], &'a [u8]);

/// Iterate over a WASM module's custom sections.
fn custom_sections(
    module: &[u8],
) -> Result<impl Iterator<Item = Result<CustomSection<'_>, ManifestError>>, ManifestError> {
    const HEADER: [u8; 8] = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    if module.len() < HEADER.len() || module[..HEADER.len()] != HEADER {
        return Err(ManifestError::NotWasm);
    }
    let mut pos = HEADER.len();
    let mut failed = false;
    Ok(std::iter::from_fn(move || loop {
        if failed || pos >= module.len() {
            return None;
        }
        let parsed = (|| {
            let id = module[pos];
            let mut p = pos + 1;
            let size = read_leb_u32(module, &mut p)? as usize;
            let end = p.checked_add(size).ok_or(ManifestError::Malformed)?;
            if end > module.len() {
                return Err(ManifestError::Malformed);
            }
            let custom = if id == 0 {
                let name_len = read_leb_u32(module, &mut p)? as usize;
                let name_end = p.checked_add(name_len).ok_or(ManifestError::Malformed)?;
                if name_end > end {
                    return Err(ManifestError::Malformed);
                }
                Some((&module[p..name_end], &module[name_end..end]))
            } else {
                None
            };
            Ok((end, custom))
        })();
        match parsed {
            Ok((end, custom)) => {
                pos = end;
                if let Some(c) = custom {
                    return Some(Ok(c));
                }
            }
            Err(e) => {
                failed = true;
                return Some(Err(e));
            }
        }
    }))
}

fn read_leb_u32(buf: &[u8], pos: &mut usize) -> Result<u32, ManifestError> {
    let mut result: u32 = 0;
    for i in 0..5 {
        let byte = *buf.get(*pos).ok_or(ManifestError::Malformed)?;
        *pos += 1;
        if i == 4 && byte & 0xf0 != 0 {
            return Err(ManifestError::Malformed);
        }
        result |= u32::from(byte & 0x7f) << (7 * i);
        if byte & 0x80 == 0 {
            return Ok(result);
        }
    }
    Err(ManifestError::Malformed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leb(mut v: u32) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let mut b = (v & 0x7f) as u8;
            v >>= 7;
            if v != 0 {
                b |= 0x80;
            }
            out.push(b);
            if v == 0 {
                return out;
            }
        }
    }

    fn custom_section(name: &str, payload: &[u8]) -> Vec<u8> {
        let mut body = leb(name.len() as u32);
        body.extend_from_slice(name.as_bytes());
        body.extend_from_slice(payload);
        let mut out = vec![0u8];
        out.extend(leb(body.len() as u32));
        out.extend(body);
        out
    }

    fn module(sections: &[Vec<u8>]) -> Vec<u8> {
        let mut m = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        // A non-custom section first (type section, empty vec), so the walker
        // has to step over a section it does not care about.
        m.extend([0x01, 0x01, 0x00]);
        for s in sections {
            m.extend_from_slice(s);
        }
        m
    }

    fn sample() -> DelegateManifest {
        DelegateManifest::new(
            vec![LifecycleKind::Installed, LifecycleKind::NodeStarted],
            vec![Capability::Background],
        )
    }

    #[test]
    fn round_trips_through_a_wasm_custom_section() {
        let m = module(&[
            custom_section("name", b"whatever"),
            custom_section(MANIFEST_SECTION_NAME, &sample().to_bytes()),
        ]);
        assert_eq!(DelegateManifest::from_wasm(&m).unwrap(), Some(sample()));
    }

    #[test]
    fn a_module_without_the_section_has_no_manifest() {
        let m = module(&[custom_section("producers", b"rustc")]);
        assert_eq!(DelegateManifest::from_wasm(&m).unwrap(), None);
    }

    /// The exact JSON the macro writes. If this changes, every delegate that
    /// declares a manifest re-keys on its next build, and older nodes must
    /// still read it.
    #[test]
    fn json_shape_is_pinned() {
        assert_eq!(
            String::from_utf8(sample().to_bytes()).unwrap(),
            r#"{"manifest_version":1,"lifecycle":["installed","node_started"],"capabilities":["background"]}"#
        );
    }

    /// A manifest from a newer stdlib — an unknown field, an unknown
    /// capability, an unknown lifecycle kind — must still be read, keeping
    /// what this reader knows. Otherwise one new capability name would strip
    /// every older node of the kinds it does understand.
    #[test]
    fn a_newer_manifest_is_read_keeping_known_entries() {
        let json = br#"{"manifest_version":3,"lifecycle":["installed","woke_up"],
            "capabilities":["background","teleport"],"brand_new_field":{"x":1}}"#;
        let m = DelegateManifest::from_bytes(json).unwrap();
        assert!(m.wants_lifecycle(LifecycleKind::Installed));
        assert!(!m.wants_lifecycle(LifecycleKind::NodeStarted));
        assert!(!m.wants_lifecycle(LifecycleKind::Unknown));
        assert!(!m.wants_capability(Capability::Unknown));
        assert_eq!(m.known_capabilities(), vec![Capability::Background]);
    }

    #[test]
    fn missing_lists_default_to_empty() {
        let m = DelegateManifest::from_bytes(br#"{"manifest_version":1}"#).unwrap();
        assert!(m.lifecycle.is_empty() && m.capabilities.is_empty());
        let m = DelegateManifest::from_bytes(
            br#"{"manifest_version":1,"lifecycle":null,"capabilities":null}"#,
        )
        .unwrap();
        assert!(m.lifecycle.is_empty() && m.capabilities.is_empty());
        let m = DelegateManifest::from_bytes(
            br#"{"manifest_version":1,"lifecycle":"installed","capabilities":{"background":{}}}"#,
        )
        .unwrap();
        assert!(m.lifecycle.is_empty() && m.capabilities.is_empty());
    }

    /// A later format might write an entry that is not a bare name. That entry
    /// is unknown to this reader; the known ones next to it must still count.
    #[test]
    fn a_non_string_entry_reads_as_unknown_not_as_an_error() {
        let json = br#"{"manifest_version":2,
            "lifecycle":[{"woke_up":{"every_s":60}},"node_started",7],
            "capabilities":[{"notify":{"max_per_hour":4}},"background",null]}"#;
        let m = DelegateManifest::from_bytes(json).unwrap();
        assert_eq!(
            m.lifecycle,
            vec![
                LifecycleKind::Unknown,
                LifecycleKind::NodeStarted,
                LifecycleKind::Unknown
            ]
        );
        assert_eq!(m.known_capabilities(), vec![Capability::Background]);
    }

    #[test]
    fn macro_agreement_check() {
        assert!(__manifest_macro_agrees(
            MANIFEST_SECTION_NAME,
            MANIFEST_VERSION
        ));
        assert!(!__manifest_macro_agrees(
            "freenet-manifesT",
            MANIFEST_VERSION
        ));
        assert!(!__manifest_macro_agrees(
            "freenet-manifest2",
            MANIFEST_VERSION
        ));
        assert!(!__manifest_macro_agrees(
            MANIFEST_SECTION_NAME,
            MANIFEST_VERSION + 1
        ));
    }

    #[test]
    fn rejects_version_zero_oversize_and_garbage() {
        assert_eq!(
            DelegateManifest::from_bytes(br#"{"manifest_version":0}"#),
            Err(ManifestError::BadVersion)
        );
        let big = vec![b' '; MAX_MANIFEST_BYTES + 1];
        assert_eq!(
            DelegateManifest::from_bytes(&big),
            Err(ManifestError::TooLarge(MAX_MANIFEST_BYTES + 1))
        );
        assert!(matches!(
            DelegateManifest::from_bytes(b"not json"),
            Err(ManifestError::Decode(_))
        ));
    }

    /// Two sections would be ambiguous. It also catches two crates in one
    /// build each emitting a manifest: the linker would normally concatenate
    /// same-named sections into one, which fails to decode instead, but a
    /// post-link tool could leave them separate.
    #[test]
    fn rejects_a_duplicate_section() {
        let payload = sample().to_bytes();
        let m = module(&[
            custom_section(MANIFEST_SECTION_NAME, &payload),
            custom_section(MANIFEST_SECTION_NAME, &payload),
        ]);
        assert_eq!(
            DelegateManifest::from_wasm(&m),
            Err(ManifestError::Duplicate)
        );
    }

    #[test]
    fn concatenated_manifests_fail_to_decode() {
        let mut payload = sample().to_bytes();
        payload.extend(sample().to_bytes());
        let m = module(&[custom_section(MANIFEST_SECTION_NAME, &payload)]);
        assert!(matches!(
            DelegateManifest::from_wasm(&m),
            Err(ManifestError::Decode(_))
        ));
    }

    #[test]
    fn rejects_non_wasm_and_truncated_modules() {
        assert_eq!(
            DelegateManifest::from_wasm(b"\0asm"),
            Err(ManifestError::NotWasm)
        );
        assert_eq!(
            DelegateManifest::from_wasm(b"hello world, not wasm"),
            Err(ManifestError::NotWasm)
        );
        let mut m = module(&[custom_section(MANIFEST_SECTION_NAME, &sample().to_bytes())]);
        m.truncate(m.len() - 3);
        assert_eq!(
            DelegateManifest::from_wasm(&m),
            Err(ManifestError::Malformed)
        );
        // A section whose declared size runs past the end.
        let mut m = module(&[]);
        m.extend([0x00, 0xff, 0xff, 0x03]);
        assert_eq!(
            DelegateManifest::from_wasm(&m),
            Err(ManifestError::Malformed)
        );
        // An over-long LEB128 (more than 5 bytes).
        let mut m = module(&[]);
        m.extend([0x00, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00]);
        assert_eq!(
            DelegateManifest::from_wasm(&m),
            Err(ManifestError::Malformed)
        );
    }

    /// Wire pin for the nested enum. Appending a variant is fine; reordering
    /// or inserting one silently reinterprets deployed delegates' input.
    #[test]
    fn lifecycle_event_tags_are_pinned() {
        fn tag(e: &LifecycleEvent) -> u32 {
            match e {
                LifecycleEvent::Installed => 0,
                LifecycleEvent::NodeStarted { .. } => 1,
            }
        }
        let all = [
            LifecycleEvent::Installed,
            LifecycleEvent::NodeStarted {
                down_since_ms: Some(0x0102_0304_0506_0708),
            },
        ];
        for e in &all {
            let enc = bincode::serialize(e).unwrap();
            assert_eq!(u32::from_le_bytes(enc[..4].try_into().unwrap()), tag(e));
            assert_eq!(&bincode::deserialize::<LifecycleEvent>(&enc).unwrap(), e);
        }
        // Full byte layout of NodeStarted: tag 1, Option tag 1, u64 LE.
        assert_eq!(
            bincode::serialize(&all[1]).unwrap(),
            vec![1, 0, 0, 0, 1, 8, 7, 6, 5, 4, 3, 2, 1]
        );
        // `kind()` agrees with the manifest kinds.
        assert_eq!(all[0].kind(), LifecycleKind::Installed);
        assert_eq!(all[1].kind(), LifecycleKind::NodeStarted);
    }
}
