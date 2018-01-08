## cargo-bloat

Find out what takes most of the space in your executable.

Inspired by [google/bloaty](https://github.com/google/bloaty).

**Note:** Linux and macOS only.

### Install

```bash
cargo install --force --git https://github.com/RazrFalcon/cargo-bloat.git
```

### Usage

Get a list of biggest functions in the release build:

```
% cargo bloat --release --trim-fn -n 10
    Finished release [optimized] target(s) in 0.2 secs
 93.3%  4.1MiB [8664 Others]
  1.3% 57.6KiB <regex::exec::ExecNoSync<'c> as regex::re_trait::RegularExpression>::read_captures_at
  0.8% 36.3KiB regex_syntax::parser::Parser::parse_expr
  0.7% 30.1KiB <cargo::core::resolver::encode::_IMPL_DESERIALIZE_FOR_EncodableResolve::<impl serde::de::Deseria...
  0.7% 29.1KiB <cargo::util::toml::_IMPL_DESERIALIZE_FOR_TomlManifest::<impl serde::de::Deserialize<'de> for ca...
  0.6% 27.2KiB cargo::util::toml::do_read_manifest
  0.5% 24.0KiB globset::GlobSet::new
  0.5% 23.7KiB cargo::core::resolver::encode::EncodableResolve::into_resolve
  0.5% 23.6KiB cargo::ops::cargo_rustc::compile_targets
  0.5% 23.0KiB <cargo::util::toml::_IMPL_DESERIALIZE_FOR_TomlProject::<impl serde::de::Deserialize<'de> for car...
  0.5% 22.9KiB cargo::ops::cargo_rustc::job_queue::JobQueue::drain_the_queue
100.0%  4.3MiB Total
```

Get a list of biggest dependencies in the release build:
```
% cargo bloat --release --crates -n 10
    Finished release [optimized] target(s) in 0.2 secs
 27.8%   1.2MiB cargo
 20.7% 918.0KiB std
 18.3% 815.0KiB [Unknown]
  5.4% 240.5KiB toml
  4.7% 210.4KiB regex
  4.7% 207.7KiB goblin
  2.8% 124.7KiB serde_ignored
  2.5% 110.8KiB regex_syntax
  2.1%  95.2KiB serde_json
  1.9%  84.5KiB docopt
100.0%   4.3MiB Total
```

Flags specific for `cargo-bloat`:
```
--crates                Per crate bloatedness
--trim-fn               Trim hash values from function names
-n NUM                  Number of lines to show, 0 to show all [default: 20]
-w, --wide              Do not trim long function names
```

### Correctness

The results are not perfect since function names parsing is not perfect.
Also, all non-Rust methods are skipped during crates resolving which moves jemalloc
and any other C libraries to the `[Unknown]` section.

The *Total* section represents the size of `.text` section, not a whole binary.

### License

*cargo-bloat* is licensed under the MIT.
