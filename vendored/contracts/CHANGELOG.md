 # Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## [0.6.3] - 2022-03-13

### Fixed

- improved hygiene around `self` parameters
- fix contract messages containing `{}` emitting warnings as they are interpreted as format strings

## [0.6.2] - 2021-07-21

### Changed

- better handling of mutable borrows and lifetime relationships for functions
  with contracts

## [0.6.1] - 2021-07-13

### Added

- support for `impl Trait` return types

## [0.6.0] - 2020-09-05
### Changed
- `pre` is now `requires`
- `post` is now `ensures`

## [0.5.2] - 2020-09-05
### Fixed
- Unused braces in function body generated code are removed

## [0.5.1] - 2020-08-06
### Changed
- Trait methods now handle attributes better.

## [0.5.0] - 2020-08-06
### Changed
- Implication operator is now `->`.

## [0.4.0] - 2020-05-01
### Added
- Added support for MIRAI assertions
- Added implication operator

## [0.3.0] - 2019-07-20
### Added
- Pseudo-function `old(expr)` which in a post-condition evaluates the expression before function execution.
- Automatic generation of documentation containing all contracts.

## [0.2.2] - 2019-07-17
### Fixed
- Errors inside functions/methods are now properly reported with the correct source location.
### Changed
- internal handling of contracts is now done in a single proc-macro pass instead of one for each contract.

## [0.2.1] - 2019-06-07
### Fixed
- Functions/methods with explicit return statements no longer skip `post` conditions

## [0.2.0] - 2014-04-12
### Added
- `contract_trait` attribute to make all implementors of a trait respect contracts.

## [0.1.1] - 2019-04-08
### Added
- Feature flags to override contract behavior.
  - `disable_contracts` ignores all checks
  - `override_debug` only checks contracts in debug configurations.
  - `override_log` only prints using the `log`-crate interface.

## [0.1.0] - 2019-04-06
### Added
- attributes `pre`/`post`/`invariant` and `debug_` versions of each.












