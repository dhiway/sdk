# Dhiway SDK patch ledger: stable2609-1

Updated 9 November 2026 for the isolated T311.37 / P36 producer upgrade.
The merge adopts released upstream `polkadot-stable2609-1` / node v1.25.1,
commit `6ab53982513b24e03de15736cb45a87e5daa6919`, preserving both its
ancestry and the existing fork head `aa886dd1c64ab77de82b47eb1e909f9d3b3ff336`.
The earlier upstream base was `polkadot-stable2609`, commit
`7a8daa68e18718c18823c1109513fdc1c8aa71fe`. No rebase, consumer re-pin,
publication, network reset or deployment is part of this SDK source change.

## Carry map

The previous fork had fourteen commits over the earlier upstream base, touching
twelve files. Compare the resulting tree with the exact new upstream, rather
than treating every historical fork commit as a remaining custom patch.

| Existing fork change | Disposition at stable2609-1 | Reason |
| --- | --- | --- |
| `64d8f6f739fe922f33891981ba20e92eb18b14a9`: workspace tracing-subscriber minimum 0.3.20 | Retain the root manifest and its existing lock resolution | Upstream still requires exact 0.3.19. The shared minimum carries RUSTSEC-2025-0055 remediation and permits current compatible downstream versions; no separate sp-tracing override. |
| `ed93bbc5b824abb5b9a07a999eed25391aa5e596`: original ledger | Replace stale current-state claims with this inventory | Historical baseline verification remains below and in its receipt; it does not describe the later native Statement delta. |
| `9d10387e62`: best/finalized allowance enforcement | Upstream-equivalent; no separate custom hunk remains | Upstream `1926bb6275c` carries the same fix. Git reconciles it without a duplicate implementation; retain its upstream regression. |
| `552780e2e3`: reject non-contributory X25519 ephemeral keys | Retain `substrate/primitives/statement-store/src/ecies.rs` and its forged-ciphertext regression | This released upstream still lacks the check. A non-contributory exchange must reject with BadData. |
| `bc385072a5`: paced native admission replay | Retain in `substrate/client/network/statement/src/lib.rs` | This released upstream lacks the carried replay pacing; do not replace native Statement transport with another sync system. |
| `39800e39a9`, `9955298eb8`: opt-in full-replication configuration and interval validation | Retain network configuration and native Store/handler integration | Defaults remain disabled. Invalid or unrepresentable repair intervals reject rather than panic or busy-loop. |
| `f725d22fd5`, `5797957c4e`: deterministic admission, repair-safe eviction and authoritative tombstones | Retain Store implementation and regressions | Equal priorities converge under the selected policy; automatic eviction can be repaired while explicit suppression remains authoritative. |
| `3187611b09`: bounded full-peer repair | Retain native handler replay, bounded sends and fairness | Complete retained-feed replication needs replay after reconnect and periodic repair, not only new-message gossip. |
| `828b960f7d`, `aa886dd1c6`: activation and genuine legacy-layout preflight | Retain the existing explicit legacy-layout decision | Inspect authoritative columns before expanding layout or maintenance. Reject ambiguous suppression or unsendable retained records without inventing provenance or erasing state. |
| `6000e7af17`: config forwarding and affinity flood checks | Retain native SDK constructors and network checks | Both native node constructors forward the same option; full-mode affinity does not bypass rate accounting. |
| `fa4528dcef`: busy run-loop and real Store/wire composition tests | Retain both test modules | Exercise two full peers despite continuously ready live ticks, and actual Store live/replay V1/V2 encoding boundaries. |
| New root `rust-toolchain.toml` | Pin Rust 1.93.0 with rustfmt, Clippy, rust-src and wasm32v1-none | Matches upstream `.github/env` CI image and the previously accepted SDK toolchain. Makes the exact archived tree self-describing for the existing isolated AWS runner; no consumer toolchain is changed. |

The full upstream patch release is merged intact, including its native, runtime,
manifest/lock and release-note changes. Only the carried Store allowance fix is
already upstream-equivalent. The remaining custom source paths, beyond the
root toolchain and ledger, are:

- `Cargo.toml` and `Cargo.lock` (shared tracing requirement and its prior resolution).
- `substrate/primitives/statement-store/src/ecies.rs`.
- `substrate/client/network/statement/src/config.rs`, `src/lib.rs` and
  `src/full_replication_tests.rs`.
- `substrate/client/statement-store/src/lib.rs` and `src/tests/full_replication.rs`.
- `substrate/bin/node/cli/src/service.rs` and
  `cumulus/polkadot-omni-node/lib/src/{cli.rs,common/statement_store.rs}`.

The inherited lock resolution is merged, not hand-regenerated or copied from
another release. Its remaining difference from upstream includes the previously
accepted transitive tracing-resolution changes. Locked Cargo preparation and
checks must validate the merged manifest/lock before consumer adoption.

## Behavior and legacy-layout boundary

Full replication is opt-in and uses native Statement admission journals,
watermarks, subscriptions and protocol messages. It preserves bounded per-peer
work, flood accounting and live/replay fairness. Local admission or a finished
replay is not a recipient acknowledgement or consensus durability proof.
Convergence still requires compatible policies, sufficient capacity and a
surviving eligible copy. Ordinary disabled-mode behavior remains supported.

The existing legacy-layout preflight is deliberately retained, as required by
this task; removal needs a separately explicit owner decision. In full mode,
legacy suppression records cannot be classified as explicit or automatic from
presence alone. Startup rejects them before column expansion, version updates
or maintenance. Follow the existing disabled-mode maintenance/removal policy
before retrying; do not silently reset the database or reinterpret old rows.
Tests retain genuine three-column databases, suppression/reopen checks, malformed
layout rejection and retained encoded-size activation checks.

## Verification boundary

Historical accepted baseline: T268.187 at
`aa886dd1c64ab77de82b47eb1e909f9d3b3ff336` passed 217 integrated primitive,
network and Store unit tests, scoped strict Clippy, affected format checks and
native SDK consumer checks. Those are previous results, not proof of this merge.
T272.1 mapped the fork and requested this ledger correction at the next real
source change; this update fulfills that advisory without rewriting its record.

Current merge checks are recorded by the owning context task T311.37 receipt
and exact-head AWS evidence. The planned development gates are primitive,
network and Store library tests, scoped formatting and scoped strict all-target
Clippy; native constructor changes are unchanged from the accepted fork but
consumer adoption/typechecking belongs to P37/P38. Do not read a listed command
as a passing result until its exact-head manifest/logs and independent review
are recorded. Full SDK matrices, live multi-node/WAN churn and capacity,
consumer pins, publication and production release qualification remain separate.
