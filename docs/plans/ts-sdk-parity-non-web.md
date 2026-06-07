# TypeScript SDK parity: non-web

Roadmap index for backend/CLI/server parity with the official Circles TypeScript SDK.

- Milestone: [TypeScript SDK parity: non-web](https://github.com/deluXtreme/circles-rs/milestone/1)
- TypeScript SDK reference: <https://github.com/aboutcircles/circles-sdk>
- Rust SDK repo: <https://github.com/deluXtreme/circles-rs>

## Scope

Included:

- RPC/data reads
- profile service behavior
- pathfinder and flow-matrix behavior
- transfer/replenish/group-token planning
- wrapper operation planning/execution
- EOA and non-browser Safe runner behavior
- V1 → V2 migration behavior
- CMG/group-specific backend methods
- TS-to-Rust ergonomic aliases where they wrap existing behavior honestly

Excluded unless a specific issue says otherwise:

- browser-provider execution
- Safe App UI execution
- frontend-only package behavior

## Issues

| Issue | Area | Priority | Status label | Notes |
| --- | --- | --- | --- | --- |
| [#44](https://github.com/deluXtreme/circles-rs/issues/44) | V1 → V2 migration | High | `status:ready` | Largest missing non-web feature. Start with config/ABI access, then read-only eligibility, then planning, then execution. |
| [#45](https://github.com/deluXtreme/circles-rs/issues/45) | ERC20/ERC1155 wrapper operations | High | `status:ready` | Add plan-first wrapper operations and direct token transfer helpers. |
| [#46](https://github.com/deluXtreme/circles-rs/issues/46) | Profile service search | Medium | `status:backlog` | Fill `circles-profiles` search/get-many parity. |
| [#47](https://github.com/deluXtreme/circles-rs/issues/47) | CMG/group-specific methods | Medium | `status:backlog` | Decide type shape, then add read/write planning/execution helpers. |
| [#48](https://github.com/deluXtreme/circles-rs/issues/48) | Convenience aliases and docs | Medium | `status:backlog` | Add honest TS-to-Rust mapping and thin aliases only where semantics match. |
| [#49](https://github.com/deluXtreme/circles-rs/issues/49) | SDK parity confidence suite | Medium | `status:backlog` | Add validation policy, mock runner helpers, golden fixture conventions, and coverage visibility. |
| [#50](https://github.com/deluXtreme/circles-rs/issues/50) | Repo hygiene | Medium | `status:ready` | Add `AGENTS.md` and this plans index. |

## Recommended sequence

1. Finish [#50](https://github.com/deluXtreme/circles-rs/issues/50) so agents and contributors have stable repo-local guidance.
2. Improve [#49](https://github.com/deluXtreme/circles-rs/issues/49) so future parity work has clear validation rules.
3. Start [#44](https://github.com/deluXtreme/circles-rs/issues/44) with migration config/ABI access only.
4. Add migration read-only eligibility (`can_self_migrate`).
5. Add migration planning helpers.
6. Add migration execution helpers as thin runner-backed wrappers.
7. Work through wrapper/profile/CMG parity in thin vertical PRs.

## PR checklist for parity work

```markdown
## TS parity

- [ ] TypeScript SDK method(s):
- [ ] Rust SDK method(s):
- [ ] Coverage status: Covered / Partial / Missing follow-up

## Tests

- [ ] Failing test observed before implementation, or docs-only PR
- [ ] Targeted tests pass
- [ ] Edge/error cases covered

## Write-capable behavior, if applicable

- [ ] Planning helper test asserts target/value/calldata selector
- [ ] Missing-runner behavior tested
- [ ] Mock-runner/pass-through execution tested
- [ ] Live write behavior is not run by default

## Commands

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --workspace`
- [ ] targeted `cargo test ...`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
```

## Notes for migration work

The migration epic should stay plan-first and safety-first:

1. expose config/ABI access without executing anything;
2. add read-only eligibility checks;
3. build transaction plans;
4. execute plans through existing runner abstractions only after planning behavior is reviewed.

This keeps Safe/EOA behavior reviewable and avoids coupling migration logic to a specific wallet transport.
