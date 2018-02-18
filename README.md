## cargo-bloat

Find out what takes most of the space in your executable.

Inspired by [google/bloaty](https://github.com/google/bloaty).

**Note:** Linux and macOS only.

### Install

```bash
cargo install cargo-bloat
```

### Usage

Get a list of biggest functions in the release build:

```
% cargo bloat --release -n 10
    Finished release [optimized] target(s) in 0.2 secs

 File  .text    Size Name
36.3%  95.0%  4.6MiB [12125 Others]
 0.4%   1.0% 50.4KiB <regex::exec::ExecNoSync<'c> as regex::re_trait::RegularExpression>::read_captures_at
 0.2%   0.6% 29.5KiB regex_syntax::parser::Parser::parse_expr
 0.2%   0.5% 26.2KiB <cargo::util::toml::_IMPL_DESERIALIZE_FOR_TomlManifest::<impl serde::de::Deserialize<'de> ...
 0.2%   0.4% 21.8KiB cargo::util::toml::targets::targets
 0.2%   0.4% 21.0KiB <cargo_bloat::_IMPL_DESERIALIZE_FOR_Flags::<impl serde::de::Deserialize<'de> for cargo_blo...
 0.2%   0.4% 20.9KiB <serde_ignored::Deserializer<'a, 'b, D, F> as serde::de::Deserializer<'de>>::deserialize_s...
 0.2%   0.4% 20.2KiB cargo::core::workspace::Workspace::new
 0.2%   0.4% 20.0KiB <toml::de::MapVisitor<'de, 'b> as serde::de::Deserializer<'de>>::deserialize_any
 0.1%   0.4% 19.4KiB <serde_ignored::Deserializer<'a, 'b, D, F> as serde::de::Deserializer<'de>>::deserialize_s...
 0.1%   0.4% 19.3KiB <cargo::util::toml::_IMPL_DESERIALIZE_FOR_TomlProject::<impl serde::de::Deserialize<'de> f...
38.2% 100.0%  4.8MiB .text section size, the file size is 12.7MiB
```

Get a list of biggest dependencies in the release build:
```
% cargo bloat --release --crates -n 10
    Finished release [optimized] target(s) in 0.2 secs

 File  .text     Size Name
14.4%  37.6%   1.8MiB std
 7.4%  19.4% 964.2KiB cargo
 2.5%   6.6% 325.2KiB [Unknown]
 2.4%   6.3% 313.5KiB toml
 2.3%   5.9% 293.2KiB libgit2_sys
 1.4%   3.7% 184.0KiB regex
 1.3%   3.4% 168.9KiB goblin
 1.2%   3.2% 159.6KiB serde_ignored
 0.9%   2.3% 113.2KiB serde_json
 0.8%   2.1% 105.7KiB regex_syntax
38.2% 100.0%   4.8MiB .text section size, the file size is 12.7MiB

Warning: numbers above are a result of guesswork.They are not 100% correct and never will be.
```

Flags specific for `cargo-bloat`:
```
--crates                Per crate bloatedness
--filter CRATE          Filter functions by crate
--split-std             Split the 'std' crate to original crates like core, alloc, etc.
--print-unknown         Print methods under the '[Unknown]' tag
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
