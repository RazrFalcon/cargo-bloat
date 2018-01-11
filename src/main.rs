extern crate cargo;
extern crate docopt;
extern crate env_logger;
extern crate memmap;
extern crate object;
extern crate rustc_demangle;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate term_size;


use std::{env, fs, cmp, path};
use std::collections::HashMap;

use object::{Object, SectionKind, SymbolKind};

use cargo::core::shell::Shell;
use cargo::core::Workspace;
use cargo::ops;
use cargo::util;
use cargo::{CliResult, Config};


const PERCENT_WIDTH: usize = 5;

const STD_CRATES: &[&str] = &[
    "core",
    "std_unicode",
    "alloc",
    "alloc_system",
    "unreachable",
    "unwind",
    "panic_unwind",
];

const USAGE: &'static str = "
Find out what takes most of the space in your executable

Usage: cargo bloat [options]

Options:
    -h, --help              Print this message
    -V, --version           Print version info and exit
    --features FEATURES     Space-separated list of features to also build
    --manifest-path PATH    Path to the manifest to analyze
    --release               Build artifacts in release mode, with optimizations
    --example NAME          Build only the specified example
    --crates                Per crate bloatedness
    --split-std             Split the 'std' crate to original crates like core, alloc, etc.
    --full-fn               Print full function name with hash values
    -n NUM                  Number of lines to show, 0 to show all [default: 20]
    -w, --wide              Do not trim long function names
    -v, --verbose           Use verbose output
    -q, --quiet             No output printed to stdout
    --color WHEN            Coloring: auto, always, never
    --frozen                Require Cargo.lock and cache are up to date
    --locked                Require Cargo.lock is up to date
    -Z FLAG ...             Unstable (nightly-only) flags to Cargo
";

#[derive(Deserialize)]
struct Flags {
    flag_version: bool,
    flag_features: Vec<String>,
    flag_manifest_path: Option<String>,
    flag_release: bool,
    flag_example: Option<String>,
    flag_crates: bool,
    flag_split_std: bool,
    flag_full_fn: bool,
    flag_n: usize,
    flag_wide: bool,
    flag_verbose: u32,
    flag_quiet: Option<bool>,
    flag_color: Option<String>,
    flag_frozen: bool,
    flag_locked: bool,
    #[serde(rename = "flag_Z")] flag_z: Vec<String>,
}

struct SymbolData<'a> {
    name: &'a str,
    size: u64,
}

struct Data<'a> {
    symbols: Vec<SymbolData<'a>>,
    file_size: u64,
    text_size: u64,
}

struct Line {
    percent_file: String,
    percent_text: String,
    size: String,
    raw_size: u64,
    name: String,
}

impl Line {
    fn new(percent_file: f64, percent_text: f64, size: u64, name: String) -> Self {
        Line {
            percent_file: format_percent(percent_file),
            percent_text: format_percent(percent_text),
            size: format_size(size),
            raw_size: size,
            name,
        }
    }
}


trait PadLeft {
    fn pad_left(&mut self, n: usize);
}

impl PadLeft for String {
    fn pad_left(&mut self, n: usize) {
        while self.len() < n {
            self.insert(0, ' ');
        }
    }
}


fn main() {
    env_logger::init().unwrap();

    let mut config = match Config::default() {
        Ok(cfg) => cfg,
        Err(e) => {
            let mut shell = Shell::new();
            cargo::exit_with_error(e.into(), &mut shell)
        }
    };

    let args: Vec<_> = env::args().collect();
    let result = cargo::call_main_without_stdin(real_main, &mut config, USAGE, &args, false);
    match result {
        Err(e) => cargo::exit_with_error(e, &mut *config.shell()),
        Ok(()) => {}
    }
}

fn real_main(flags: Flags, config: &mut Config) -> CliResult {
    if flags.flag_version {
        println!("cargo-bloat {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    config.configure(
        flags.flag_verbose,
        flags.flag_quiet,
        &flags.flag_color,
        flags.flag_frozen,
        flags.flag_locked,
        &flags.flag_z,
    )?;

    let root = util::important_paths::find_root_manifest_for_wd(
        flags.flag_manifest_path.clone(),
        config.cwd()
    )?;
    let workspace = Workspace::new(&root, config)?;
    let (pkgs, _) = ops::resolve_ws(&workspace)?;

    let mut crates: Vec<String> = pkgs.package_ids().map(|p| p.name().replace("-", "_")).collect();
    crates.push("std".to_string());
    if flags.flag_split_std {
        for crate_name in STD_CRATES {
            crates.push(crate_name.to_string());
        }
    }
    let crates = &crates[..];

    let mut examples = Vec::new();
    let mut opt = ops::CompileOptions::default(&config, ops::CompileMode::Build);
    opt.features = &flags.flag_features;
    opt.release = flags.flag_release;

    if let Some(ref name) = flags.flag_example {
        examples.push(name.clone());

        opt.filter = ops::CompileFilter::new(
            false,
            &[], false,
            &[], false,
            &examples[..], false,
            &[], false,
            false,
        );
    }

    let comp = ops::compile(&workspace, &opt)?;

    let mut is_processed = false;

    'outer: for (_, lib) in comp.libraries {
        for (_, path) in lib {
            let path_str = path.to_str().unwrap();
            if path_str.ends_with(".so") || path_str.ends_with(".dylib") {
                process_bin(&path, crates, &flags);

                // The 'cdylib' can be defined only once, so exit immediately.
                is_processed = true;
                break 'outer;
            }
        }
    }

    if !is_processed && !comp.binaries.is_empty() {
        process_bin(&comp.binaries[0], crates, &flags);
        is_processed = true;
    }

    if !is_processed {
        println!("Only 'bin' and 'cdylib' targets are supported.");
    }

    Ok(())
}

fn process_bin(path: &path::Path, crates: &[String], flags: &Flags) {
    let file = fs::File::open(path).unwrap();
    let file = unsafe { memmap::Mmap::map(&file).unwrap() };
    let file = object::File::parse(&*file).unwrap();

    let mut total_size = 0;
    let mut list = Vec::new();
    for symbol in file.symbol_map().symbols() {
        match symbol.kind() {
            SymbolKind::Section | SymbolKind::File => continue,
            _ => {}
        }

        if symbol.section_kind() != Some(SectionKind::Text) {
            continue;
        }

        total_size += symbol.size();
        list.push(SymbolData {
            name: symbol.name().unwrap_or("<unknown>"),
            size: symbol.size(),
        });
    }

    let data = Data {
        symbols: list,
        file_size: fs::metadata(path).unwrap().len(),
        text_size: total_size,
    };

    if flags.flag_crates {
        print_crates(data, crates, flags);
    } else {
        print_methods(data, flags);
    }
}

fn print_methods(mut d: Data, flags: &Flags) {
    d.symbols.sort_by_key(|v| v.size);

    let mut lines: Vec<Line> = Vec::new();
    let mut other_size = d.text_size;

    let n = if flags.flag_n == 0 { d.symbols.len() } else { flags.flag_n };

    for sym in d.symbols.iter().rev().take(n) {
        other_size -= sym.size;
        let percent_file = sym.size as f64 / d.file_size as f64 as f64 * 100.0;
        let percent_text = sym.size as f64 / d.text_size as f64 as f64 * 100.0;
        let mut dem_name = rustc_demangle::demangle(sym.name).to_string();

        // crate::mod::fn::h5fbe0f2f0b5c7342 -> crate::mod::fn
        if !flags.flag_full_fn {
            if let Some(pos) = dem_name.bytes().rposition(|b| b == b':') {
                dem_name.drain((pos - 1)..);
            }
        }

        lines.push(Line::new(percent_file, percent_text, sym.size, dem_name));
    }

    lines.push(Line::new(
        other_size as f64 / d.file_size as f64 * 100.0,
        other_size as f64 / d.text_size as f64 * 100.0,
        other_size,
        format!("[{} Others]", d.symbols.len() - flags.flag_n),
    ));

    lines.sort_by(|a, b| b.raw_size.cmp(&a.raw_size));

    print_table(d, lines, flags);
}

fn print_crates(d: Data, crates: &[String], flags: &Flags) {
    const UNKNOWN: &str = "[Unknown]";

    let mut sizes = HashMap::new();

    for sym in d.symbols.iter() {
        let name = rustc_demangle::demangle(sym.name).to_string();

        // Skip non-Rust names.
        let mut crate_name = if !name.contains("::") {
            UNKNOWN.to_string()
        } else {
            if let Some(s) = name.split("::").next() {
                s.to_owned()
            } else {
                name.clone()
            }
        };

        if crate_name.starts_with("<") {
            while crate_name.starts_with("<") {
                crate_name.remove(0);
            }

            crate_name = crate_name.split_whitespace().last().unwrap().to_owned();
        }

        if !flags.flag_split_std {
            if STD_CRATES.contains(&crate_name.as_str()) {
                crate_name = "std".to_string();
            }
        }

        if crate_name != UNKNOWN && !crates.contains(&crate_name) {
            crate_name = UNKNOWN.to_string();
        }

        if flags.flag_verbose > 0 {
            println!("{} from {}", crate_name, name);
        }

        if let Some(v) = sizes.get(&crate_name).cloned() {
            sizes.insert(crate_name, v + sym.size);
        } else {
            sizes.insert(crate_name, sym.size);
        }
    }

    let mut list: Vec<(&String, &u64)> = sizes.iter().collect();
    list.sort_by_key(|v| v.1);

    let mut lines = Vec::new();
    let n = if flags.flag_n == 0 { list.len() } else { flags.flag_n };
    for &(k, v) in list.iter().rev().take(n) {
        let percent_file = *v as f64 / d.file_size as f64 as f64 * 100.0;
        let percent_text = *v as f64 / d.text_size as f64 as f64 * 100.0;

        lines.push(Line::new(percent_file, percent_text, *v, k.clone()));
    }

    print_table(d, lines, flags);
}

fn format_percent(n: f64) -> String {
    let mut s = format!("{:.1}", n);
    s.pad_left(PERCENT_WIDTH);

    s
}

fn format_size(bytes: u64) -> String {
    let kib = 1024;
    let mib = 1024 * kib;

    if bytes >= mib {
        format!("{:.1}MiB", bytes as f64 / mib as f64)
    } else if bytes >= kib {
        format!("{:.1}KiB", bytes as f64 / kib as f64)
    } else {
        format!("{}B", bytes)
    }
}

fn print_table(d: Data, mut lines: Vec<Line>, flags: &Flags) {
    lines.push(Line::new(
        d.text_size as f64 / d.file_size as f64 * 100.0,
        100.0,
        d.text_size,
        format!(".text section size, the file size is {}", format_size(d.file_size))
    ));

    let term_width = if !flags.flag_wide {
        term_size::dimensions().map(|v| v.0)
    } else {
        None
    };

    let max_size_len = lines.iter().fold(0, |acc, ref v| cmp::max(acc, v.size.len()));

    print_header(max_size_len);
    for line in lines.iter() {
        print_line(line, max_size_len, term_width);
    }
}

fn print_header(max_size_len: usize) {
    let mut size_title = "Size".to_string();
    size_title.pad_left(max_size_len);

    println!();
    println!("  File  .text {} Name", size_title);
}

fn print_line(line: &Line, max_size_len: usize, term_width: Option<usize>) {
    let mut size_s = line.size.clone();
    size_s.pad_left(max_size_len);

    let mut name = line.name.clone();
    if let Some(term_width) = term_width {
        let name_width = term_width - max_size_len - PERCENT_WIDTH * 2 - 6;

        if line.name.len() > name_width {
            name.drain((name_width - 3)..);
            name.push_str("...");
        }
    }

    println!("{}% {}% {} {}", line.percent_file, line.percent_text, size_s, name);
}
