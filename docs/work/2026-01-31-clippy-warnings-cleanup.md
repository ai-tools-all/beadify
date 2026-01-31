# Clippy Warnings Cleanup

**Date:** January 31, 2026

## Summary

Systematically reduced clippy warnings from 33+ down to 6 remaining through 5 iterations of targeted fixes.

## Commits

1. **c9316a9** - `fix(clippy): remove unnecessary as_bytes, writeln, and unused imports - iteration 1`
   - Fixed `needless_as_bytes` in log.rs (2 occurrences)
   - Replaced `write!(file, "\n")` with `writeln!(file)` in repo.rs
   - Removed unused `std::str::FromStr` import in enums.rs

2. **88702f6** - `fix(clippy): replace vec! with arrays and fix mutable references - iteration 2`
   - Replaced `vec![...]` with array syntax in error.rs (3 occurrences)
   - Fixed `unnecessary_mut_passed` in db.rs test code (4 calls)
   - Fixed doc comment formatting in issue/update.rs

3. **84d637c** - `fix(clippy): remove unnecessary mutable references and use and_then - iteration 3`
   - Removed unnecessary `mut` from transaction variables in db.rs tests (4 occurrences)
   - Replaced `map().flatten()` with `and_then()` in issue/create.rs

4. **e1ec86b** - `fix(clippy): initialize structs in constructors instead of field reassignment - iteration 4`
   - Converted field reassignments to struct initialization in issue/create.rs
   - Converted field reassignments to struct initialization in issue/update.rs
   - Converted field reassignments to struct initialization in update.rs

5. **c5fd506** - `fix(clippy): remove empty format string literals and combine with text - iteration 5`
   - Fixed 11 println! calls removing empty `{}` format strings in doc.rs, issue/list.rs, and list.rs
   - Consolidated format placeholders by combining literal text with format args

## Results

- **Before:** 33+ clippy warnings
- **After:** 6 warnings (all "too many arguments" function warnings, which are design decisions)
- **Coverage:** beads-core and beads crates
- **Type Distribution:**
  - Code style: 17 fixes
  - Function signatures: 4 fixes
  - Format strings: 11 fixes

## Remaining Warnings

The 6 remaining warnings are "too many arguments" in functions:
- `issue/create.rs:26` - 9 args
- `issue/list.rs:194` - 10 args
- `issue/update.rs:25` - 10 args
- `list.rs:175` - 8 args

These represent valid design decisions and would require refactoring with structs or builders, which is outside scope of code cleanup.

## Methodology

Each iteration:
1. Ran `cargo clippy --all-targets --all-features`
2. Identified top 3 highest-impact warnings
3. Applied fixes
4. Committed with descriptive message

This incremental approach kept each commit focused and reviewable.
