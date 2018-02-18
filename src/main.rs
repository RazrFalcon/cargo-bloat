extern crate cargo;
extern crate docopt;
extern crate env_logger;
extern crate goblin;
extern crate memmap;
extern crate object;
extern crate rustc_demangle;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate term_size;


mod table;


use std::{env, fs, path, str};
use std::collections::HashMap;

use object::Object;

use cargo::core::shell::Shell;
use cargo::core::Workspace;
use cargo::ops;
use cargo::util::errors::{CargoResult, CargoError};
use cargo::util;
use cargo::{CliResult, Config};

use table::Table;


const USAGE: &'static str = "
Find out what takes most of the space in your executable

Usage: cargo bloat [options]

Options:
    -h, --help              Print this message
    -V, --version           Print version info and exit
    --bin NAME              Name of the bin target to run
    --example NAME          Build only the specified example
    --release               Build artifacts in release mode, with optimizations
    --features FEATURES     Space-separated list of features to also build
    --all-features          Build all available features
    --no-default-features   Do not build the `default` feature
    --target TRIPLE         Build for the target triple
    --manifest-path PATH    Path to the manifest to analyze
    -v, --verbose           Use verbose output
    -q, --quiet             No output printed to stdout
    --color WHEN            Coloring: auto, always, never
    --frozen                Require Cargo.lock and cache are up to date
    --locked                Require Cargo.lock is up to date
    -Z FLAG ...             Unstable (nightly-only) flags to Cargo
    --crates                Per crate bloatedness
    --filter CRATE          Filter functions by crate
    --split-std             Split the 'std' crate to original crates like core, alloc, etc.
    --print-unknown         Print methods under the '[Unknown]' tag
    --full-fn               Print full function name with hash values
    -n NUM                  Number of lines to show, 0 to show all [default: 20]
    -w, --wide              Do not trim long function names
";

#[derive(Deserialize)]
struct Flags {
    flag_version: bool,
    flag_bin: Option<String>,
    flag_example: Option<String>,
    flag_release: bool,
    flag_features: Vec<String>,
    flag_all_features: bool,
    flag_no_default_features: bool,
    flag_target: Option<String>,
    flag_manifest_path: Option<String>,
    flag_verbose: u32,
    flag_quiet: Option<bool>,
    flag_color: Option<String>,
    flag_frozen: bool,
    flag_locked: bool,
    #[serde(rename = "flag_Z")] flag_z: Vec<String>,
    flag_crates: bool,
    flag_filter: Option<String>,
    flag_split_std: bool,
    flag_print_unknown: bool,
    flag_full_fn: bool,
    flag_n: usize,
    flag_wide: bool,
}

struct SymbolData {
    name: String,
    size: u64,
}

struct Data {
    symbols: Vec<SymbolData>,
    file_size: u64,
    text_size: u64,
}

struct CrateData {
    data: Data,
    std_crates: Vec<String>,
    dep_crates: Vec<String>,
    c_symbols: HashMap<String, String>,
}


fn main() {
    if !(cfg!(target_os = "linux") || cfg!(target_os = "macos")) {
        eprintln!("This OS is not supported.");
        std::process::exit(1);
    }

    env_logger::init();

    let cwd = env::current_dir().expect("couldn't get the current directory of the process");
    let mut config = create_config(cwd);

    let args: Vec<_> = env::args().collect();
    let result = cargo::call_main_without_stdin(real_main, &mut config, USAGE, &args, false);
    match result {
        Err(e) => cargo::exit_with_error(e, &mut *config.shell()),
        Ok(()) => {}
    }
}

fn create_config(path: path::PathBuf) -> Config {
    let shell = Shell::new();
    let homedir = util::config::homedir(&path).expect(
        "Cargo couldn't find your home directory. \
         This probably means that $HOME was not set.");
    Config::new(shell, path, homedir)
}

fn real_main(flags: Flags, config: &mut Config) -> CliResult {
    if flags.flag_version {
        println!("cargo-bloat {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let crate_data = process_crate(&flags, config)?;

    let mut table = Table::new(&["File", ".text", "Size", "Name"]);

    let term_width = if !flags.flag_wide {
        term_size::dimensions().map(|v| v.0)
    } else {
        None
    };
    table.set_width(term_width);


    if flags.flag_crates {
        print_crates(crate_data, &flags, &mut table);
    } else {
        print_methods(crate_data, &flags, &mut table);
    }

    println!();
    print!("{}", table);

    if flags.flag_crates {
        println!();
        println!("Warning: numbers above are a result of guesswork.\
                  They are not 100% correct and never will be.");
    }

    Ok(())
}

fn process_crate(flags: &Flags, config: &mut Config) -> CargoResult<CrateData> {
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

    let mut bins = Vec::new();
    let mut examples = Vec::new();

    let mut opt = ops::CompileOptions::default(&config, ops::CompileMode::Build);
    opt.features = &flags.flag_features;
    opt.all_features = flags.flag_all_features;
    opt.no_default_features = flags.flag_no_default_features;
    opt.release = flags.flag_release;

    if let Some(ref target) = flags.flag_target {
        opt.target = Some(target);
    }

    if let Some(ref name) = flags.flag_bin {
        bins.push(name.clone());
    } else if let Some(ref name) = flags.flag_example {
        examples.push(name.clone());
    }

    if flags.flag_bin.is_some() || flags.flag_example.is_some() {
        opt.filter = ops::CompileFilter::new(
            false,
            &bins[..], false,
            &[], false,
            &examples[..], false,
            &[], false,
            false,
        );
    }

    let comp = ops::compile(&workspace, &opt)?;

    let mut rlib_paths = collect_rlib_paths(&comp.deps_output);
    let mut dep_crates: Vec<String> = rlib_paths.iter().map(|v| v.0.clone()).collect();

    let mut crate_name = workspace.current().unwrap().name().to_string();
    crate_name = crate_name.replace("-", "_");
    dep_crates.push(crate_name);

    let mut std_crates = Vec::new();
    if let Some(path) = comp.target_dylib_path {
        let paths = collect_rlib_paths(&path);
        std_crates = paths.iter().map(|v| v.0.clone()).collect();

        rlib_paths.extend_from_slice(&paths);
    }

    let c_symbols = collect_c_symbols(rlib_paths)?;

    for (_, lib) in comp.libraries {
        for (_, path) in lib {
            let path_str = path.to_str().unwrap();
            if path_str.ends_with(".so") || path_str.ends_with(".dylib") {
                return Ok(CrateData {
                    data: collect_data(&path)?,
                    std_crates,
                    dep_crates,
                    c_symbols,
                });
            }
        }
    }

    if !comp.binaries.is_empty() {
        return Ok(CrateData {
            data: collect_data(&comp.binaries[0])?,
            std_crates,
            dep_crates,
            c_symbols,
        });
    }

    Err(CargoError::from("Only 'bin' and 'cdylib' targets are supported."))
}

fn collect_rlib_paths(deps_dir: &path::Path) -> Vec<(String, path::PathBuf)> {
    let mut rlib_paths: Vec<(String, path::PathBuf)> = Vec::new();
    if let Ok(entries) = fs::read_dir(deps_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(Some("rlib")) = path.extension().map(|s| s.to_str()) {
                    let mut stem = path.file_stem().unwrap().to_str().unwrap().to_string();
                    if let Some(idx) = stem.bytes().position(|b| b == b'-') {
                        stem.drain(idx..);
                    }

                    stem.drain(0..3); // trim 'lib'

                    rlib_paths.push((stem, path));
                }
            }
        }
    }

    rlib_paths.sort_by(|a, b| a.0.cmp(&b.0));

    rlib_paths
}

fn collect_c_symbols(libs: Vec<(String, path::PathBuf)>) -> CargoResult<HashMap<String, String>> {
    let mut map = HashMap::new();

    for (name, path) in libs {
        let file = fs::File::open(path)?;
        let file = unsafe { memmap::Mmap::map(&file)? };
        match goblin::archive::Archive::parse(&*file) {
            Ok(archive) => {
                for (_, _, symbols) in archive.summarize() {
                    for sym in symbols {
                        if !sym.starts_with("_ZN") {
                            map.insert(sym.to_string(), name.clone());
                        }
                    }
                }
            }
            Err(e) => {
                return Err(CargoError::from(e.to_string().as_str()));
            }
        }
    }

    Ok(map)
}

fn collect_data(path: &path::Path) -> CargoResult<Data> {
    let file = fs::File::open(path)?;
    let file = unsafe { memmap::Mmap::map(&file)? };
    let file = object::File::parse(&*file)?;

    let mut total_size = 0;
    let mut list = Vec::new();
    for symbol in file.symbol_map().symbols() {
        match symbol.kind() {
            object::SymbolKind::Section | object::SymbolKind::File => continue,
            _ => {}
        }

        if symbol.section_kind() != Some(object::SectionKind::Text) {
            continue;
        }

        total_size += symbol.size();

        let fn_name = symbol.name().unwrap_or("<unknown>");
        let fn_name = rustc_demangle::demangle(fn_name).to_string();

        list.push(SymbolData {
            name: fn_name,
            size: symbol.size(),
        });
    }

    let d = Data {
        symbols: list,
        file_size: fs::metadata(path)?.len(),
        text_size: total_size,
    };

    Ok(d)
}

fn print_methods(mut d: CrateData, flags: &Flags, table: &mut Table) {
    d.data.symbols.sort_by_key(|v| v.size);

    let dd = &d.data;
    let mut other_size = dd.text_size;

    let n = if flags.flag_n == 0 { dd.symbols.len() } else { flags.flag_n };

    for sym in dd.symbols.iter().rev() {
        let percent_file = sym.size as f64 / dd.file_size as f64 * 100.0;
        let percent_text = sym.size as f64 / dd.text_size as f64 * 100.0;

        if let Some(ref name) = flags.flag_filter {
            if !sym.name.contains(name) {
                continue;
            }
        }

        other_size -= sym.size;

        let mut name = sym.name.clone();

        // crate::mod::fn::h5fbe0f2f0b5c7342 -> crate::mod::fn
        if !flags.flag_full_fn {
            if let Some(pos) = name.bytes().rposition(|b| b == b':') {
                name.drain((pos - 1)..);
            }
        }

        push_row(table, percent_file, percent_text, sym.size, name);

        if n != 0 && table.rows_count() == n {
            break;
        }
    }

    {
        let lines_len = table.rows_count();
        let percent_file_s = format_percent(other_size as f64 / dd.file_size as f64 * 100.0);
        let percent_text_s = format_percent(other_size as f64 / dd.text_size as f64 * 100.0);
        let size_s = format_size(other_size);
        let name_s = format!("[{} Others]", dd.symbols.len() - lines_len);
        table.insert(0, &[&percent_file_s, &percent_text_s, &size_s, &name_s]);
    }

    push_total(table, dd);
}

fn print_crates(d: CrateData, flags: &Flags, table: &mut Table) {
    const UNKNOWN: &str = "[Unknown]";

    let dd = &d.data;
    let mut sizes = HashMap::new();

    for sym in dd.symbols.iter() {
        // Skip non-Rust names.
        let mut crate_name = if !sym.name.contains("::") {
            if let Some(v) = d.c_symbols.get(&sym.name) {
                v.clone()
            } else {
                if flags.flag_print_unknown {
                    println!("{}", sym.name);
                }

                UNKNOWN.to_string()
            }
        } else {
            if let Some(s) = sym.name.split("::").next() {
                s.to_owned()
            } else {
                sym.name.clone()
            }
        };

        if crate_name.starts_with("<") {
            while crate_name.starts_with("<") {
                crate_name.remove(0);
            }

            crate_name = crate_name.split_whitespace().last().unwrap().to_owned();
        }

        if !flags.flag_split_std {
            if d.std_crates.contains(&crate_name) {
                crate_name = "std".to_string();
            }
        }

        if     crate_name != UNKNOWN
            && crate_name != "std"
            && !d.std_crates.contains(&crate_name)
            && !d.dep_crates.contains(&crate_name) {
            if let Some(v) = d.c_symbols.get(&sym.name) {
                crate_name = v.clone();
            } else {
                if flags.flag_print_unknown {
                    println!("{}", sym.name);
                }

                crate_name = UNKNOWN.to_string();
            }
        }

        if let Some(v) = sizes.get(&crate_name).cloned() {
            sizes.insert(crate_name.to_string(), v + sym.size);
        } else {
            sizes.insert(crate_name.to_string(), sym.size);
        }
    }

    let mut list: Vec<(&String, &u64)> = sizes.iter().collect();
    list.sort_by_key(|v| v.1);

    let n = if flags.flag_n == 0 { list.len() } else { flags.flag_n };
    for &(k, v) in list.iter().rev().take(n) {
        let percent_file = *v as f64 / dd.file_size as f64 * 100.0;
        let percent_text = *v as f64 / dd.text_size as f64 * 100.0;

        push_row(table, percent_file, percent_text, *v, k.clone());
    }

    push_total(table, dd);
}

fn push_row(table: &mut Table, percent_file: f64, percent_text: f64, size: u64, name: String) {
    let percent_file_s = format_percent(percent_file);
    let percent_text_s = format_percent(percent_text);
    let size_s = format_size(size);

    table.push(&[percent_file_s, percent_text_s, size_s, name]);
}

fn push_total(table: &mut Table, d: &Data) {
    let percent_file = d.text_size as f64 / d.file_size as f64 * 100.0;
    let name = format!(".text section size, the file size is {}", format_size(d.file_size));
    push_row(table, percent_file, 100.0, d.text_size, name);
}

fn format_percent(n: f64) -> String {
    format!("{:.1}%", n)
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
