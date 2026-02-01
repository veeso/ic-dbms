# Changelog

- [Changelog](#changelog)
  - [0.4.0](#040)
  - [0.3.0](#030)
  - [0.2.1](#021)
  - [0.2.0](#020)
  - [0.1.0](#010)

## 0.4.0

Unreleased

- [Issue 13](https://github.com/veeso/ic-dbms/issues/13): Added JSON filtering capabilities for querying JSON columns.
  - `JsonFilter::Contains` for PostgreSQL `@>` style structural containment checks
  - `JsonFilter::Extract` for extracting values at JSON paths with comparison operations
  - `JsonFilter::HasKey` for checking path existence in JSON structures
  - Path syntax supports dot notation with bracket array indices (e.g., `user.items[0].name`)
- [Issue 22](https://github.com/veeso/ic-dbms/issues/22): Added `AgentClient` for the ic-dbms-canister to interact with
  the IC from an IC Agent.
- Fixed an issue with the IcCanisterClient which called `update` with the wrong amount of arguments.
- [Issue 12](https://github.com/veeso/ic-dbms/issues/12): Bump pocket-ic to 12.0.0.

## 0.3.0

Released on 2025-12-24

- [Field Sanitizers](https://github.com/veeso/ic-dbms/pull/7): it is now possible to tag fields for sanitization.
  Sanitizers can be specified in the schema and will be executed before inserting or updating records.
  - The library comes with built-in sanitizers for common use cases (e.g., trimming whitespace, converting to
    lowercase).
- [Memory Alignment](https://github.com/veeso/ic-dbms/pull/15): Changed the previous memory model which used to store
  records sequentially in a contiguous block of memory with padded fields to a more efficient model that aligns fields
  based on their data types. This change improves memory access speed and reduces fragmentation.
  - [Added a new `MemoryError::OffsetNotAligned`](https://github.com/veeso/ic-dbms/pull/16) variant to handle cases
    where field offsets are not properly aligned
    when writing, which notifies memory corruptions issues.
- [Int8, Int16, Uint8, Uint16 data types](https://github.com/veeso/ic-dbms/pull/17): Added support for smaller integer
  types to optimize memory usage
  and improve performance for applications that require precise control over data sizes.
- [Added `From` implementation for `Value` for inner types](https://github.com/veeso/ic-dbms/pull/18): `i8`, `i16`,
  `i32`, `i64`, `u8`, `u16`, `u32`, `u64`,
  `&[u8]`, `Vec<u8>`, `Principal`, `rust_decimal::Decimal`, `Uuid`, which
  automatically builds the corresponding `Value` variant when converting from these types.
  - Added `FromStr`, `From<&str>`, and `From<String>` implementations for `Value`, which automatically builds a
    `Value::Text`
    variant when converting from string types.
- [FreeSegmentLedger now uses many pages](https://github.com/veeso/ic-dbms/pull/20): The FreeSegmentLedger has been
  updated to utilize multiple pages for tracking free segments.
  This enhancement allows for the free segments ledger to grow and not to die when a single page is full.
  - Added logic to handle reading and writing free segments across multiple pages.
  - Updated tests to cover scenarios involving multiple pages in the FreeSegmentLedger.

## 0.2.1

Released on 2025-12-23

- TableReader never read following pages when reading a table. #5c0ffe6f

## 0.2.0

Released on 2025-12-21

- [Field Validation](https://github.com/veeso/ic-dbms/pull/6): it is now possible to tag fields for validation.
  Validators can be specified in the schema and will be executed before inserting or updating records.
  - The library comes with built-in validators for common use cases (e.g., email, URL, number range).

## 0.1.0

Released on 2025-12-11

- First stable release.
