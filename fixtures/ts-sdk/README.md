# TypeScript SDK golden fixtures

This directory is reserved for curated fixtures generated from the official Circles TypeScript SDK and consumed by Rust tests.

The Rust workspace should not need to execute Node/TypeScript during normal CI. Instead, generate fixtures intentionally, commit them, and make Rust tests compare against the checked-in data.

## Fixture metadata

Every fixture or fixture group should document:

- TypeScript SDK repository URL
- TypeScript SDK commit SHA
- package name/version, if relevant
- generation command or script
- input cases
- normalization rules, if any

Recommended metadata shape:

```json
{
  "source": {
    "repo": "https://github.com/aboutcircles/circles-sdk",
    "commit": "<sha>",
    "package": "@circles-sdk/sdk",
    "version": "<version>"
  },
  "generated_by": "<script or command>",
  "normalization": [
    "lowercase Ethereum addresses",
    "sort arrays by stable key before comparison"
  ],
  "cases": []
}
```

## Current fixtures

| Fixture | Source | Rust comparison test | Covers |
| --- | --- | --- | --- |
| [`converter-demurrage-inflation.json`](converter-demurrage-inflation.json) | `@circles-sdk/utils` `CirclesConverter` at `bdd94bd1f771335d8e678e823705a35dcac840cf` | `cargo test -p circles-utils matches_ts_golden_converter_fixture` | demurraged/static conversion, V1 CRC conversion, UI circle conversion |

Regenerate the converter fixture with:

```bash
node fixtures/ts-sdk/scripts/generate-converter-fixture.mjs > fixtures/ts-sdk/converter-demurrage-inflation.json
```

## Good next fixture candidates

- pathfinder packing and flow-matrix transformations
- wrapped-token total helpers
- transfer/replenish transaction plan shapes
- V1 → V2 migration transaction plan shapes
- profile service request/response shapes
- ABI calldata selectors and important encoded arguments

## Comparison rules

Prefer exact equality when outputs are deterministic.

Use normalization only for differences that are semantically irrelevant, for example:

- address casing
- object field ordering
- stable sorting of unordered arrays

Do not normalize away semantic differences such as amount changes, missing vertices, missing transaction calls, different calldata selectors, or different target addresses.

## CI policy

- Rust tests may read these fixtures in default CI.
- Fixture generation scripts should not run in default CI unless the repository explicitly opts into a Node/TypeScript validation job.
- Network-dependent fixture generation should be documented and reproducible, but not required for normal Rust CI.
