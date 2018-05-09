# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]
### Changed
- Remove std crates from the `std` group that was explicitly added as dependencies.
- Cargo update to v0.26.

## [0.3.0] - 2018-04-02
### Changed
- Cargo update to v0.25.
- All Unix-based OS'es are allowed now.

## [0.2.2] - 2018-02-18
### Added
- A warning to the `--creates` output.

### Changed
- `rustc-demangle` updated.

## [0.2.1] - 2018-01-23
### Added
- `--bin` flag.
- `--target` flag.

## [0.2.0] - 2018-01-18
### Added
- `C` symbols lookup in `rlib`'s. So `*-sys` crates are properly detected now.

### Changed
- Get a list of crate names by parsing `rlib` names in the `deps` dir
  and not by requesting the depended packages from cargo.
- When running on an unsupported OS you will get an error and not a random panic.
- The table has a dynamic column width now.

[Unreleased]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/RazrFalcon/cargo-bloat/compare/0.2.2...0.3.0
[0.2.2]: https://github.com/RazrFalcon/cargo-bloat/compare/0.2.1...0.2.2
[0.2.1]: https://github.com/RazrFalcon/cargo-bloat/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/RazrFalcon/cargo-bloat/compare/0.1.0...0.2.0
