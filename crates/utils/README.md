# circles-utils

Shared Circles utility functions, currently focused on demurrage/inflation conversions ported from the TypeScript `CirclesConverter`.

## Usage
```rust
use circles_utils::converter::{
    atto_circles_to_atto_static_circles,
    atto_static_circles_to_atto_circles,
    day_from_timestamp,
};
use alloy_primitives::U256;

let val = U256::from(1_000_000_000_000_000_000u128);
let static_val = atto_circles_to_atto_static_circles(val, None); // uses current time
let back = atto_static_circles_to_atto_circles(static_val, None);
assert_eq!(back, val);
let day = day_from_timestamp(1_602_720_000); // 0 (day zero)
```

## Notes
- Pure, synchronous math; no IO or async dependencies.
- Constants mirror the TS SDK: Gamma/Beta 64.64 factors, Circles day zero, 1e18 atto factor; tests tolerate tiny floating drift to match TS fixtures.
- Downstream crates (pathfinder, transfers) reuse these converters for wrapper/token handling.
