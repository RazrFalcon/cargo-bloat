# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]
### Added
- Forward compiler errors when building a crate.

### Fixed
- Run `cargo clean` after `cargo bloat --time` to prevent `RUSTC_WRAPPER` caching.

## [0.10.1] - 2021-07-03
### Added
- `dylib` support and the `--lib` flag thanks to [@bjorn3](https://github.com/bjorn3).
- `--symbols-section` argument to select a custom ELF symbols section.

### Changed
- `regex` is enabled by default.

### Fixed
- Improve error processing.

## [0.10.0] - 2020-09-08
### Changed
- The binary parsing code has been moved to the
  [binfarce](https://github.com/Shnatsel/binfarce) crate
  thanks to [@Shnatsel](https://github.com/Shnatsel).

## [0.9.3] - 2020-04-06
### Changed
- Update dependencies.

## [0.9.2] - 2020-01-10
### Added
- `--manifest-path` and `-v` flags.

## [0.9.1] - 2019-11-09
### Changed
- Use an actual `.text` section size instead of calculating it from symbols.
- Remove duplicated methods that have equal address.

## [0.9.0] - 2019-10-07
### Added
- Windows MinGW target support.
- `--no-relative-size` flag.

## [0.8.4] - 2019-10-02
### Added
- `--message-format table|json` option.
- Better `xargo` support by [@roblabla](https://github.com/roblabla)

### Changed
- `--crates` and `--time` cannot be used anymore.

## [0.8.3] - 2019-08-29
### Added
- **v0** mangling scheme support.

### Changed
- The `-n` flag affects the crates output too again.

## [0.8.2] - 2019-08-07
### Added
- `--package` support.

## [0.8.1] - 2019-07-22
### Changed
- Enable ELF support on macOS.

### Fixed
- ELF parsing.

## [0.8.0] - 2019-07-21
### Added
- Custom ELF and Mach-O parsers.

### Changed
- Filtering using `regex` is optional now. You can enable it via `--features regex-filter`.
- Use `json` instead of `serde_json`.
- Use `pico-args` instead of `structopt`.

### Removed
- `object` dependency.
- `goblin` dependency.

## [0.7.2] - 2019-07-13
### Added
- `--test` support.

### Changed
- The `[N Others]` row moved to the bottom and contains a more detailed text now.
- Allow to combine `--no-default-features` with a custom feature set.

## [0.7.1] - 2019-06-20
### Changed
- Revert older version of `object`, which works correctly on macOS.
- `--time --release` will remove only release artifacts now.

## [0.7.0] - 2019-06-04
### Added
- `--time` flag.

### Changed
- The `-n` flag affects only the functions table now.

## [0.6.4] - 2019-05-13
### Fixed
- Cargo.lock

## [0.6.3] - 2019-05-06
### Changed
- Dependencies updated.

## [0.6.2] - 2018-11-20
### Added
- Better error messages.

## [0.6.1] - 2018-11-02
### Fixed
- `cargo` output parsing.
- Double quoted paths in error messages.

## [0.6.0] - 2018-09-07
### Changed
- From now not all libraries from `%crate%/target/%mode%/deps/` will be processed,
  but only one that was used during the building.
- Better error processing.

### Removed
- `cargo` dependency.

## [0.5.2] - 2018-07-31
### Changed
- Cargo updated to v0.28.

## [0.5.1] - 2018-06-27
### Added
- Print path to the analyzed binary.

### Fixed
- Filter `cdylib` libraries by the `TargetKind` and not by the file extension.
- *Others* size.
- Rows count specified by the `-n` flag.

## [0.5.0] - 2018-05-29
### Added
- An ability to filter by regexp.
- A *filtered data size* row into the main table during filtering.

### Changed
- Cargo updated to v0.27.
- The *Others* row will show a number of functions that was filtered but not shown.
  Previously, it was showing the total functions amount.

## [0.4.0] - 2018-05-10
### Added
- A *Crate* column to the main table.
- A better crates resolving algorithm.

### Changed
- Remove std crates from the `std` group that was explicitly added as dependencies.
- Cargo updated to v0.26.

### Fixed
- The `--filter` flag behavior.

### Removed
- The `--print-unknown` flag.

## [0.3.0] - 2018-04-02
### Changed
- Cargo update to v0.25.
- All Unix-based OS'es are allowed now.

## [0.2.2] - 2018-02-18
### Added
- A warning to the `--crates` output.

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

[Unreleased]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.10.1...HEAD
[0.10.1]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.10.0...v0.10.1
[0.10.0]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.9.3...v0.10.0
[0.9.3]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.9.2...v0.9.3
[0.9.2]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.9.1...v0.9.2
[0.9.1]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.8.4...v0.9.0
[0.8.4]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.8.3...v0.8.4
[0.8.3]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.8.2...v0.8.3
[0.8.2]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.8.1...v0.8.2
[0.8.1]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.7.2...v0.8.0
[0.7.2]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.6.4...v0.7.0
[0.6.4]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.6.3...v0.6.4
[0.6.3]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.6.2...v0.6.3
[0.6.2]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.6.1...v0.6.2
[0.6.1]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.5.2...v0.6.0
[0.5.2]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/RazrFalcon/cargo-bloat/compare/v0.1.0...v0.2.0
