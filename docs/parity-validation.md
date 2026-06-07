# Parity validation

Validation policy for TypeScript SDK parity work in `circles-rs`.

This document is intentionally conservative: parity work should be easy to review, safe by default, and grounded in tests or fixtures that make TypeScript-to-Rust behavior differences visible.

## Goals

- Keep default CI deterministic and secret-free.
- Add explicit validation layers for non-web TypeScript SDK parity.
- Prefer planning/dry-run assertions before transaction execution.
- Make live checks opt-in and safe by default.
- Give contributors a standard PR checklist for parity work.

## Default CI validation

The current CI should continue to run the core Rust checks:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo build --all
```

For local or future CI expansion, prefer the workspace-explicit form:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace
cargo doc --workspace --all-features --no-deps
```

If local OpenSSL development files are missing, crates that depend on `openssl-sys` may fail to compile. In that case, document the local blocker and rely on GitHub Actions for full workspace verification.

## Validation layers

### Layer 1 — Unit tests

Use unit tests for deterministic local behavior:

- demurrage/inflation conversion
- day-index calculations
- config defaults
- URL construction
- serde request/response decoding
- path/flow math
- token aggregation
- error mapping

These should run in default CI and must not require network access or secrets.

### Layer 2 — Prepared transaction tests

Write-capable SDK behavior should be tested at the planning layer first.

For every new transaction-planning helper, assert:

- target address
- value
- calldata selector
- important calldata arguments
- ordering inside batches

Planning tests should not require a runner, private key, network access, or live chain state.

### Layer 3 — Runner behavior tests

For SDK methods that execute transactions through a `ContractRunner`, add tests for:

- `SdkError::MissingRunner` when no runner is configured
- execution methods passing planned transactions through unchanged
- batch order preservation
- error propagation from the runner

Prefer reusable mock-runner helpers over repeated ad hoc mocks.

### Layer 4 — TypeScript golden fixtures

Do not require Node/TypeScript execution in normal Rust CI. Instead, commit curated fixtures generated from the official TypeScript SDK and compare Rust output against them.

Recommended fixture areas:

- conversion/demurrage/inflation cases
- pathfinder packing and flow-matrix transformations
- wrapped-token total helpers
- transfer/replenish transaction plan shapes
- migration transaction plan shapes
- profile service request/response shapes
- ABI calldata selectors and arguments for planned writes

A fixture should record:

- TypeScript SDK repository URL
- TypeScript SDK commit
- package/version if relevant
- command or script used to generate it
- normalization rules, if any

See [`fixtures/ts-sdk/README.md`](../fixtures/ts-sdk/README.md).

### Layer 5 — Optional live read tests

Live read tests are useful but must be opt-in and ignored/gated by default.

Common environment variables:

```bash
RUN_LIVE=1
LIVE_AVATAR=0x...
CIRCLES_RPC_URL=https://...
CIRCLES_CHAIN_RPC_URL=https://...
CIRCLES_PATHFINDER_URL=https://...
CIRCLES_PROFILE_URL=https://...
```

Profile-service tests may also use:

```bash
RUN_LIVE_PROFILE_TESTS=1
PROFILE_CID=...
```

Rules:

- Live read tests should skip cleanly when variables are missing.
- Live read tests must not mutate chain or profile state unless explicitly documented.
- Default CI must not require live endpoints.

### Layer 6 — Optional live write tests

Live write tests are allowed only as explicit, separately gated tests.

Rules:

- Never run live writes by default.
- Require a dedicated environment flag beyond `RUN_LIVE=1`.
- Document expected side effects.
- Use test-only wallets/accounts.
- Never print private keys, bearer tokens, seed phrases, or wallet material.
- Prefer local/anvil tests before live writes.

## Coverage visibility

Coverage is useful as a signal, not an initial gate.

Recommended first step:

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --workspace --all-features --summary-only
```

Do not enforce a hard percentage until baseline coverage is known. Start by publishing or recording the summary so coverage gaps are visible.

## Parity PR checklist

Every parity PR should include a checklist like this:

```markdown
## TS parity

- [ ] TypeScript SDK method(s):
- [ ] Rust SDK method(s):
- [ ] Coverage status: Covered / Partial / Follow-up required
- [ ] Browser/web behavior excluded or explicitly scoped

## Tests

- [ ] Failing test observed before implementation, or docs-only PR
- [ ] Targeted tests pass
- [ ] Edge/error cases covered
- [ ] Golden fixture added/updated, or reason documented

## Write-capable behavior, if applicable

- [ ] Planning helper test asserts target/value/calldata selector
- [ ] Important calldata arguments tested
- [ ] Missing-runner behavior tested
- [ ] Mock-runner/pass-through execution tested
- [ ] Live write behavior is not run by default

## Commands

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --workspace`
- [ ] targeted `cargo test ...`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] CI green
```

## Validation expectations by work type

| Work type | Required validation |
| --- | --- |
| Docs only | `git diff --check`, optional `cargo fmt --all -- --check` |
| Pure logic | failing unit test, targeted test, workspace check where practical |
| RPC/profile decoding | fixture decode test and error/empty case |
| URL/query construction | unit test for encoded URL/body shape |
| Transaction planning | prepared transaction target/value/selector/arg assertions |
| Execution wrapper | missing-runner test plus mock-runner pass-through test |
| Live read helper | ignored/gated live test if deterministic fixture coverage is not enough |
| Live write helper | explicit opt-in live/anvil test plan; never default CI |

## Recommended first validation PRs

1. Add this policy document and fixture convention.
2. Add reusable prepared-transaction assertion helpers.
3. Add reusable mock-runner test helpers.
4. Add initial TypeScript golden fixtures for converter/pathfinder behavior.
5. Add coverage summary job or documented `cargo-llvm-cov` workflow.
