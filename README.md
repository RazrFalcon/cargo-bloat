## cargo-bloat

Find out what takes most of the space in your executable.

Inspired by [google/bloaty](https://github.com/google/bloaty).

**Note:** supports ELF (Linux, BSD), Mach-O (macOS) and PE (Windows) binaries.

**Note:** Windows MSVC target is not supported. See [#17](https://github.com/RazrFalcon/cargo-bloat/issues/17).

**Note:** WASM is not supported. Prefer [twiggy](https://github.com/rustwasm/twiggy) instead.

### Install

```bash
cargo install cargo-bloat
```

or

```bash
cargo install cargo-bloat --features regex-filter
```

if you need regex filtering using the `--filter` option.

### Usage

Get a list of the biggest functions in the release build:

```
% cargo bloat --release -n 10
Compiling ...
Analyzing target/release/cargo-bloat

 File  .text     Size       Crate Name
 0.9%   7.1%  27.0KiB cargo_bloat cargo_bloat::main
 0.8%   5.7%  21.4KiB cargo_bloat cargo_bloat::process_crate
 0.3%   2.3%   8.6KiB   [Unknown] read_line_info
 0.3%   2.1%   7.9KiB         std std::sys::unix::process::process_common::Command::capture_env
 0.3%   2.1%   7.8KiB        json json::parser::Parser::parse
 0.2%   1.7%   6.5KiB   [Unknown] elf_add
 0.2%   1.7%   6.3KiB         std __rdos_backtrace_dwarf_add
 0.2%   1.3%   5.0KiB         std <rustc_demangle::legacy::Demangle as core::fmt::Display>::fmt
 0.2%   1.3%   4.9KiB         std std::sys_common::backtrace::_print
 0.2%   1.3%   4.8KiB         std core::num::flt2dec::strategy::dragon::format_shortest
 9.8%  73.5% 278.0KiB             And 932 smaller methods. Use -n N to show more.
13.3% 100.0% 378.0KiB             .text section size, the file size is 2.8MiB
```

Get a list of the biggest dependencies in the release build:
```
% cargo bloat --release --crates
Compiling ...
Analyzing target/release/cargo-bloat

 File  .text     Size Crate
 8.1%  61.2% 231.5KiB std
 2.5%  19.2%  72.4KiB cargo_bloat
 1.2%   9.4%  35.5KiB [Unknown]
 1.0%   7.2%  27.2KiB json
 0.3%   2.2%   8.5KiB pico_args
 0.1%   0.4%   1.7KiB multimap
 0.0%   0.3%   1.1KiB memmap
 0.0%   0.0%     175B term_size
 0.0%   0.0%      45B time
13.3% 100.0% 378.0KiB .text section size, the file size is 2.8MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

Get a list of the biggest functions in the release build filtered by the regexp:

**Note**: you have to build `cargo-bloat` with a `regex-filter` feature enabled.

```
% cargo bloat --release --filter '^__' -n 10
Compiling ...
Analyzing target/release/cargo-bloat

File .text    Size Crate Name
0.2%  1.7%  6.3KiB   std __rdos_backtrace_dwarf_add
0.1%  0.5%  1.9KiB   std __rdos_backtrace_qsort
0.0%  0.2%    843B   std __udivmodti4
0.0%  0.1%    296B   std __floattidf
0.0%  0.1%    290B   std __floattisf
0.0%  0.1%    284B   std __rdos_backtrace_initialize
0.0%  0.1%    253B   std __floatuntisf
0.0%  0.1%    253B   std __floatuntidf
0.0%  0.1%    211B   std __rdos_backtrace_get_view
0.0%  0.0%    180B   std __rdos_backtrace_vector_grow
0.1%  0.7%  2.8KiB       And 37 smaller methods. Use -n N to show more.
0.5%  3.6% 13.5KiB       filtered data size, the file size is 2.8MiB
```

Flags specific for `cargo-bloat`:
```
    --crates                   Per crate bloatedness
    --time                     Per crate build time. Will run `cargo clean` first
    --filter <CRATE|REGEXP>    Filter functions by crate
    --split-std                Split the 'std' crate to original crates like core, alloc, etc.
    --no-relative-size         Hide 'File' and '.text' columns
    --full-fn                  Print full function name with hash values
-n <NUM>                       Number of lines to show, 0 to show all [default: 20]
-w, --wide                     Do not trim long function names
    --message-format <FMT>     Output format [default: table] [possible values: table, json]
```

### License

*cargo-bloat* is licensed under the MIT license.
