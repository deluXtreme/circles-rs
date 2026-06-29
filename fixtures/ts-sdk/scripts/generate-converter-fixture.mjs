#!/usr/bin/env node
// Generate converter-demurrage-inflation.json from the official TypeScript SDK
// CirclesConverter formulas at:
// https://github.com/aboutcircles/circles-sdk/blob/bdd94bd1f771335d8e678e823705a35dcac840cf/packages/utils/src/circlesConverter.ts
//
// This script is intentionally dependency-free so fixture regeneration does not
// require installing the full TypeScript workspace. Keep constants and formulas
// byte-for-byte aligned with the source file and update the source commit below
// whenever refreshing the fixture from a newer SDK revision.

const SOURCE = {
  repo: 'https://github.com/aboutcircles/circles-sdk',
  commit: 'bdd94bd1f771335d8e678e823705a35dcac840cf',
  package: '@circles-sdk/utils',
  version: '0.29.2',
  source_file: 'packages/utils/src/circlesConverter.ts',
};

const ONE_36 = 1_000_000_000_000_000_000_000_000_000_000_000_000n;
const GAMMA_36 = 999_801_332_008_598_957_430_613_406_568_191_166n;
const BETA_36 = 1_000_198_707_468_214_629_156_271_489_013_303_962n;
const SECONDS_PER_DAY = 86_400n;
const INFLATION_DAY_ZERO_UNIX = 1_602_720_000n;
const ATTO_FACTOR = 1_000_000_000_000_000_000n;
const V1_ACCURACY = 100_000_000n;
const V1_INFLATION_PCT_NUM = 107n;
const V1_INFLATION_PCT_DEN = 100n;
const PERIOD_SEC = 31_556_952n;

function mul36(a, b) {
  return (a * b) / ONE_36;
}

function pow36(base36, exp) {
  let result = ONE_36;
  let base = base36;
  let e = exp;

  while (e > 0n) {
    if ((e & 1n) === 1n) {
      result = mul36(result, base);
    }
    base = mul36(base, base);
    e >>= 1n;
  }
  return result;
}

function dayFromTimestamp(unixSeconds) {
  return (unixSeconds - INFLATION_DAY_ZERO_UNIX) / SECONDS_PER_DAY;
}

function attoCirclesToCircles(atto) {
  if (atto === 0n) return 0;

  const whole = atto / ATTO_FACTOR;
  const frac = atto % ATTO_FACTOR;
  const maxSafeInt = BigInt(Number.MAX_SAFE_INTEGER);
  if (whole > maxSafeInt || whole < -maxSafeInt) {
    throw new RangeError('Atto value’s integer component exceeds JS double precision.');
  }

  return Number(whole) + Number(frac) / Number(ATTO_FACTOR);
}

function inflationaryToDemurrageExact(inflationary, day) {
  const factor = pow36(GAMMA_36, day);
  return (inflationary * factor) / ONE_36;
}

function demurrageToInflationaryExact(demurraged, day) {
  const factor = pow36(BETA_36, day);
  return (demurraged * factor) / ONE_36;
}

function attoCirclesToAttoStaticCirclesExact(demurraged, nowUnixSeconds) {
  return demurrageToInflationaryExact(demurraged, dayFromTimestamp(nowUnixSeconds));
}

function attoStaticCirclesToAttoCirclesExact(staticCircles, nowUnixSeconds) {
  return inflationaryToDemurrageExact(staticCircles, dayFromTimestamp(nowUnixSeconds));
}

function v1InflateFactor(periodIdx) {
  if (periodIdx === 0n) return V1_ACCURACY;
  return (V1_ACCURACY * V1_INFLATION_PCT_NUM ** periodIdx) / (V1_INFLATION_PCT_DEN ** periodIdx);
}

function attoCirclesToAttoCrc(demurraged, blockTimestampUtc) {
  const secondsSinceEpoch = blockTimestampUtc - INFLATION_DAY_ZERO_UNIX;
  const periodIdx = secondsSinceEpoch / PERIOD_SEC;
  const secondsIntoPeriod = secondsSinceEpoch % PERIOD_SEC;
  const factorCur = v1InflateFactor(periodIdx);
  const factorNext = v1InflateFactor(periodIdx + 1n);
  const rP = factorCur * (PERIOD_SEC - secondsIntoPeriod) + factorNext * secondsIntoPeriod;
  return (demurraged * 3n * V1_ACCURACY * PERIOD_SEC) / rP;
}

const inputs = [
  {
    name: 'epoch-day-zero-one-circle',
    timestamp: 1_602_720_000n,
    demurraged_atto_circles: 1_000_000_000_000_000_000n,
  },
  {
    name: 'nov-2023-one-circle',
    timestamp: 1_700_000_000n,
    demurraged_atto_circles: 1_000_000_000_000_000_000n,
  },
  {
    name: 'nov-2023-fractional-six-decimals',
    timestamp: 1_700_000_000n,
    demurraged_atto_circles: 1_234_567_890_000_000_000n,
  },
  {
    name: 'jan-2026-large',
    timestamp: 1_767_225_600n,
    demurraged_atto_circles: 42_000_000_000_000_000_000_000n,
  },
];

const cases = inputs.map((input) => {
  const staticAttoCircles = attoCirclesToAttoStaticCirclesExact(
    input.demurraged_atto_circles,
    input.timestamp
  );

  return {
    name: input.name,
    timestamp: Number(input.timestamp),
    demurraged_atto_circles: input.demurraged_atto_circles.toString(),
    static_atto_circles: staticAttoCircles.toString(),
    roundtrip_demurraged_atto_circles: attoStaticCirclesToAttoCirclesExact(
      staticAttoCircles,
      input.timestamp
    ).toString(),
    v1_crc_atto: attoCirclesToAttoCrc(
      input.demurraged_atto_circles,
      input.timestamp
    ).toString(),
    ui_circles: attoCirclesToCircles(input.demurraged_atto_circles),
  };
});

console.log(JSON.stringify({
  source: SOURCE,
  generated_by: 'node fixtures/ts-sdk/scripts/generate-converter-fixture.mjs',
  normalization: [
    'BigInt outputs serialized as base-10 strings',
    'Uses exact 1e36 CirclesConverter methods for demurrage/static conversion',
  ],
  cases,
}, null, 2));
