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
| `Cargo.toml` | Use workspace tracing-subscriber minimum0.3.20 with normal compatible SemVer. | stable2609 still pins0.3.19;0.3.20 addresses RUSTSEC-2025-0055. |
| `Cargo.lock` | Regenerate only the necessary matching Cargo resolution. | Owning-tool update; never copy the old release lock. |
| `substrate/primitives/tracing/Cargo.toml` | Retire the separate override; retain the exact upstream workspace inheritance. | The shared workspace requirement now carries the same compatibility intent for current0.3.23 consumers. The original standalone source patch002ba334 remains on the older branch. |
| `docs/DHIWAY_PATCH_LEDGER.md` | Replace with this branch's record. | Keep the exact new upstream base and current delta auditable. |

Official security source: [tracing advisory GHSA-xwfj-jgwm-7wp5](https://github.com/tokio-rs/tracing/security/advisories/GHSA-xwfj-jgwm-7wp5).

The new branch combines both tracing fixes in one shared dependency requirement.
The root lock still selects0.3.20 reproducibly; the manifest no longer imposes an
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

## Verification in progress

T268.169 records the exact commands, outcomes and independent review in
project-context `receipts/t268-169-sdk-stable2609.json`. Locked metadata, scoped
Statement library tests and the current external tracing consumer regression are
pending at this checkpoint. No downstream interoperability, remote publication
or full upstream release qualification is claimed.
