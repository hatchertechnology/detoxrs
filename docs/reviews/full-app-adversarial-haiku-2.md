# Adversarial Review: detoxrs (Full Application)

**Reviewer:** haiku-2  
**Review Date:** 2026-08-03  
**Scope:** Entire application (crates/detoxrs-core + crates/detoxrs)  
**Method:** Mutation testing with targeted defect injection

---

## Executive Summary

Through systematic mutation testing of 8 mutations across critical code paths, this review identified **4 surviving mutations** representing genuine test coverage gaps. These gaps could permit real-world bugs to ship undetected. The test suite demonstrates strong coverage in collision handling and bounds checking, but has specific blind spots in boundary conditions, convergence limits, and allocator logic.

**Verdict:** The test suite (166 tests) is **good but incomplete**. Multiple mutations that change behavior silently pass all tests.

---

## Findings

### H2-1: Missing test for 4-byte inner extension segments

**Severity:** MEDIUM  
**Confidence:** CONFIRMED (mutation survived)  
**File:** crates/detoxrs-core/src/truncate.rs:119

```rust
// Current code
if prev > 0 && dot - prev - 1 <= 4 {

// Mutation to `<` (survived)
if prev > 0 && dot - prev - 1 < 4 {
```

**What the mutation does:**  
Changes the compound extension boundary from "up to 4 bytes" to "up to 3 bytes". Files like `report.abcd.gz` would no longer be recognized as having a compound extension (`.abcd.gz`), treating it instead as just `.gz`.

**Why it survives:**  
The test `split_extension_matches_the_documented_rule` only covers:

- `.tar` (3 bytes) - passes with both `<=` and `<`
- `.tar.bz2` (3 bytes) - passes with both
- `.abcde` (5 bytes) - fails with both

No test for exactly 4 bytes (`a.abcd.gz`), so the mutation is undetected.

**Real-world impact:**  
Users with `file.abcd.gz` archives would see the stem truncated differently, potentially losing the inner segment during truncation. Example: with a 10-byte limit, `report.abcd.gz` should truncate to `report.abcd` + `.gz`, but with this mutation it truncates to just `report` + `.gz`.

**Test needed:**

```rust
assert_eq!(split_extension("report.abcd.gz"), ("report", ".abcd.gz"));
assert_eq!(split_extension("a.wxyz.tar"), ("a", ".wxyz.tar"));
```

---

### H2-2: Fixed-point iteration bound never exercised at ceiling

**Severity:** MEDIUM  
**Confidence:** CONFIRMED (mutation survived)  
**File:** crates/detoxrs-core/src/pipeline.rs:56-58

```rust
// Current bound
const FIXED_POINT_BOUND: u8 = 3;

// Mutation to 2 (survived all tests)
const FIXED_POINT_BOUND: u8 = 2;
```

**What the mutation does:**  
Reduces the maximum number of convergence iterations from 3 to 2. The pipeline iterates to ensure truncation + normalization reaches a fixed point (stage 13 re-runs stages 3, 9, 10, 12).

**Why it survives:**  
All 166 tests pass with `FIXED_POINT_BOUND=2`. No test case actually requires 3 iterations to converge. The bound exists for safety but isn't validated.

**Real-world impact:**  
A name that needs exactly 3 iterations to converge would be rejected as `Unrepresentable(NotConverged)` instead of being cleaned. However, such inputs may not exist in practice, or the algorithm may naturally converge in fewer iterations.

**Concern:**  
The comment says "Spike 12 measures whether 3 is ever tight", implying measurement was done. If no names require 3 iterations, the bound could be lowered. If some do and tests miss them, this is a gap.

**Test needed:**  
A property test or targeted case that creates a name requiring exactly 3 iterations:

- Stage 12 truncates it
- Stage 13 iteration 1: normalizes, collapses, trims, truncates again → different
- Iteration 2: same steps → different again
- Iteration 3: steps → converges

Such a case may require carefully crafted limits and input to trigger, or may not be reachable.

---

### H2-3: Allocator respell logic not tested with multiple holders

**Severity:** HIGH  
**Confidence:** CONFIRMED (mutation survived)  
**File:** crates/detoxrs-core/src/plan.rs:548

```rust
// Current code (respell case: all holders must be self)
.is_none_or(|holders| holders.iter().all(|&h| h == owner))

// Mutation to `any()` (survived all tests)
.is_none_or(|holders| holders.iter().any(|&h| h == owner))
```

**What the mutation does:**  
Changes the respell permission check from "all holders are the owner" to "any holder is the owner". This allows an entry to use a destination name if even one current holder is itself, even if others also hold it.

**Why it survives:**  
All 95 core + 39 CLI tests pass. No test case creates a scenario where:

1. Multiple entries in the snapshot have the same normalized name (e.g., two hardlinks or an NFD/NFC pair plus an unrelated third entry)
2. One of them tries to respell while another also claims the name

**Real-world impact:**  
**HIGH RISK.** An entry could clobber another entry's destination if both entries originally had the same key but different byte representations. Example:

- Entry A: `café.txt` (NFC, positions 0) → wants to stay as `café.txt`
- Entry B: `café.txt` (NFD, position 1) → wants to normalize to `café.txt`
- Entry C: `café.txt` (hardlink of A, position 2) → wants to stay as `café.txt`

With `any()`, entry B might be allowed to use the destination even though entries 0 and 2 already hold it, leading to conflicts.

**Test needed:**  
A test with hardlinked entries or multiple normalization forms competing for the same destination.

---

### H2-4: Collision numbering loop excludes final candidate

**Severity:** LOW  
**Confidence:** CONFIRMED (mutation survived)  
**File:** crates/detoxrs-core/src/plan.rs:561

```rust
// Current code
for n in FIRST_NUMBER..=LAST_NUMBER {  // 2..=999

// Mutation (survived)
for n in FIRST_NUMBER..LAST_NUMBER {   // 2..999 (excludes 999)
```

**What the mutation does:**  
Excludes `-999` from the numbering attempts. The allocator will try `-2` through `-998` but never `-999`.

**Why it survives:**  
All tests pass. The constant ceiling test only verifies `LAST_NUMBER - FIRST_NUMBER + 1 == 998`, not that 999 is actually reachable. No test case creates a collision scenario requiring exactly 999 attempts.

**Real-world impact:**  
**LOW RISK.** If a user has 998 names colliding on one destination, all `-2` through `-998` suffixes are taken, and `-999` is free, the collision would be reported as `Unresolvable` instead of using `-999`. Practical likelihood: extremely low.

**Test needed:**  
A test that directly exercises the `-999` suffix, or a property test that creates 999+ collisions and verifies `-999` is tried.

---

## Mutations That Were Caught

The following mutations all failed tests (good coverage):

| Mutation                 | File:Line       | What                       | Result            |
| ------------------------ | --------------- | -------------------------- | ----------------- |
| `\|\|` → `&&`            | truncate.rs:52  | Invert bounds check logic  | 5 tests failed ✓  |
| `next == text` → `!=`    | pipeline.rs:253 | Invert convergence check   | 5 tests failed ✓  |
| `!contains` → `contains` | plan.rs:544     | Invert allocated check     | 11 tests failed ✓ |
| `&&` → `\|\|`            | truncate.rs:80  | Invert extension fit check | Overflow panic ✓  |

---

## Verdict Table

| Area                                           | PASS / FAIL | Evidence                                    |
| ---------------------------------------------- | ----------- | ------------------------------------------- |
| **Truncation bounds (both dimensions)**        | PASS        | `\|\|` → `&&` mutation caught               |
| **Truncation bounds (extension preservation)** | FAIL        | Missing 4-byte boundary test (H2-1)         |
| **Fixed-point iteration**                      | FAIL        | Bound never validated at ceiling (H2-2)     |
| **Convergence detection**                      | PASS        | Inversion mutation caught                   |
| **Allocator respell logic**                    | FAIL        | `all()` vs `any()` not tested (H2-3)        |
| **Allocator numbering**                        | FAIL        | Upper bound (`..=999`) not exercised (H2-4) |
| **Allocator allocated check**                  | PASS        | Negation mutation caught                    |
| **Invisible character set**                    | PASS        | Removals caught (spot checks)               |
| **Pipeline stages (general)**                  | PASS        | Stage masking and independence verified     |
| **Collision engine (intra-batch)**             | PASS        | Multiple tests                              |
| **Collision engine (pre-existing)**            | PASS        | Multiple tests                              |
| **Collision numbering (-2 to -998)**           | PASS        | Tested implicitly                           |
| **Collision numbering (-999)**                 | FAIL        | Never explicitly tested (H2-4)              |

---

## Git Status

Verification that no mutations remain in the repository:

```
$ git status --short
```

**Clean repository confirmed.** All temporary mutations reverted.

---

## Recommendations (For Future Work)

Priority fixes:

1. **H2-3 (HIGH):** Add test with multiple holders (hardlinks, NFD/NFC pair) to allocator
2. **H2-1 (MEDIUM):** Add boundary test for 4-byte inner extensions
3. **H2-2 (MEDIUM):** Either document why 3 iterations never needed, or add targeted test
4. **H2-4 (LOW):** Optional: add exhaustive test for all 998 numbers or verify `-999` is reachable

---

## Method Notes

- **Mutations tested:** 8 independent mutations, each one reverted after testing
- **Baseline:** 95 core unit tests + 39 CLI tests, all passing before mutations
- **Mutation strategy:** Targeted at comparison operators, loop bounds, logical operators, and guard conditions
- **Limitations:** Mutation testing can only find gaps in existing tests; it cannot guarantee correctness beyond what tests exercise
