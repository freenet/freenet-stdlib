# The delegate wire format: what is safe to change, and what is not

*Not in `docs/` because a bare `docs` line in `.gitignore` makes that directory
untracked, and CI skips it via `paths-ignore` — a file put there is silently
never committed.*

The host↔delegate boundary (`InboundDelegateMsg`, `OutboundDelegateMsg`, and the
structs they carry) is serialized with **bincode 1.3.3**. Once a delegate WASM is
built and deployed, its decoder is frozen. You cannot fix a wire mistake in a
later release, because the thing holding the old expectation is someone else's
compiled artifact.

This note records what bincode actually does, established by running it rather
than by reading its docs — several of these contradict the intuition, and one
contradicts what its own error strings suggest. Where a rule has an executable
form, the test is named; prefer changing the test over changing this file.

## The one-line rules

1. **Never append a field to a struct that is already on the wire.**
2. **Appending an enum variant is safe only in one direction**, and the other
   direction is a hard failure you must design around, not discover.
3. **Prefer a host function to a new variant** when the answer is synchronous.
4. **`#[non_exhaustive]` does nothing on the wire.** It is not a compatibility
   mechanism. It is a `match` attribute.
5. **Never reach for `#[serde(other)]` here.** It appears to solve the problem
   and makes it worse.
6. **A delegate should not originate an absolute deadline.** It *can* read a
   clock, but a deadline bakes the delegate's reading of it into a field the
   host must honour, and a relative delay does not.

## Appending an enum variant

| direction | result |
|---|---|
| old sender → new receiver | **fine.** Existing tags are unchanged. |
| new sender → old receiver | **hard error.** Not a skipped message, not a misparse. |

The error text, for the 9th variant arriving at a receiver that knows 8:

```
invalid value: integer `8`, expected variant index 0 <= i < 8
```

It is an `ErrorKind::Custom` produced by serde's derived visitor. It is **not**
`ErrorKind::InvalidTagEncoding`, despite that variant's description string
reading "tag for enum is not valid" — bincode constructs that one only for a bad
`Option` discriminant, in a single place (`bincode-1.3.3/src/de/mod.rs:340`). An
external reviewer once recommended asserting `InvalidTagEncoding` here; a test
written to that suggestion would have failed permanently while looking correct.

**The consequence that matters.** A new *inbound* variant is sent by the host, so
the host can break every deployed delegate the moment it sends one. That is safe
only if the host sends it **strictly in reply to something an older delegate
cannot have sent**. `WakeupFired` is safe because only a delegate that emitted
`ScheduleWakeup` ever receives one. A hypothetical "your run was truncated"
notice would **not** be safe, because nothing stops the host emitting it to a
delegate that never opted in.

That opt-in property is a property of the *host implementation*, not of the wire
format. Nothing here enforces it. If you add an inbound variant, say in the PR
which host behaviour keeps it safe, and make sure the host half preserves it.

Executable form: `delegate_wire_compat::a_new_variant_does_not_decode_on_an_old_receiver`,
`::a_hand_built_old_encoder_payload_decodes_into_the_same_variant`.

## Appending a struct field — the dangerous one

Appending an enum variant and appending a struct field break in **opposite**
directions, and carrying the intuition from one to the other is how this goes
wrong.

| direction | result |
|---|---|
| old bytes → new struct | **hard error**: `io error: unexpected end of file` |
| new bytes → old struct, struct terminal in its message | silently truncated, no error |
| new bytes → old struct, **not** terminal | **silent corruption of everything after it** |

The corruption is real and quiet. Encoding `(NewS { a, b, c }, trailing: u32)`
and decoding as `(OldS { a, b }, u32)` yields the wrong trailing value — the
appended field's bytes are read as the next field's — with no error anywhere.
bincode is positional and carries no field names, so there is nothing for a
decoder to skip.

**`#[serde(default)]` does not help.** It fires only when a self-describing
format reports a field absent *by name*; bincode just runs out of bytes. That
`ContractState::size_bytes` carries `#[serde(default)]` makes this easy to
misread — that attribute protects `serde_json`, not this path.

Note who this hurts: "old bytes → new struct is a hard error" means a delegate
built against a **newer** stdlib breaks on an **older** node — the ordinary state
of the world between a stdlib release and the freenet-core bump that consumes it,
so not an exotic case.

**That one deployment gets bitten twice, and the second bite is the quiet one.**
The same newer-stdlib delegate on the same older node also *sends* new bytes to
it, which is the silent-truncation-or-corruption row of the table. The hard error
is the direction you notice; the corruption is the direction that matters, and
they arrive together. A document about this wire is not doing its job if it names
only the loud one.

Prefer a new enum variant over a new field on an existing wire struct: a variant
is only seen by a peer that asked for it **when the sender guarantees that** (see
the variant section above — nothing in the format enforces it). A struct field
has no such option even in principle: it rides along on every message of that
type, to every peer, whether or not anyone asked. The comparison holds; it is the
sender's discipline that carries it, not the wire.

Executable form: the `struct_field_wire_compat` module, including
`node_diagnostics_response_is_terminal_in_its_message`, which pins a terminality
property that is invisible at the definition site.

## `#[non_exhaustive]` is not a wire mechanism

Encoded bytes are **byte-identical** with and without it, and decode behaviour is
identical in both directions. It changes nothing at runtime.

What it does is force downstream crates to include a wildcard arm. That is why
the two enums differ deliberately, and the difference is not stylistic:

- **`InboundDelegateMsg` is `#[non_exhaustive]`.** Its consumers are third-party
  delegate WASM, which can reasonably ignore a variant it does not recognize —
  *at the source level*. This buys nothing at the wire level; see above.
- **`OutboundDelegateMsg` is deliberately not.** freenet-core dispatches it in
  exhaustive matches with no wildcard, so a new variant is a compile error there
  until someone handles it. Marking it would let a new variant compile against
  the host with **no handler**, silently swallowing the delegate's request.

Two honest limits on that argument, both learned the hard way:

- The compile error forces an **arm**, not a working handler. An arm returning
  `Ok(())` compiles perfectly and is exactly the silent stub in question. This
  crate proves it: the FlatBuffers encoder has six arms that log and drop.
- `#[non_exhaustive]` has no effect **inside the defining crate**, which is why
  the tag map is an exhaustive match here — that is what makes an unpinned
  variant a compile error rather than a silent omission.

## Do not use `#[serde(other)]`

The common advice is that `#[serde(other)]` only works for self-describing
formats. **That is wrong for bincode 1.x** — a unit catch-all variant does absorb
an unknown tag, and the decode "succeeds".

That is worse than the error it replaces, and the precise statement is stronger
than the loose one, so state it precisely.

The catch-all consumes only the *tag*, never the unknown variant's payload,
because the receiver has no idea how many bytes that variant carries.

- If the unknown variant is a **unit** variant, there is no payload, nothing
  after it is misread, and the decode is genuinely clean.
- The moment the unknown variant carries a **payload**, those bytes are read as
  whatever comes next in the buffer, silently.

So the failure is conditional on a property of a variant **that does not exist
yet** — you are betting that nobody ever gives a future variant a field. That is
exactly why it is unusable: a developer who tries it against a unit variant sees
it work, concludes the warning is overstated, and ships the bet.

Executable form, in `delegate_wire_compat`:
`serde_other_does_absorb_an_unknown_tag_in_bincode` (the claim that contradicts
the common advice), `the_catch_all_silently_corrupts_trailing_data` (the payload
case), and `the_catch_all_is_clean_for_a_unit_variant` (the case that misleads).
On mock types, deliberately — the real enums must never grow a catch-all.

Take the hard error.

## Prefer a host function where you can

Where a capability can be expressed either as a wire variant or as a host
function, the host function has the better failure mode:

| | old peer → new peer | new peer → old peer |
|---|---|---|
| new enum variant | fine | hard decode error, **mid-protocol**, unpreventable |
| new host function | unaffected if not imported | fails to **instantiate**, named missing-import, **at load** |

> **This row is reasoned, not measured — the only such claim in this file.**
> Every other rule here names a test. The instantiation-failure behaviour is
> standard WASM linking and there is no reason to doubt it, but freenet-core has
> no `unknown import` handling or test anywhere, and an attempt to demonstrate it
> locally failed to link for unrelated reasons. A `wat`-based test is recorded as
> owed on freenet-core#3972. Until it exists, treat this as the well-founded
> expectation it is rather than as a verified property — and note that
> freenet-stdlib#82's design rests on it.

Host functions resolve by name at module instantiation. A delegate that does not
import one is entirely unaffected. A delegate that does, on a node too old to
provide it, fails to load with an error naming the missing import — before it has
touched any state, and in a form a human can act on. Compare a new variant, which
fails partway through a protocol exchange with no way for the delegate to have
checked first.

They also cost no tag, so they cannot collide with a concurrent change.

`DelegateCtx::list_subscriptions` and `subscribe_contract_checked` are both this
shape. Note the tradeoff: a host function's return channel is an `i64`, so design
it to carry outcomes rather than a `bool` — collapsing it is how
`subscribe_contract` came to report "registered but not pinned" as success.

## If you are adding a variant

1. Append at the **END**. Never insert, never reorder, never remove.
2. Add it to `pinned_inbound_tag` / `pinned_outbound_tag` — exhaustive matches,
   so this is a compile error if you forget.
3. Bump `INBOUND_VARIANT_COUNT` / `OUTBOUND_VARIANT_COUNT`, and add a value to
   `every_inbound` / `every_outbound`. The unknown-tag probe fails if you don't.
4. Say in the PR **which host behaviour makes the new direction safe.**
5. Bump the crate minor version. Appending to `OutboundDelegateMsg` is
   source-breaking for downstream exhaustive matches; on a 0.x line that is a
   minor bump.

**If a tag pin fails, do not update the expected numbers.** Either a variant was
inserted or reordered — revert it and append instead — or one was removed, which
renumbers every later tag and needs a deliberate release decision. The
`RegisterDelegateWithPredecessors` removal in 0.9.0 is the shape of that
decision: it was appended last specifically so that removing it renumbered
nothing.

## Rebasing a stacked PR after a squash merge

The lower PR merges as a **squash**, so its commits are not ancestors of `main` —
only their combined content is, under one new SHA. Your stacked branch still
carries the originals.

A plain `git rebase origin/main` therefore replays **the whole merged change**
on top of a `main` that already contains it. Observed on freenet-stdlib#82 after
#98 merged: the PR went `CONFLICTING` with a diff of **+2022 across 8 files**,
where its own change is 3 files.

Replay only your own commits:

```bash
git rebase --onto origin/main <your-commit>^   # or <old base head>
```

The tell is the diff size. If a rebase leaves you with the lower PR's files in
your diff, you rebased onto the wrong base — the branch is fine, the rebase was
wrong. That is worth knowing in advance, because the natural reading is the
opposite.

## Resolving a conflict in a test module

Two changes appending at the same anchor conflict there, and the resolution is
almost always "keep both sides". That is right for the *content* and unsafe for
the *delimiters*.

Git's conflict regions are line ranges. They do not respect syntactic
boundaries, so a closing `}` or an attribute can sit in the shared trailing
context rather than inside either side, and concatenating both sides drops it.
Observed while merging two test modules: `mod a_tests { … }`'s closing brace and
the following `#[cfg(test)]` both vanished, because the brace belonged to the
context and not to either alternative.

### What the compiler actually catches, measured

Do not guess at this — the intuitive answer is wrong in both directions.

| What is lost | Does the test still run? | What you are told |
|---|---|---|
| `#[cfg(test)]` on the module | **Yes, all of them** | `cargo build` warns (`unused import`) — the module now compiles into the library |
| `#[test]` on a function | **No** — silently absent from the run | `warning: function … is never used` (dead_code) |
| The function or `mod` itself, absorbed into a neighbour | **No** | **Nothing.** There is no referent left to warn about |
| A closing `}` | n/a | Hard error — but only because of where the token happened to sit |

So `#[cfg(test)]` is **not** a silent-loss mode: `#[test]` functions are collected
by the harness regardless of it, and the attribute governs whether the module
compiles *outside* test builds, not whether its tests run *inside* one. Losing it
makes the build noisier, not quieter.

The genuinely dangerous rows are the middle two. A dropped `#[test]` does earn a
`dead_code` warning, but the **test run itself stays green with a smaller count**,
and that warning arrives in a wall of build output nobody reads on a green run. A
function absorbed wholesale earns nothing at all.

The real silent-exclusion mechanism in this repo is a **target** gate, not a test
gate: the twelve `list_subscriptions` guards sat behind
`cfg(target_family = "wasm")`, which genuinely excludes them, and CI's wasm32 jobs
build and lint without executing anything. That is the shape to fear.

### The check

Because the failure is a smaller number rather than a red one:

```
cargo test --features … <module_name>     # per module, and count them
```

Confirm each affected module still **executes**, by name and by count. A total
that looks plausible is not evidence; the number you are checking against moved
too.

## A stacked PR gets no CI here, silently

Wire changes often arrive as a stack — one PR appends a variant, the next appends
another behind it — because that is the only way to keep each diff reviewable.
Be aware of what that costs.

`.github/workflows/ci.yml` triggers only on pull requests targeting `main`:

```yaml
pull_request:
  branches: [main]
```

So a PR based on **another PR's branch** runs **none** of the Rust jobs. It does
not fail. It shows whatever always-on checks exist (`license/cla`) and nothing
else — one green check, no red ones, which reads as passing and is not. There is
no red mark anywhere to notice.

The sequence that works:

1. Base the upper PR on the lower one, so its diff shows only its own change.
2. Verify it locally and **say in the PR that CI has not run**, with the commands
   and their output. A local run is evidence; it is not the gate.
3. Merge the lower PR. GitHub retargets the upper one to `main` automatically.
4. **CI fires then.** Only now is the upper PR mergeable.

Never queue the upper PR before its jobs have actually run. "Not red" is not
green when nothing was asked.

This is the same defect class the rest of this file is about: a true signal
answering the wrong question. `license/cla` really did pass. It just says
nothing about whether the code compiles.

## Time: prefer a delay to a deadline

Relevant to any wire field carrying a deadline, and easy to get wrong in both
directions.

**A delegate can read a clock.** `freenet_stdlib::time::now()` imports
`freenet_time::__frnt__time__utc_now`, and freenet-core registers it on the
**same** `Linker<HostState>` as the four `freenet_delegate_*` namespaces, inside
the same `register_host_functions`, which both production engine constructors
call. There is no separate delegate linker and no import allowlist. Core's own
conformance doc treats this as intended rather than incidental: for a delegate a
host clock "is unremarkable", because a delegate holds private per-node state, is
never replicated, and has no merge laws to satisfy. The #5465 host-clock
deprecation is **contract-only**.

What *is* true, and worth knowing separately: `SystemTime::now()` from `std`
compiles for `wasm32-unknown-unknown` and then **panics at runtime** — "time not
implemented on this platform". The clock arrives by host import, not from `std`.

**So prefer a relative delay anyway**, for reasons that have nothing to do with
availability:

- A deadline bakes the *delegate's* reading of a clock into a value the host must
  honour. A delay is resolved by the host against its own clock, which is the
  only clock that can act on it.
- Wall-clock steps — NTP correction, a manual change — become the host
  scheduler's business rather than a semantic left undefined on the wire.
- A delay is strictly more capable: absolute scheduling is `target - now`,
  expressible in terms of a delay, while a deadline gains nothing a delay lacks.
- A delegate re-arming a recurring wakeup asks for the same delay again, so it
  needs no timestamp echoed back to it.
- `SystemTime` has an encoding hazard a `Duration` does not: a **pre-epoch**
  value fails to serialize outright (`SystemTime must be later than
  UNIX_EPOCH`), and that is an error on the *sender*, at a point where the
  surrounding code may well `unwrap` it.

Both types encode as **u64 seconds LE + u32 nanos**, 12 bytes, so nothing is paid
for preferring the delay.

### Why a survey of host functions missed the clock

Worth more than the correction itself, because the next person will survey the
same way.

An earlier draft of this file asserted that a delegate has **no** clock, on the
strength of enumerating the four `freenet_delegate_*` namespaces and finding
nothing temporal. The clock is in a fifth namespace, `freenet_time`, which is not
delegate-prefixed — so the enumeration was bounded by a **naming convention**
rather than by the actual import surface. An enumeration is only as good as the
boundary drawn around it.

It is worse than that, and this is the part to remember. The clock is registered
through **constants** — `conformance::HOST_CLOCK_NAMESPACE` and
`HOST_CLOCK_IMPORT` — rather than string literals, deliberately, so that core's
conformance detector cannot drift from the registration. That is a good decision.
It also means **grepping the engine for `"freenet_time"` does not find the
registration** — the one hit is a WAT test fixture (`wasmtime_engine.rs:3413`),
not the `func_wrap` call. The measure that made the registration robust for one
tool made it invisible to another. If you land on that fixture, it is a thread
worth pulling rather than a dead end: it exists precisely because the real
registration is not greppable. If you are surveying what a guest can import, follow the constants, or
read the linker registration rather than searching for the names it uses.

Do not put a count of host functions in this file. It changes with nearly every
release, and a stale number in a reference reads as authority.

There is a sharper reason too: **the question has no single answer.** Counting
stdlib's `fn __frnt__delegate__*` externs gives what a delegate may *import*;
counting core's `func_wrap` registrations in the same namespaces gives what a
node *provides*. On one measurement those were 18 and 16, and the gap of 2 was
exactly the pair added to stdlib with no core half yet.

**The two sides are deliberately out of step**, and permanently so: the
stdlib-first policy means a wire or ABI addition lands here, is released, and
only then is implemented in core. Declared therefore always leads provided. A
count that does not say which side it means is not merely perishable, it is
ambiguous — and the gap is a normal, healthy state rather than a defect to
reconcile.
