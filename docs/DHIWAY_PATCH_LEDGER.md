# Dhiway SDK patch ledger: release-v1.24.2

This branch merges upstream `polkadot-stable2606-2` at
`72284b37a234f8a1565f2043ddbf8280a69fcf29` into Dhiway's consumed
`release-v1.24.1` tip `ee8b9fb13cb3a36cafb5d199e64a27b18873eb72`.
The old upstream base is `8ae9775dc43c0d8cdd0f6d87700596e14278b1e1`.
The merge retains the Dhiway commit as an ancestor; it does not rewrite the
published release branch. The upstream tag and commit resolve to the same SHA.

## Delta against the new upstream base

| Path | Disposition | Reason |
| --- | --- | --- |
| `Cargo.toml` | Retain the exact `tracing-subscriber =0.3.20` workspace pin from `ee8b9fb`; upstream still has `=0.3.19`. | Preserve the only custom change on the currently consumed branch. |
| `Cargo.lock` | Retain the matching `0.3.20` resolution and Cargo-selected transitive changes. | Keep the manifest and lockfile consistent. Cargo validation is recorded below. |
| `docs/DHIWAY_PATCH_LEDGER.md` | Add this record. | Make the fork delta and future rebases auditable. |

All other merged source changes come from the exact upstream target. There are
no Statement source overrides in this fork delta. Upstream changes to the
Statement workspace in this release are limited to the network and store crate
manifests; the relevant APIs therefore still need downstream integration
testing rather than an SDK patch here.

## Historical fork and open PR disposition

The older `release-v1.18.2` branch carried `95c0973d0b5f55a94cc2abc83692ccca3ba42dc3`
(`add chain name`); `release-v1.19.2`, `release-v1.20.0`, and `release-v1.21.2`
repeated the `CliConfig::impl_name` addition. Those changes are absent from
the currently consumed `release-v1.24.1` branch, and the new upstream CLI
uses its executable name. They are not revived in this release merge; a
current downstream requirement for a distinct node name would warrant a
separate, tested change. Dhiway-authored historical upstream contributions
`da9c73125f` and `f5e7eaf610` are already in upstream ancestry.

| Open PR | Disposition at this baseline |
| --- | --- |
| #9, stable2509 update | Its fetched head `4bcf5fe815` adds 67 stable2509 commits after the old `release-v1.20.2` branch, with no Statement path diff or Dhiway-authored patch in that range. Do not merge the old release line into stable2606. |
| #7, hickory-proto 0.24.4 | Not in this target's lockfile (`0.24.1`, `0.25.2`, `0.26.1`). Treat as a separate dependency review, not part of the SDK baseline alignment. |
| #6, rustix 0.38.42 | Already resolved in the target lockfile. |
| #4, openssl 0.10.72 | Superseded by the target lockfile's `0.10.79`. |
| #1, elliptic 6.6.1 | Targets the old `cumulus/scripts/scale_encode_genesis` path, absent from the target tree. |

## Validation

The upstream workspace does not pin a root Rust toolchain. Its CI references
Rust 1.93.0, which is the selected local toolchain for the scoped checks.
With isolated Cargo home and target directories, the following passed on
macOS arm64 with Rust 1.93.0:

- `cargo update -p tracing-subscriber@0.3.20 --precise 0.3.20` regenerated the
  lockfile. Cargo also selected four Windows dependency edges; no source
  override was added for them.
- `cargo metadata --locked --offline --no-deps --format-version 1` accepted the
  merged workspace manifest and lockfile.
- `cargo test --locked -p sp-statement-store -p sc-network-statement -p
  sc-statement-store -p pallet-statement --lib` passed: 15 primitives, 49
  network, and 53 store tests. The pallet compiled and has zero library tests.
- `git diff --check` and `git diff --cached --check` found no whitespace errors.

The generated test-runtime build warned that it could not locate the root
`Cargo.lock` through its temporary build directory; the scoped test command
completed successfully. Full-node and chain-dev runtime interoperability
remain downstream integration gates.
