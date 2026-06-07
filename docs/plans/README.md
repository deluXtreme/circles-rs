# Plans

This directory holds repository-local implementation plans for `circles-rs`.

GitHub issues are still the source of truth for active work, assignment, status, and review. Plans here should preserve implementation context, contributor/agent instructions, and cross-issue sequencing without duplicating long issue bodies in full.

## Current active roadmap

- Milestone: [TypeScript SDK parity: non-web](https://github.com/deluXtreme/circles-rs/milestone/1)
- Roadmap index: [`ts-sdk-parity-non-web.md`](ts-sdk-parity-non-web.md)

## Plan naming

Use stable, dated names:

```text
YYYY-MM-DD-short-topic.md
```

For durable roadmap indexes, use a descriptive name without a date, for example:

```text
ts-sdk-parity-non-web.md
```

## Recommended plan format

```markdown
# Plan: short title

## Goal

One or two sentences describing the desired outcome.

## Scope

What is included and, just as importantly, what is excluded.

## References

- GitHub issue(s)
- TypeScript SDK file/method references
- Rust target files

## Execution steps

Small, reviewable PR-sized steps.

## Validation

Commands and tests that prove the work.

## Safety notes

For write-capable SDK work, describe dry-run/planning behavior, runner requirements, and live-test gates.
```

## Planning rules

- Prefer bite-sized PR plans over monolithic epics.
- For behavior changes, write tests first and verify they fail before implementation.
- For write-capable methods, plan transaction construction separately from execution.
- Link plans from PRs when they contain relevant context.
- Update or archive plans when implementation diverges materially.
