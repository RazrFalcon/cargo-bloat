## cargo-bloat

Find out what takes most of the space in your executable.

Inspired by [google/bloaty](https://github.com/google/bloaty).

**Note:** supports ELF (Linux, BSD) and Mach-O (macOS) platforms only.

### Install

```bash
cargo install cargo-bloat
```

### Usage

Get a list of biggest functions in the release build:

```
% cargo bloat --release -n 10
    Finished release [optimized] target(s) in 0.2 secs

 File  .text    Size                 Crate Name
36.5%  93.9%  5.0MiB                       [13502 Others]
 0.4%   1.0% 55.6KiB                 regex <regex::exec::ExecNoSync<'c> as regex::re_trait::Regu...
 0.4%   0.9% 49.0KiB unicode_normalization unicode_normalization::tables::compatibility_fully_de...
 0.3%   0.9% 48.6KiB          regex_syntax <regex_syntax::ast::parse::ParserI<'s, P>>::parse_wit...
 0.3%   0.8% 44.7KiB          regex_syntax <regex_syntax::ast::parse::ParserI<'s, P>>::parse_wit...
 0.3%   0.7% 38.1KiB unicode_normalization unicode_normalization::tables::canonical_fully_decomp...
 0.2%   0.4% 21.5KiB                 toml? <toml::de::MapVisitor<'de, 'b> as serde::de::Deserial...
 0.1%   0.4% 20.1KiB                 cargo cargo::core::resolver::activate_deps_loop
 0.1%   0.4% 20.1KiB                 cargo cargo::util::toml::targets::targets
 0.1%   0.3% 18.0KiB                 regex <regex::re_trait::Matches<'t, R> as core::iter::itera...
 0.1%   0.3% 17.7KiB                 cargo <cargo::util::toml::_IMPL_DESERIALIZE_FOR_TomlManifes...
38.8% 100.0%  5.3MiB                       .text section size, the file size is 13.6MiB
```

Get a list of biggest dependencies in the release build:
```
% cargo bloat --release --crates -n 10
    Finished release [optimized] target(s) in 0.2 secs

 File  .text     Size Name
11.0%  28.4%   1.5MiB cargo
 7.6%  19.5%   1.0MiB std
 3.0%   7.7% 420.2KiB regex_syntax
 2.8%   7.1% 385.0KiB toml
 2.5%   6.5% 351.0KiB regex
 2.2%   5.6% 301.5KiB [Unknown]
 2.0%   5.2% 281.3KiB libgit2_sys
 1.1%   2.9% 157.4KiB goblin
 0.8%   2.1% 112.6KiB serde_json
 0.8%   2.1% 111.7KiB docopt
38.8% 100.0%   5.3MiB .text section size, the file size is 13.6MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
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
