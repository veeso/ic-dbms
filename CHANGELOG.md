# Changelog

- [Changelog](#changelog)
  - [0.3.0](#030)
  - [0.2.0](#020)
  - [0.1.0](#010)

## 0.3.0

Released on ??

- [Field Sanitizers](https://github.com/veeso/ic-dbms/pull/7): it is now possible to tag fields for sanitization. Sanitizers can be specified in the schema and will be executed before inserting or updating records.
  - The library comes with built-in sanitizers for common use cases (e.g., trimming whitespace, converting to lowercase).

## 0.2.0

Released on 2025-12-21

- [Field Validation](https://github.com/veeso/ic-dbms/pull/6): it is now possible to tag fields for validation. Validators can be specified in the schema and will be executed before inserting or updating records.
  - The library comes with built-in validators for common use cases (e.g., email, URL, number range).

## 0.1.0

Released on 2025-12-11

- First stable release.
