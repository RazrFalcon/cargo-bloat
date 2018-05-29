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

 File  .text    Size                 Crate Name
 0.0%   0.0%      0B                       [14819 Others]
 0.3%   0.8% 48.6KiB          regex_syntax <regex_syntax::ast::parse::ParserI<'s, P>>::parse_wit...
 0.3%   0.7% 43.8KiB          regex_syntax <regex_syntax::ast::parse::ParserI<'s, P>>::parse_wit...
 0.2%   0.6% 34.2KiB unicode_normalization unicode_normalization::tables::compatibility_fully_de...
 0.2%   0.4% 26.2KiB unicode_normalization unicode_normalization::tables::canonical_fully_decomp...
 0.2%   0.4% 24.6KiB                  clap clap::app::parser::Parser::get_matches_with
 0.2%   0.4% 24.5KiB                 cargo cargo::core::resolver::activate_deps_loop
 0.1%   0.4% 21.4KiB                 toml? <toml::de::MapVisitor<'de, 'b> as serde::de::Deserial...
 0.1%   0.3% 20.5KiB                 cargo cargo::util::toml::targets::targets
 0.1%   0.3% 17.9KiB                goblin <goblin::mach::load_command::CommandVariant as scroll...
 0.1%   0.3% 17.7KiB                 cargo <cargo::util::toml::_IMPL_DESERIALIZE_FOR_TomlManifes...
 0.1%   0.3% 17.6KiB                 cargo <cargo::util::toml::_IMPL_DESERIALIZE_FOR_TomlProject...
38.6% 100.0%  5.8MiB                       .text section size, the file size is 14.9MiB
```

Get a list of the biggest dependencies in the release build:
```
% cargo bloat --release --crates -n 10
    Finished release [optimized] target(s) in 0.2 secs

 File  .text     Size Name
10.8%  27.9%   1.6MiB cargo
 7.7%  20.1%   1.2MiB std
 2.8%   7.1% 420.5KiB regex_syntax
 2.5%   6.6% 386.5KiB toml
 2.3%   5.8% 344.1KiB [Unknown]
 2.0%   5.3% 310.8KiB libgit2_sys
 2.0%   5.1% 303.4KiB clap
 1.7%   4.4% 260.8KiB regex
 1.1%   2.8% 165.7KiB goblin
 0.6%   1.4%  84.1KiB serde_json
38.6% 100.0%   5.8MiB .text section size, the file size is 14.9MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

Get a list of the biggest functions in the release build filtered by the regexp:
```
% cargo bloat --release --filter '^__' -n 10
    Finished release [optimized] target(s) in 0.2 secs

File .text    Size         Crate Name
0.0%  0.0%      0B               [20 Others]
0.0%  0.1%  6.8KiB backtrace_sys __rbt_backtrace_dwarf_add
0.0%  0.1%  4.2KiB backtrace_sys __rbt_backtrace_qsort
0.0%  0.0%    575B backtrace_sys __rbt_backtrace_syminfo
0.0%  0.0%    559B backtrace_sys __rbt_backtrace_pcinfo
0.0%  0.0%    357B backtrace_sys __rbt_backtrace_initialize
0.0%  0.0%    216B backtrace_sys __rbt_backtrace_get_view
0.0%  0.0%    200B backtrace_sys __rbt_backtrace_create_state
0.0%  0.0%    193B backtrace_sys __rbt_backtrace_vector_grow
0.0%  0.0%    158B backtrace_sys __rbt_backtrace_open
0.0%  0.0%    156B           std __rust_start_panic
0.0%  0.0%    132B           std __rust_maybe_catch_panic
0.1%  0.2% 14.5KiB               filtered data size, the file size is 14.9MiB
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
