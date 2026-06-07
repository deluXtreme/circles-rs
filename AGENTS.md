# AGENTS.md

Guidance for AI agents and contributors working in this repository.

## Project purpose

`circles-rs` is a Rust workspace that ports the non-web/backend-relevant parts of the official Circles TypeScript SDK while keeping the Rust API idiomatic. The workspace is alpha-quality: preserve compatibility where possible, but expect API refinement while parity work continues.

The current application-level entrypoint is `circles-sdk`; lower-level crates remain usable for direct RPC, pathfinding, transfer planning, utilities, shared types, and generated ABIs.

## Crate layout

- `crates/sdk` — high-level orchestrator, typed avatars, registration, profile integration, runners, invitation/referral helpers.
- `crates/rpc` — Circles JSON-RPC client, pagination, event parsing/subscriptions.
- `crates/pathfinder` — flow matrix/pathfinding helpers and contract-preparation utilities.
- `crates/transfers` — transfer/replenish/group-token transaction planning.
- `crates/profiles` — profile service client.
- `crates/types` — shared response/config/domain types.
- `crates/utils` — demurrage/inflation/day-index conversion utilities.
- `crates/abis` — generated contract bindings.

## Source-of-truth references

For TypeScript SDK parity work, use these references:

- Official TypeScript SDK repo: <https://github.com/aboutcircles/circles-sdk>
- Rust repo milestone: <https://github.com/deluXtreme/circles-rs/milestone/1>
- Active parity issue set:
  - [#44 — V1 → V2 migration](https://github.com/deluXtreme/circles-rs/issues/44)
  - [#45 — ERC20/ERC1155 wrapper operations](https://github.com/deluXtreme/circles-rs/issues/45)
  - [#46 — profile service search](https://github.com/deluXtreme/circles-rs/issues/46)
  - [#47 — CMG/group-specific methods](https://github.com/deluXtreme/circles-rs/issues/47)
  - [#48 — TS-to-Rust convenience aliases and docs](https://github.com/deluXtreme/circles-rs/issues/48)
  - [#49 — SDK parity confidence suite](https://github.com/deluXtreme/circles-rs/issues/49)
  - [#50 — AGENTS.md and parity plans index](https://github.com/deluXtreme/circles-rs/issues/50)

When issue text and repo docs disagree, inspect current code and the linked TypeScript SDK before implementing. Update docs as part of parity PRs when the status changes.

## Scope rules for parity work

- Count backend/CLI/server functionality.
- Do not count browser-specific behavior unless an issue explicitly asks for it.
- Excluded by default:
  - browser provider execution
  - Safe App UI execution
  - frontend-only package behavior
- Included by default:
  - read-only RPC/data/profile behavior
  - pathfinder and flow-matrix behavior
  - transfer/replenish/wrapper planning
  - EOA and non-browser Safe runner behavior
  - transaction planning and calldata encoding
  - migration, wrapper, CMG/group, profile-search, and SDK ergonomic parity

## Development rules

### Prefer thin vertical PRs

Do not implement an entire epic in one PR. For example, migration parity should be split into configuration/ABI access, read-only eligibility, planning, execution, and docs/examples.

Each PR should:

- link an issue with `Closes #...` or `Refs #...`
- state the TypeScript SDK method(s) it covers
- state the Rust method(s) it adds or changes
- include tests or explicitly explain why tests are not applicable
- keep write execution as thin wrappers over planning helpers where possible

### Use test-driven development for behavior changes

For new behavior or bug fixes:

1. Add a failing test first.
2. Run the targeted test and verify it fails for the expected reason.
3. Implement the smallest change that passes.
4. Re-run the targeted test.
5. Run the relevant crate/workspace checks.

Documentation-only PRs do not need failing tests, but they should still run cheap formatting/inspection commands where practical.

### Keep write paths safe and plan-first

For transaction-writing parity work:

- Add planning helpers before execution helpers.
- Assert `PreparedTransaction` target, value, selector, and important calldata arguments.
- Add missing-runner tests for SDK methods that require a runner.
- Add mock-runner/pass-through tests for execution helpers.
- Never make live write tests run by default.

## Standard validation commands

Use the most specific command that proves your change, then run broader checks before opening a PR.

Common commands:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo doc --workspace --all-features --no-deps
```

For smaller iterations:

```bash
cargo test -p circles-sdk
cargo test -p circles-rpc
cargo test -p circles-pathfinder
cargo test -p circles-transfers
cargo test -p circles-profiles
```

If local OpenSSL development files are missing, `openssl-sys` may fail to build. In that case, note the local blocker in the PR/review and rely on GitHub Actions for full workspace verification.

## Live test policy

Live tests are opt-in only. They must skip cleanly unless the required environment variables are set.

Common live-test variables:

```bash
RUN_LIVE=1
LIVE_AVATAR=0x...
CIRCLES_RPC_URL=https://...
CIRCLES_CHAIN_RPC_URL=https://...
CIRCLES_PATHFINDER_URL=https://...
CIRCLES_PROFILE_URL=https://...
```

Profile-service live tests may also use:

```bash
RUN_LIVE_PROFILE_TESTS=1
PROFILE_CID=...
```

Rules:

- Read-only live tests may be ignored/gated.
- Write-capable live tests must be separately gated and clearly documented.
- Do not require secrets for default CI.
- Do not print private keys, bearer tokens, or sensitive wallet material.

## Plans

Repository-local implementation plans live under [`docs/plans/`](docs/plans/). GitHub issues remain the source of truth for active work and review status; checked-in plans should summarize, link, and preserve execution context rather than duplicate issue bodies wholesale.
