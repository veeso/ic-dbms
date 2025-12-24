# Changelog

- [Changelog](#changelog)
    - [0.3.0](#030)
    - [0.2.1](#021)
    - [0.2.0](#020)
    - [0.1.0](#010)

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
- Added `FromStr`, `From<&str>`, and `From<String>` implementations for `Value`, which automatically builds a
  `Value::Text`
  variant when converting from string types.

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
