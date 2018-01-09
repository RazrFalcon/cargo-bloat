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

use cargo::core::manifest;
use cargo::core::shell::Shell;
use cargo::core::Workspace;
use cargo::ops;
use cargo::util;
use cargo::{CliResult, Config};


const PERCENT_WIDTH: usize = 5;

const USAGE: &'static str = "
Find out what takes most of the space in your executable

Usage: cargo bloat [options]

Options:
    -h, --help              Print this message
    -V, --version           Print version info and exit
    --features FEATURES     Space-separated list of features to also build
    --manifest-path PATH    Path to the manifest to analyze
    --release               Build artifacts in release mode, with optimizations
    --crates                Per crate bloatedness
    --trim-fn               Trim hash values from function names
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
    flag_crates: bool,
    flag_trim_fn: bool,
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
    total_size: u64,
}

struct Line {
    percent: String,
    size: String,
    raw_size: u64,
    name: String,
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

    let mut opt = ops::CompileOptions::default(&config, ops::CompileMode::Build);
    opt.features = &flags.flag_features;
    opt.release = flags.flag_release;
    let comp = ops::compile(&workspace, &opt)?;

    let cdylib_kind = manifest::TargetKind::Lib(vec![manifest::LibKind::Other("cdylib".to_string())]);

    let mut is_processed = false;

    'outer: for (_, lib) in comp.libraries {
        for (target, path) in lib {
            if target.kind() == &cdylib_kind {
                process_bin(&path, &crates[..], &flags);

                // The 'cdylib' can be defined only once, so exit immediately.
                is_processed = true;
                break 'outer;
            }
        }
    }

    if !comp.binaries.is_empty() {
        process_bin(&comp.binaries[0], &crates[..], &flags);
        is_processed = true;
    }

    if !is_processed {
        println!("Only 'bin' and 'cdylib' targets are supported.");
    }

    Ok(())
}

fn process_bin(path: &path::Path, crates: &[String], flags: &Flags) {
    let pwd = env::current_dir().unwrap();
    println!("File: {}", path.strip_prefix(&pwd).unwrap().to_str().unwrap());

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
        total_size,
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
    let mut other_size = d.total_size;

    let n = if flags.flag_n == 0 { d.symbols.len() } else { flags.flag_n };

    for sym in d.symbols.iter().rev().take(n) {
        other_size -= sym.size;
        let percent = sym.size as f64 / d.total_size as f64 as f64 * 100.0;
        let mut dem_name = rustc_demangle::demangle(sym.name).to_string();

        // crate::mod::fn::h5fbe0f2f0b5c7342 -> crate::mod::fn
        if flags.flag_trim_fn {
            if let Some(pos) = dem_name.bytes().rposition(|b| b == b':') {
                dem_name.drain((pos - 1)..);
            }
        }

        lines.push(Line {
            percent: format_percent(percent),
            size: format_size(sym.size),
            raw_size: sym.size,
            name: dem_name,
        });
    }

    lines.push(Line {
        percent: format_percent(other_size as f64 / d.total_size as f64 * 100.0),
        size: format_size(other_size),
        raw_size: other_size,
        name: format!("[{} Others]", d.symbols.len() - flags.flag_n),
    });

    lines.sort_by_key(|v| v.raw_size);

    lines.insert(0, Line {
        percent: "100.0".into(),
        size: format_size(d.total_size),
        raw_size: d.total_size,
        name: "Total".into(),
    });

    let max_size_len = lines.iter().fold(0, |acc, ref v| cmp::max(acc, v.size.len()));

    let term_width = if !flags.flag_wide {
        term_size::dimensions().map(|v| v.0)
    } else {
        None
    };

    for line in lines.iter().rev() {
        print_line(line, max_size_len, term_width);
    }
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

        match crate_name.as_str() {
              "core"
            | "std_unicode"
            | "alloc"
            | "alloc_system"
            | "unreachable"
            | "unwind"
            | "panic_unwind" => {
                crate_name = "std".to_string();
            }
            _ => {}
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
        let percent = *v as f64 / d.total_size as f64 as f64 * 100.0;

        lines.push(Line {
            percent: format_percent(percent),
            size: format_size(*v),
            raw_size: 0,
            name: k.clone(),
        });
    }

    lines.push(Line {
        percent: "100.0".into(),
        size: format_size(d.total_size),
        raw_size: d.total_size,
        name: "Total".into(),
    });

    let max_size_len = lines.iter().fold(0, |acc, ref v| cmp::max(acc, v.size.len()));

    for line in lines.iter() {
        print_line(line, max_size_len, None);
    }
}

fn format_percent(n: f64) -> String {
    let mut s = format!("{:.1}", n);
    while s.len() < PERCENT_WIDTH {
        s.insert(0, ' ');
    }

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

fn print_line(line: &Line, max_size_len: usize, term_width: Option<usize>) {
    let mut size_s = line.size.clone();
    while size_s.len() < max_size_len {
        size_s.insert(0, ' ');
    }

    let mut name = line.name.clone();
    if let Some(term_width) = term_width {
        let name_width = term_width - max_size_len - PERCENT_WIDTH - 3;

        if line.name.len() > name_width {
            name.drain((name_width - 3)..);
            name.push_str("...");
        }
    }

    println!("{}% {} {}", line.percent, size_s, name);
}
