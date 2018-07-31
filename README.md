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
36.7%  95.5%  5.3MiB                       [14430 Others]
 0.3%   0.9% 48.8KiB          regex_syntax <regex_syntax::ast::parse::ParserI<'s, P>>::parse_wit...
 0.2%   0.6% 34.2KiB unicode_normalization unicode_normalization::tables::compatibility_fully_de...
 0.2%   0.5% 26.2KiB unicode_normalization unicode_normalization::tables::canonical_fully_decomp...
 0.2%   0.4% 25.1KiB                  clap clap::app::parser::Parser::get_matches_with
 0.2%   0.4% 23.0KiB                 cargo cargo::core::resolver::activate_deps_loop
 0.1%   0.4% 22.0KiB                 cargo cargo::util::toml::targets::targets
 0.1%   0.4% 20.4KiB                 toml? <toml::de::MapVisitor<'de, 'b> as serde::de::Deserial...
 0.1%   0.3% 19.1KiB                 cargo <cargo::util::toml::_IMPL_DESERIALIZE_FOR_TomlProject...
 0.1%   0.3% 19.1KiB                 cargo cargo::core::compiler::context::Context::compile
 0.1%   0.3% 17.9KiB                 cargo <cargo::util::toml::_IMPL_DESERIALIZE_FOR_TomlManifes...
38.5% 100.0%  5.6MiB                       .text section size, the file size is 14.5MiB
```

Get a list of the biggest dependencies in the release build:
```
% cargo bloat --release --crates -n 10
    Finished release [optimized] target(s) in 0.2 secs
   Analyzing target/release/cargo-bloat

 File  .text     Size Name
11.6%  30.2%   1.7MiB cargo
 8.1%  21.2%   1.2MiB std
 2.6%   6.7% 385.2KiB toml
 2.3%   6.0% 342.4KiB [Unknown]
 2.2%   5.7% 322.7KiB libgit2_sys
 2.0%   5.3% 303.8KiB clap
 1.4%   3.6% 207.2KiB regex_syntax
 1.1%   2.8% 160.2KiB regex
 1.0%   2.6% 148.4KiB goblin
 0.9%   2.3% 131.8KiB serde_json
38.5% 100.0%   5.6MiB .text section size, the file size is 14.5MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

Get a list of the biggest functions in the release build filtered by the regexp:
```
% cargo bloat --release --filter '^__' -n 10
    Finished release [optimized] target(s) in 0.2 secs
   Analyzing target/release/cargo-bloat

File .text    Size         Crate Name
0.0%  0.0%  1.3KiB               [25 Others]
0.0%  0.1%  4.7KiB backtrace_sys __rbt_backtrace_dwarf_add
0.0%  0.1%  3.0KiB backtrace_sys __rbt_backtrace_qsort
0.0%  0.0%   1000B           std __udivmodti4
0.0%  0.0%    565B backtrace_sys __rbt_backtrace_syminfo
0.0%  0.0%    565B backtrace_sys __rbt_backtrace_pcinfo
0.0%  0.0%    357B backtrace_sys __rbt_backtrace_initialize
0.0%  0.0%    219B backtrace_sys __rbt_backtrace_get_view
0.0%  0.0%    211B backtrace_sys __rbt_backtrace_vector_grow
0.0%  0.0%    197B backtrace_sys __rbt_backtrace_create_state
0.0%  0.0%    150B backtrace_sys __rbt_backtrace_open
0.1%  0.2% 12.2KiB               filtered data size, the file size is 14.5MiB
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
