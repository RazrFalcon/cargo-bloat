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
Compiling ...
Analyzing target/release/cargo-bloat

 File  .text    Size        Crate Name
20.2%  86.6%  1.2MiB              [3651 Others]
 0.8%   3.2% 47.4KiB regex_syntax <regex_syntax::ast::parse::ParserI<'s, P>>::parse_with_comments
 0.4%   1.8% 25.7KiB         clap clap::app::parser::Parser::get_matches_with
 0.3%   1.5% 21.6KiB  cargo_bloat cargo_bloat::process_crate
 0.3%   1.1% 16.5KiB       goblin <goblin::mach::load_command::CommandVariant as scroll::ctx::Tr...
 0.3%   1.1% 16.0KiB         clap clap::app::help::Help::write_arg
 0.2%   1.0% 15.3KiB         clap clap::app::validator::Validator::validate_matched_args
 0.2%   1.0% 14.5KiB  cargo_bloat cargo_bloat::main
 0.2%   0.9% 13.7KiB        regex regex::exec::ExecBuilder::build
 0.2%   0.9% 12.6KiB         clap clap::app::help::Help::write_help
 0.2%   0.8% 12.2KiB         clap clap::app::usage::get_required_usage_from
23.3% 100.0%  1.4MiB              .text section size, the file size is 6.1MiB
```

Get a list of the biggest dependencies in the release build:
```
% cargo bloat --release --crates -n 10
Compiling ...
Analyzing target/release/cargo-bloat

 File  .text     Size Name
 7.0%  29.9% 437.5KiB std
 4.8%  20.5% 299.7KiB clap
 3.3%  14.1% 206.7KiB regex_syntax
 2.3%   9.8% 143.2KiB regex
 2.2%   9.4% 137.5KiB goblin
 1.6%   6.8%  99.4KiB [Unknown]
 0.7%   3.1%  45.4KiB cargo_bloat
 0.5%   2.3%  33.2KiB serde_json
 0.2%   1.0%  14.8KiB object
 0.2%   0.7%  10.2KiB rustc_demangle
23.3% 100.0%   1.4MiB .text section size, the file size is 6.1MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

Get a list of the biggest functions in the release build filtered by the regexp:

**Note**: you have to build `cargo-bloat` with a `regex-filter` feature enabled.

```
% cargo bloat --release --filter '^__' -n 10
Compiling ...
Analyzing target/release/cargo-bloat

File .text   Size     Crate Name
0.0%  0.0%    82B           [10 Others]
0.0%  0.1%   976B       std __udivmodti4
0.0%  0.0%   153B       std __rust_start_panic
0.0%  0.0%   128B       std __rust_maybe_catch_panic
0.0%  0.0%   101B [Unknown] __libc_csu_init
0.0%  0.0%    67B [Unknown] __pthread_atfork
0.0%  0.0%    45B       std __rust_probestack
0.0%  0.0%    45B       std __rde_alloc_zeroed
0.0%  0.0%    45B       std __rde_dealloc
0.0%  0.0%    45B       std __rde_alloc
0.0%  0.0%    40B       std __rde_realloc
0.0%  0.1% 1.7KiB           filtered data size, the file size is 6.1MiB
```

Flags specific for the `cargo-bloat`:
```
        --crates                   Per crate bloatedness
        --time                     Per crate build time. Will run `cargo clean` first
        --filter <CRATE|REGEXP>    Filter functions by crate
        --split-std                Split the 'std' crate to original crates like core, alloc, etc.
        --full-fn                  Print full function name with hash values
    -n <NUM>                       Number of lines to show, 0 to show all [default: 20]
    -w, --wide                     Do not trim long function names
```

### License

*cargo-bloat* is licensed under the MIT.
