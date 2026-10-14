# Dhiway SDK patch ledger: release-v1.25.0

Prepared from exact upstream `polkadot-stable2609` / peeled `polkadot-v1.25.0`
commit `7a8daa68e18718c18823c1109513fdc1c8aa71fe`, released 2026-10-12.
Owner instruction 2026-10-14 requests this local release branch and current custom
patch carry. The pre-release stack permits a clean break; no Dhiway migration
framework or legacy SDK API adapter is introduced. Standard upstream code stays
intact. Existing release/development branches and consumer pins are unchanged.

## Current custom patch disposition

Inventory compared old fork `002ba334e45ed6a44f2ee1cb4fb56758c14fb074`
with upstream stable2606-2 `72284b37a234f8a1565f2043ddbf8280a69fcf29`.
Only four files differ: root manifest/lock, sp-tracing manifest, and the former
patch ledger. Historical CLI-name patches absent from that live fork are not revived.

| Path | Disposition | Reason |
| --- | --- | --- |
| `Cargo.toml` | Use workspace tracing-subscriber minimum0.3.20 with normal compatible SemVer. | stable2609 still pins 0.3.19; 0.3.20 addresses RUSTSEC-2025-0055. |
| `Cargo.lock` | Regenerate only the necessary matching Cargo resolution. | Owning-tool update; never copy the old release lock. |
| `substrate/primitives/tracing/Cargo.toml` | Retire the separate override; retain the exact upstream workspace inheritance. | The shared workspace requirement now carries the same compatibility intent for current 0.3.23 consumers. The original standalone source patch 002ba334 remains on the older branch. |
| `docs/DHIWAY_PATCH_LEDGER.md` | Replace with this branch's record. | Keep the exact new upstream base and current delta auditable. |

Official security source: [tracing advisory GHSA-xwfj-jgwm-7wp5](https://github.com/tokio-rs/tracing/security/advisories/GHSA-xwfj-jgwm-7wp5).

The new branch combines both tracing fixes in one shared dependency requirement.
The root lock still selects 0.3.20 reproducibly; the manifest no longer imposes an
exact old subscriber version on current downstream library users. No separate
sp-tracing manifest delta or compatibility shim is needed.

## Statement Store baseline

No custom Statement Store, gossip, RPC, runtime or quota source overrides are
carried. All new Statement changes are the exact stable2609 upstream code.
Relevant upstream improvements include persisted admission-cursor subscription
replay, admission-journal initial gossip sync, bounded per-peer outboxes and
inbound batches, nonblocking initial sync/propagation, expired-message filtering,
depleted-allowance rejection, index optimizations and multi-topic notification fixes.

The native APIs change, including recent-statement admission sequence tuples,
admission watermark/cursor walks and new subscriptions. Consumers should adopt
these native APIs directly in separate assigned work; no compatibility shim is added.
The upstream usage guide still disclaims guaranteed delivery/timing. Its journal
skips entries whose Statements have left the store; it is not an application
mailbox, permanent history or proof of recipient receipt.

## Validation

At source `64d8f6f739fe922f33891981ba20e92eb18b14a9`, using Rust 1.93.0 on
macOS arm64, the following passed. This ledger's later documentation-only update
preserves every production source/manifest/lock blob; the closing receipt binds
that identity and the independent verdict to the actual branch head.

- `cargo +1.93.0 test --locked -p sp-statement-store -p sc-network-statement -p sc-statement-store -p pallet-statement --lib`:189 passed, 0 failed, 0 ignored (15 primitives, 85 network, 89 store; pallet compiled with 0 library tests). This is the committed-source rerun, separate from the earlier successful cold build.
- `cargo +1.93.0 test --locked -p sp-tracing`:9 doctests passed,0 library unit tests.
- Current external path consumer with exact tracing-subscriber 0.3.23: 2 captured-output/filtering regressions passed after the shared-minimum simplification. The same fixture against pure upstream failed dependency resolution on exact 0.3.19, as expected.
- `cargo +1.93.0 clippy --locked -p sp-tracing --all-targets -- -D warnings`:passed.
- `cargo +1.93.0 metadata --locked --offline --no-deps --format-version 1`, TOML parsing and scoped Git whitespace checks:passed.
- A fresh exact-upstream-plus-one-line control regenerated a byte-identical lock. The accepted online update includes Cargo-selected transitive additions/edge rebindings; they are not claimed proved inherited. The earlier broad offline resolution was rejected and retained only as evidence.

The test runtime's external-target-directory Cargo.lock auto-discovery warnings
remain recorded. The final rerun supplied `WASM_BUILD_WORKSPACE_HINT`, but that
variable alone does not invalidate the cached build script, whose warnings were
replayed. No warning-free cold Wasm rebuild or Wasm reproducibility is claimed.
`taplo format --check --config .config/taplo.toml Cargo.toml substrate/primitives/tracing/Cargo.toml`
could not run because taplo is not installed. TOML syntax/metadata checks pass,
and the manifest change preserves the upstream line's formatting.

T268.169 retains exact commands, logs, source review and final head acceptance in
project-context `receipts/t268-169-sdk-stable2609.json`. Full SDK matrices,
consumer pin/adoption, live-chain interoperability, remote publication and full
release qualification are separate; these component checks do not establish them.
