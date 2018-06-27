## cargo-bloat

Find out what takes most of the space in your executable.

Inspired by [google/bloaty](https://github.com/google/bloaty).

**Note:** supports ELF (Linux, BSD) and Mach-O (macOS) platforms only.

### Install

```bash
cargo install cargo-bloat
```

### Usage

Get a list of the biggest functions in the release build:

```
% cargo bloat --release -n 10
    Finished release [optimized] target(s) in 0.2 secs
   Analyzing target/release/cargo-bloat

 File  .text    Size                 Crate Name
36.1%  95.2%  5.4MiB                       [14743 Others]
 0.3%   0.8% 48.8KiB          regex_syntax <regex_syntax::ast::parse::ParserI<'s, P>>::parse_wit...
 0.3%   0.7% 43.0KiB          regex_syntax <regex_syntax::ast::parse::ParserI<'s, P>>::parse_wit...
 0.2%   0.6% 34.2KiB unicode_normalization unicode_normalization::tables::compatibility_fully_de...
 0.2%   0.4% 26.2KiB unicode_normalization unicode_normalization::tables::canonical_fully_decomp...
 0.2%   0.4% 25.1KiB                  clap clap::app::parser::Parser::get_matches_with
 0.2%   0.4% 25.0KiB                 cargo cargo::core::resolver::activate_deps_loop
 0.1%   0.4% 20.4KiB                 toml? <toml::de::MapVisitor<'de, 'b> as serde::de::Deserial...
 0.1%   0.3% 20.2KiB                 cargo cargo::util::toml::targets::targets
 0.1%   0.3% 17.8KiB                 cargo <cargo::util::toml::_IMPL_DESERIALIZE_FOR_TomlManifes...
 0.1%   0.3% 17.6KiB                 cargo <cargo::util::toml::_IMPL_DESERIALIZE_FOR_TomlProject...
37.9% 100.0%  5.7MiB                       .text section size, the file size is 15.0MiB
```

Get a list of the biggest dependencies in the release build:
```
% cargo bloat --release --crates -n 10
    Finished release [optimized] target(s) in 0.2 secs
   Analyzing target/release/cargo-bloat

 File  .text     Size Name
10.4%  27.6%   1.6MiB cargo
 7.7%  20.4%   1.2MiB std
 2.6%   7.0% 406.0KiB regex_syntax
 2.5%   6.5% 380.2KiB toml
 2.2%   5.9% 342.3KiB [Unknown]
 2.1%   5.5% 320.0KiB libgit2_sys
 2.0%   5.3% 309.1KiB clap
 1.6%   4.3% 248.3KiB regex
 1.0%   2.7% 157.3KiB goblin
 0.7%   1.7% 101.7KiB serde_json
37.9% 100.0%   5.7MiB .text section size, the file size is 15.0MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

Get a list of the biggest functions in the release build filtered by the regexp:
```
% cargo bloat --release --filter '^__' -n 10
    Finished release [optimized] target(s) in 0.2 secs
   Analyzing target/release/cargo-bloat

File .text    Size         Crate Name
0.0%  0.0%    945B               [19 Others]
0.0%  0.1%  4.7KiB backtrace_sys __rbt_backtrace_dwarf_add
0.0%  0.1%  3.0KiB backtrace_sys __rbt_backtrace_qsort
0.0%  0.0%    565B backtrace_sys __rbt_backtrace_syminfo
0.0%  0.0%    565B backtrace_sys __rbt_backtrace_pcinfo
0.0%  0.0%    357B backtrace_sys __rbt_backtrace_initialize
0.0%  0.0%    219B backtrace_sys __rbt_backtrace_get_view
0.0%  0.0%    211B backtrace_sys __rbt_backtrace_vector_grow
0.0%  0.0%    197B backtrace_sys __rbt_backtrace_create_state
0.0%  0.0%    150B backtrace_sys __rbt_backtrace_open
0.0%  0.0%    143B           std __rust_start_panic
0.1%  0.2% 10.9KiB               filtered data size, the file size is 15.0MiB
```

Flags specific for the `cargo-bloat`:
```
        --crates                   Per crate bloatedness
        --filter <CRATE|REGEXP>    Filter functions by crate
        --split-std                Split the 'std' crate to original crates like core, alloc, etc.
        --full-fn                  Print full function name with hash values
    -n <NUM>                       Number of lines to show, 0 to show all [default: 20]
    -w, --wide                     Do not trim long function names
```

### License

*cargo-bloat* is licensed under the MIT.
