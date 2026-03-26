use alloy_primitives::U256;
use num_bigint::BigUint;
use num_traits::{ToPrimitive, Zero};

/// Demurrage/inflation conversion helpers ported from the TS CirclesConverter.
/// We use 1e36-scaled factors (GAMMA_36/BETA_36) to avoid overflow and match
/// the exact math in the reference implementation.
fn gamma_36() -> BigUint {
    // 0.93^(1/365.25) scaled to 1e36 (rounded half-up)
    BigUint::parse_bytes(b"999801332008598957430613406568191166", 10).unwrap()
}

fn beta_36() -> BigUint {
    // 1 / GAMMA scaled to 1e36 (rounded half-up)
    BigUint::parse_bytes(b"1000198707468214629156271489013303962", 10).unwrap()
}

fn one_36() -> BigUint {
    // 1e36
    BigUint::parse_bytes(b"1000000000000000000000000000000000000", 10).unwrap()
}

const SECONDS_PER_DAY: u64 = 86_400;
const INFLATION_DAY_ZERO_UNIX: u64 = 1_602_720_000; // 2020-10-15 00:00:00 UTC
const V1_ACCURACY: u64 = 100_000_000; // 1e8
const V1_INFLATION_PCT_NUM: u64 = 107;
const V1_INFLATION_PCT_DEN: u64 = 100;
const PERIOD_SEC: u64 = 31_556_952; // 365.25 days

/// UNIX timestamp (seconds) → Circles day index (can be negative if before day zero).
pub fn day_from_timestamp(unix_seconds: u64) -> i64 {
    let secs = unix_seconds as i64;
    let zero = INFLATION_DAY_ZERO_UNIX as i64;
    (secs - zero) / SECONDS_PER_DAY as i64
}

fn pow36(base: &BigUint, exp: u64) -> BigUint {
    let mut result = one_36();
    let mut b = base.clone();
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            result = (result * &b) / one_36();
        }
        b = (&b * &b) / one_36();
        e >>= 1;
    }
    result
}

fn u256_to_big(val: U256) -> BigUint {
    let limbs = val.into_limbs();
    let mut out = BigUint::zero();
    for (i, limb) in limbs.iter().enumerate() {
        out += BigUint::from(*limb) << (64 * i);
    }
    out
}

fn big_to_u256(val: BigUint) -> U256 {
    let mut temp = val;
    let mut limbs = [0u64; 4];
    for limb_ref in &mut limbs {
        let limb = &temp & BigUint::from(u64::MAX);
        *limb_ref = limb.to_u64_digits().first().copied().unwrap_or(0);
        temp >>= 64;
    }
    U256::from_limbs(limbs)
}

fn v1_inflate_factor(period_idx: u64) -> BigUint {
    if period_idx == 0 {
        return BigUint::from(V1_ACCURACY);
    }
    let num = BigUint::from(V1_INFLATION_PCT_NUM).pow(period_idx as u32);
    let den = BigUint::from(V1_INFLATION_PCT_DEN).pow(period_idx as u32);
    (BigUint::from(V1_ACCURACY) * num) / den
}

/// Demurraged atto-circles → static atto-circles for a given timestamp.
///
/// If `now_unix_seconds` is `None`, uses the current time. Negative day indices
/// (pre-epoch) are returned unchanged.
pub fn atto_circles_to_atto_static_circles(
    demurraged: U256,
    now_unix_seconds: Option<u64>,
) -> U256 {
    let day = day_from_timestamp(now_unix_seconds.unwrap_or_else(now_ts));
    if day < 0 {
        return demurraged;
    }
    let factor = pow36(&beta_36(), day as u64);
    let dem = u256_to_big(demurraged);
    let static_val = (dem * factor) / one_36();
    big_to_u256(static_val)
}

/// Static atto-circles → demurraged atto-circles for a given timestamp.
///
/// If `now_unix_seconds` is `None`, uses the current time. Negative day indices
/// (pre-epoch) are returned unchanged.
pub fn atto_static_circles_to_atto_circles(
    static_circles: U256,
    now_unix_seconds: Option<u64>,
) -> U256 {
    let day = day_from_timestamp(now_unix_seconds.unwrap_or_else(now_ts));
    if day < 0 {
        return static_circles;
    }
    let factor = pow36(&gamma_36(), day as u64);
    let infl = u256_to_big(static_circles);
    let dem = (infl * factor) / one_36();
    big_to_u256(dem)
}

/// Demurraged atto-circles → UI circles as `f64`.
pub fn atto_circles_to_circles(atto: U256) -> f64 {
    if atto.is_zero() {
        return 0.0;
    }

    let big = u256_to_big(atto);
    let scale = BigUint::from(1_000_000_000_000_000_000u64);
    let whole = &big / &scale;
    let frac = &big % &scale;

    whole.to_f64().unwrap_or(f64::INFINITY) + frac.to_f64().unwrap_or(0.0) / 1e18
}

/// Demurraged atto-circles → inflationary CRC amount at the provided timestamp.
pub fn atto_circles_to_atto_crc(demurraged: U256, block_timestamp_utc: u64) -> U256 {
    let seconds_since_epoch = block_timestamp_utc.saturating_sub(INFLATION_DAY_ZERO_UNIX);
    let period_idx = seconds_since_epoch / PERIOD_SEC;
    let seconds_into_period = seconds_since_epoch % PERIOD_SEC;

    let factor_cur = v1_inflate_factor(period_idx);
    let factor_next = v1_inflate_factor(period_idx + 1);

    let period = BigUint::from(PERIOD_SEC);
    let seconds_into = BigUint::from(seconds_into_period);
    let r_p = (&factor_cur * (BigUint::from(PERIOD_SEC) - &seconds_into))
        + (&factor_next * &seconds_into);

    let numerator =
        u256_to_big(demurraged) * BigUint::from(3u64) * BigUint::from(V1_ACCURACY) * &period;
    big_to_u256(numerator / r_p)
}

fn now_ts() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_index_zero() {
        assert_eq!(day_from_timestamp(INFLATION_DAY_ZERO_UNIX), 0);
    }

    #[test]
    fn matches_ts_fixture() {
        // TS CirclesConverter: attoCirclesToAttoStaticCircles(1e18, ts=1700000000) -> 1250475269390674654
        let ts = 1_700_000_000u64;
        let dem = U256::from(1_000_000_000_000_000_000u64);
        let static_val = atto_circles_to_atto_static_circles(dem, Some(ts));
        let expected = U256::from(1_250_475_269_390_674_654u64);
        let diff = if static_val > expected {
            static_val - expected
        } else {
            expected - static_val
        };
        assert!(diff < U256::from(1_000u64));
        let back = atto_static_circles_to_atto_circles(static_val, Some(ts));
        let back_diff = if back > dem { back - dem } else { dem - back };
        assert!(back_diff < U256::from(2u64));
    }
}
