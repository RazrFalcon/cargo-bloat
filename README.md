## cargo-bloat

Find out what takes most of the space in your executable.

Inspired by [google/bloaty](https://github.com/google/bloaty).

**Note:** Linux and macOS only.

### Install

```bash
cargo install cargo-bloat
```

or

```bash
cargo install --force --git https://github.com/RazrFalcon/cargo-bloat.git
```

### Usage

Get a list of biggest functions in the release build:

```
% cargo bloat --release -n 10          
    Finished release [optimized] target(s) in 0.2 secs

  File  .text    Size Name
 38.0%  93.3%  4.1MiB [8671 Others]
  0.5%   1.3% 57.6KiB <regex::exec::ExecNoSync<'c> as regex::re_trait::RegularExpression>::read_captures_at
  0.3%   0.8% 36.3KiB regex_syntax::parser::Parser::parse_expr
  0.3%   0.7% 30.1KiB <cargo::core::resolver::encode::_IMPL_DESERIALIZE_FOR_EncodableResolve::<impl serde::de:...
  0.3%   0.7% 29.1KiB <cargo::util::toml::_IMPL_DESERIALIZE_FOR_TomlManifest::<impl serde::de::Deserialize<'de...
  0.2%   0.6% 27.2KiB cargo::util::toml::do_read_manifest
  0.2%   0.5% 24.0KiB globset::GlobSet::new
  0.2%   0.5% 23.7KiB cargo::core::resolver::encode::EncodableResolve::into_resolve
  0.2%   0.5% 23.6KiB cargo::ops::cargo_rustc::compile_targets
  0.2%   0.5% 23.0KiB <cargo::util::toml::_IMPL_DESERIALIZE_FOR_TomlProject::<impl serde::de::Deserialize<'de>...
  0.2%   0.5% 22.9KiB cargo::ops::cargo_rustc::job_queue::JobQueue::drain_the_queue
 40.8% 100.0%  4.3MiB .text section size, the file size is 10.7MiB
```

Get a list of biggest dependencies in the release build:
```
% cargo bloat --release --crates -n 10
    Finished release [optimized] target(s) in 0.2 secs
    
  File  .text     Size Name
 11.3%  27.8%   1.2MiB cargo
  8.4%  20.6% 918.1KiB std
  7.5%  18.3% 815.0KiB [Unknown]
  2.2%   5.4% 240.5KiB toml
  1.9%   4.7% 210.4KiB regex
  1.9%   4.7% 207.7KiB goblin
  1.1%   2.8% 124.7KiB serde_ignored
  1.0%   2.5% 110.8KiB regex_syntax
  0.9%   2.1%  95.2KiB serde_json
  0.8%   1.9%  84.5KiB docopt
 40.8% 100.0%   4.3MiB .text section size, the file size is 10.7MiB
```

Flags specific for `cargo-bloat`:
```
--crates                Per crate bloatedness
--filter CRATE          Filter functions by crate
--split-std             Split the 'std' crate to original crates like core, alloc, etc.
--full-fn               Print full function name with hash values
-n NUM                  Number of lines to show, 0 to show all [default: 20]
-w, --wide              Do not trim long function names
```

### Correctness

The results are not perfect since function names parsing is not perfect.
Also, all non-Rust methods are skipped during crates resolving which moves jemalloc
and any other C libraries to the `[Unknown]` section.

### License

*cargo-bloat* is licensed under the MIT.
