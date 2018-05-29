extern crate cargo;
extern crate env_logger;
extern crate goblin;
extern crate memmap;
extern crate multimap;
extern crate object;
extern crate regex;
extern crate rustc_demangle;
extern crate term_size;
#[macro_use] extern crate failure;
#[macro_use] extern crate structopt;


mod table;


use std::{env, fs, path, str};
use std::collections::HashMap;

use object::Object;

use cargo::core::resolver::Method;
use cargo::core::shell::Shell;
use cargo::core::Workspace;
use cargo::ops;
use cargo::util::errors::CargoResult;
use cargo::util;
use cargo::{CliResult, Config};

use structopt::clap::AppSettings;
use structopt::StructOpt;

use regex::Regex;

use multimap::MultiMap;

use table::Table;


#[derive(StructOpt)]
#[structopt(bin_name = "cargo")]
enum Opts {
    #[structopt(
        name = "bloat",
        raw(
            setting = "AppSettings::UnifiedHelpMessage",
            setting = "AppSettings::DeriveDisplayOrder",
            setting = "AppSettings::DontCollapseArgsInUsage"
        )
    )]
    /// Find out what takes most of the space in your executable
    Bloat(Args),
}

#[derive(StructOpt)]
struct Args {
    #[structopt(long = "bin", value_name = "NAME")]
    /// Build only the specified binary
    bin: Option<String>,

    #[structopt(long = "example", value_name = "NAME")]
    /// Build only the specified example
    example: Option<String>,

    #[structopt(long = "release")]
    /// Build artifacts in release mode, with optimizations
    release: bool,

    #[structopt(long = "features", value_name = "FEATURES")]
    /// Space-separated list of features to activate
    features: Option<String>,

    #[structopt(long = "all-features")]
    /// Activate all available features
    all_features: bool,

    #[structopt(long = "no-default-features")]
    /// Do not activate the `default` feature
    no_default_features: bool,

    #[structopt(long = "target", value_name = "TARGET")]
    /// Build for the target triple
    target: Option<String>,

    #[structopt(long = "verbose", short = "v", parse(from_occurrences))]
    /// Use verbose output (-vv very verbose/build.rs output)
    verbose: u32,

    #[structopt(long = "quiet", short = "q")]
    /// No output printed to stdout
    quiet: Option<bool>,

    #[structopt(long = "color", value_name = "WHEN")]
    /// Coloring: auto, always, never
    color: Option<String>,

    #[structopt(long = "frozen")]
    /// Require Cargo.lock and cache are up to date
    frozen: bool,

    #[structopt(long = "locked")]
    /// Require Cargo.lock is up to date
    locked: bool,

    #[structopt(short = "Z", value_name = "FLAG")]
    /// Unstable (nightly-only) flags to Cargo
    unstable_flags: Vec<String>,

    #[structopt(long = "crates")]
    /// Per crate bloatedness
    crates: bool,

    #[structopt(long = "filter", value_name = "CRATE|REGEXP")]
    /// Filter functions by crate
    filter: Option<String>,

    #[structopt(long = "split-std")]
    /// Split the 'std' crate to original crates like core, alloc, etc.
    split_std: bool,

    #[structopt(long = "full-fn")]
    /// Print full function name with hash values
    full_fn: bool,

    #[structopt(short = "n", default_value = "20", value_name = "NUM")]
    /// Number of lines to show, 0 to show all
    n: usize,

    #[structopt(short = "w", long = "wide")]
    /// Do not trim long function names
    wide: bool,
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
    deps_symbols: MultiMap<String, String>, // symbol, crate
}

#[derive(Fail, Debug)]
enum Error {
    #[fail(display = "{}", _0)]
    Io(::std::io::Error),

    #[fail(display = "{}", _0)]
    String(String),
}

impl From<::std::io::Error> for Error {
    fn from(value: ::std::io::Error) -> Error {
        Error::Io(value)
    }
}

impl<'a> From<&'a str> for Error {
    fn from(value: &str) -> Error {
        Error::String(value.to_string())
    }
}


fn main() {
    if cfg!(not(unix)) {
        eprintln!("This OS is not supported.");
        std::process::exit(1);
    }

    env_logger::init();

    let cwd = env::current_dir().expect("couldn't get the current directory of the process");
    let mut config = create_config(cwd);

    let Opts::Bloat(args) = Opts::from_args();

    if let Err(e) = real_main(args, &mut config) {
        let mut shell = Shell::new();
        cargo::exit_with_error(e.into(), &mut shell)
    }
}

fn create_config(path: path::PathBuf) -> Config {
    let shell = Shell::new();
    let homedir = util::config::homedir(&path).expect(
        "Cargo couldn't find your home directory. \
         This probably means that $HOME was not set.");
    Config::new(shell, path, homedir)
}

fn real_main(args: Args, config: &mut Config) -> CliResult {
    let crate_data = process_crate(&args, config)?;

    let mut table = if args.crates {
        Table::new(&["File", ".text", "Size", "Name"])
    } else {
        Table::new(&["File", ".text", "Size", "Crate", "Name"])
    };

    let term_width = if !args.wide {
        term_size::dimensions().map(|v| v.0)
    } else {
        None
    };
    table.set_width(term_width);


    if args.crates {
        print_crates(crate_data, &args, &mut table);
    } else {
        print_methods(crate_data, &args, &mut table);
    }

    println!();
    print!("{}", table);

    if args.crates {
        println!();
        println!("Note: numbers above are a result of guesswork. \
                  They are not 100% correct and never will be.");
    }

    Ok(())
}

fn process_crate(args: &Args, config: &mut Config) -> CargoResult<CrateData> {
    config.configure(
        args.verbose,
        args.quiet,
        &args.color,
        args.frozen,
        args.locked,
        &args.unstable_flags,
    )?;

    let root = util::important_paths::find_root_manifest_for_wd(config.cwd())?;
    let workspace = Workspace::new(&root, config)?;

    let mut bins = Vec::new();
    let mut examples = Vec::new();

    let features = Method::split_features(&args.features.clone().into_iter().collect::<Vec<_>>());

    let mut opt = ops::CompileOptions::default(&config, ops::CompileMode::Build);
    opt.features = features;
    opt.all_features = args.all_features;
    opt.no_default_features = args.no_default_features;
    opt.release = args.release;
    opt.target = args.target.clone();

    if let Some(ref name) = args.bin {
        bins.push(name.clone());
    } else if let Some(ref name) = args.example {
        examples.push(name.clone());
    }

    if args.bin.is_some() || args.example.is_some() {
        opt.filter = ops::CompileFilter::new(
            false,
            bins.clone(), false,
            Vec::new(), false,
            examples.clone(), false,
            Vec::new(), false,
            false,
        );
    }

    let comp = ops::compile(&workspace, &opt)?;

    let mut rlib_paths = collect_rlib_paths(&comp.deps_output);
    let mut dep_crates: Vec<String> = rlib_paths.iter().map(|v| v.0.clone()).collect();
    dep_crates.dedup();

    let mut crate_name = workspace.current().unwrap().name().to_string();
    crate_name = crate_name.replace("-", "_");
    dep_crates.push(crate_name);

    dep_crates.sort();

    let mut std_crates = Vec::new();
    if let Some(path) = comp.target_dylib_path {
        let paths = collect_rlib_paths(&path);
        std_crates = paths.iter().map(|v| v.0.clone()).collect();

        rlib_paths.extend_from_slice(&paths);
    }
    std_crates.sort();

    // Remove std crates that was explicitly added as dependencies.
    //
    // Like: getopts, bitflags, backtrace, log, etc.
    for c in &dep_crates {
        if let Some(idx) = std_crates.iter().position(|v| v == c) {
            std_crates.remove(idx);
        }
    }

    let deps_symbols = collect_deps_symbols(rlib_paths)?;

    for (_, lib) in comp.libraries {
        for (_, path) in lib {
            let path_str = path.to_str().unwrap();
            if path_str.ends_with(".so") || path_str.ends_with(".dylib") {
                return Ok(CrateData {
                    data: collect_self_data(&path)?,
                    std_crates,
                    dep_crates,
                    deps_symbols,
                });
            }
        }
    }

    if !comp.binaries.is_empty() {
        return Ok(CrateData {
            data: collect_self_data(&comp.binaries[0])?,
            std_crates,
            dep_crates,
            deps_symbols,
        });
    }

    bail!("Only 'bin' and 'cdylib' targets are supported.")
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

fn collect_deps_symbols(libs: Vec<(String, path::PathBuf)>) -> CargoResult<MultiMap<String, String>> {
    let mut map = MultiMap::new();

    for (name, path) in libs {
        let file = fs::File::open(path)?;
        let file = unsafe { memmap::Mmap::map(&file)? };
        match goblin::archive::Archive::parse(&*file) {
            Ok(archive) => {
                for (_, _, symbols) in archive.summarize() {
                    for sym in symbols {
                        let sym = rustc_demangle::demangle(sym).to_string();
                        map.insert(sym, name.clone());
                    }
                }
            }
            Err(e) => {
                bail!(e.to_string())
            }
        }
    }

    for (_, v) in map.iter_all_mut() {
        v.dedup();
    }

    Ok(map)
}

fn collect_self_data(path: &path::Path) -> Result<Data, Error> {
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

fn print_methods(mut d: CrateData, args: &Args, table: &mut Table) {
    fn push_row(table: &mut Table, percent_file: f64, percent_text: f64, size: u64,
                crate_name: String, name: String) {
        let percent_file_s = format_percent(percent_file);
        let percent_text_s = format_percent(percent_text);
        let size_s = format_size(size);
        table.push(&[percent_file_s, percent_text_s, size_s, crate_name, name]);
    }

    d.data.symbols.sort_by_key(|v| v.size);

    let dd = &d.data;
    let mut other_size = dd.text_size;

    let n = if args.n == 0 { dd.symbols.len() } else { args.n };

    enum FilterBy {
        None,
        Crate(String),
        Regex(Regex),
    }

    let filter = if let Some(ref text) = args.filter {
        if d.std_crates.contains(text) || d.dep_crates.contains(text) {
            FilterBy::Crate(text.clone())
        } else {
            match Regex::new(text) {
                Ok(re) => FilterBy::Regex(re),
                Err(_) => {
                    eprintln!("Warning: the filter value contains an unknown crate \
                               or an invalid regexp. Ignored.");
                    FilterBy::None
                }
            }
        }
    } else {
        FilterBy::None
    };


    for sym in dd.symbols.iter().rev() {
        let percent_file = sym.size as f64 / dd.file_size as f64 * 100.0;
        let percent_text = sym.size as f64 / dd.text_size as f64 * 100.0;

        let (mut crate_name, is_exact) = crate_from_sym(&d, args, &sym.name);

        if !is_exact {
            crate_name.push('?');
        }

        other_size -= sym.size;

        let mut name = sym.name.clone();

        // crate::mod::fn::h5fbe0f2f0b5c7342 -> crate::mod::fn
        if !args.full_fn {
            if let Some(pos) = name.bytes().rposition(|b| b == b':') {
                name.drain((pos - 1)..);
            }
        }

        match filter {
            FilterBy::None => {}
            FilterBy::Crate(ref crate_name_f) => {
                if crate_name_f != &crate_name {
                    continue;
                }
            }
            FilterBy::Regex(ref re) => {
                if !re.is_match(&name) {
                    continue;
                }
            }
        }

        push_row(table, percent_file, percent_text, sym.size, crate_name, name);

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
        table.insert(0, &[&percent_file_s, &percent_text_s, &size_s, "", &name_s]);
    }

    {
        let percent_file = dd.text_size as f64 / dd.file_size as f64 * 100.0;
        let name = format!(".text section size, the file size is {}", format_size(dd.file_size));
        push_row(table, percent_file, 100.0, dd.text_size, String::new(), name);
    }
}

#[derive(Debug)]
enum Symbol {
    Function(String),
    CFunction,
    Trait(String, String),
}

// A simple stupid symbol parser.
// Should be replaced by something better later.
fn parse_sym(sym: &str) -> Symbol {
    if !sym.contains("::") {
        return Symbol::CFunction;
    }

    // TODO: ` for `

    if sym.contains(" as ") {
        let parts: Vec<_> = sym.split(" as ").collect();
        Symbol::Trait(parse_crate_from_sym(parts[0]), parse_crate_from_sym(parts[1]))
    } else {
        Symbol::Function(parse_crate_from_sym(sym))
    }
}

fn parse_crate_from_sym(sym: &str) -> String {
    if !sym.contains("::") {
        return String::new();
    }

    let mut crate_name = if let Some(s) = sym.split("::").next() {
        s.to_string()
    } else {
        sym.to_string()
    };

    if crate_name.starts_with("<") {
        while crate_name.starts_with("<") {
            crate_name.remove(0);
        }

        crate_name = crate_name.split_whitespace().last().unwrap().to_owned();
    }

    crate_name
}

fn crate_from_sym(d: &CrateData, flags: &Args, sym: &str) -> (String, bool) {
    const UNKNOWN: &str = "[Unknown]";

    let mut is_exact = true;

    // If the symbols aren't present in dependencies - try to figure out
    // where it was defined.
    //
    // The algorithm below is completely speculative.
    let mut crate_name = match parse_sym(sym) {
        Symbol::CFunction => {
            if let Some(name) = d.deps_symbols.get(sym) {
                name.to_string()
            } else {
                // If the symbols is a C function and it wasn't found
                // in `deps_symbols` that we can't do anything about it.
                UNKNOWN.to_string()
            }
        }
        Symbol::Function(crate_name) => {
            // Just a simple function like:
            // getopts::Options::parse

            if let Some(mut names) = d.deps_symbols.get_vec(sym) {
                if names.len() == 1 {
                    // In case the symbol was instanced in a different crate.
                    names[0].clone()
                } else {
                    crate_name
                }
            } else {
                crate_name
            }
        }
        Symbol::Trait(ref crate_name1, ref crate_name2) => {
            // <crate_name1::Type as crate_name2::Trait>::fn

            // `crate_name1` can be empty in cases when it's just a type parameter, like:
            // <T as core::fmt::Display>::fmt::h92003a61120a7e1a
            if crate_name1.is_empty() {
                crate_name2.clone()
            } else {
                if crate_name1 == crate_name2 {
                    crate_name1.clone()
                } else {
                    // This is an uncertain case.
                    //
                    // Example:
                    // <euclid::rect::TypedRect<f64> as resvg::geom::RectExt>::x
                    //
                    // Here we defined and instanced the `RectExt` trait
                    // in the `resvg` crate, but the first crate is `euclid`.
                    // Usually, those traits will be present in `deps_symbols`
                    // so they will be resolved automatically, in other cases it's an UB.

                    if let Some(names) = d.deps_symbols.get_vec(sym) {
                        if names.contains(crate_name1) {
                            crate_name1.clone()
                        } else if names.contains(crate_name2) {
                            crate_name2.clone()
                        } else {
                            // Example:
                            // <std::collections::hash::map::DefaultHasher as core::hash::Hasher>::finish
                            // ["cc", "cc", "fern", "fern", "svgdom", "svgdom"]

                            is_exact = false;
                            crate_name1.clone()
                        }
                    } else {
                        // If the symbol is not in `deps_symbols` then it probably
                        // was imported/inlined to the crate bin itself.

                        is_exact = false;
                        crate_name1.clone()
                    }
                }
            }
        }
    };

    // If the detected crate is unknown (and not marked as `[Unknown]`),
    // then mark it as `[Unknown]`.
    if     crate_name != UNKNOWN
        && crate_name != "std"
        && !d.std_crates.contains(&crate_name)
        && !d.dep_crates.contains(&crate_name)
        {
            // There was probably a bug in the code above if we get here.
            crate_name = UNKNOWN.to_string();
        }


    if !flags.split_std {
        if d.std_crates.contains(&crate_name) {
            crate_name = "std".to_string();
        }
    }

    (crate_name, is_exact)
}

fn print_crates(d: CrateData, flags: &Args, table: &mut Table) {
    let dd = &d.data;
    let mut sizes = HashMap::new();

    for sym in dd.symbols.iter() {
        let (mut crate_name, _) = crate_from_sym(&d, flags, &sym.name);

        if let Some(v) = sizes.get(&crate_name).cloned() {
            sizes.insert(crate_name.to_string(), v + sym.size);
        } else {
            sizes.insert(crate_name.to_string(), sym.size);
        }
    }

    let mut list: Vec<(&String, &u64)> = sizes.iter().collect();
    list.sort_by_key(|v| v.1);

    fn push_row(table: &mut Table, percent_file: f64, percent_text: f64, size: u64, name: String) {
        let percent_file_s = format_percent(percent_file);
        let percent_text_s = format_percent(percent_text);
        let size_s = format_size(size);

        table.push(&[percent_file_s, percent_text_s, size_s, name]);
    }

    let n = if flags.n == 0 { list.len() } else { flags.n };
    for &(k, v) in list.iter().rev().take(n) {
        let percent_file = *v as f64 / dd.file_size as f64 * 100.0;
        let percent_text = *v as f64 / dd.text_size as f64 * 100.0;

        push_row(table, percent_file, percent_text, *v, k.clone());
    }

    {
        let percent_file = dd.text_size as f64 / dd.file_size as f64 * 100.0;
        let name = format!(".text section size, the file size is {}", format_size(dd.file_size));
        push_row(table, percent_file, 100.0, dd.text_size, name);
    }
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
