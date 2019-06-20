use std::{fs, fmt, path, str};
use std::collections::HashMap;
use std::process::{self, Command};

use object::Object;

use structopt::clap::AppSettings;
use structopt::StructOpt;

use serde_derive::{Serialize, Deserialize};

use regex::Regex;

use multimap::MultiMap;

mod table;
use crate::table::Table;


#[derive(StructOpt)]
#[structopt(bin_name = "cargo")]
#[structopt(author = "")]
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

    #[structopt(short = "j", long = "jobs", value_name = "N")]
    /// Number of parallel jobs, defaults to # of CPUs
    jobs: Option<u32>,

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

    #[structopt(long = "target-dir", value_name = "DIRECTORY")]
    /// Directory for all generated artifacts
    target_dir: Option<String>,

    #[structopt(long = "frozen")]
    /// Require Cargo.lock and cache are up to date
    frozen: bool,

    #[structopt(long = "locked")]
    /// Require Cargo.lock is up to date
    locked: bool,

    #[structopt(long = "crates")]
    /// Per crate bloatedness
    crates: bool,

    #[structopt(long = "time")]
    /// Per crate build time. Will run `cargo clean` first
    time: bool,

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
    exe_path: Option<String>,
    data: Data,
    std_crates: Vec<String>,
    dep_crates: Vec<String>,
    deps_symbols: MultiMap<String, String>, // symbol, crate
    times: Vec<Elapsed>,
}

#[derive(Deserialize, Debug)]
struct Target {
    name: String,
    crate_types: Vec<String>,
    #[serde(skip)]
    __do_not_match_exhaustively: (),
}

#[derive(Deserialize, Debug)]
struct BuildOutput {
    target: Option<Target>,
    filenames: Option<Vec<String>>,
    #[serde(skip)]
    __do_not_match_exhaustively: (),
}

#[derive(Deserialize, Debug)]
struct Metadata {
    workspace_root: String,
    #[serde(skip)]
    __do_not_match_exhaustively: (),
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum ArtifactKind {
    Binary,
    Library,
    CDynLib,
}

#[derive(Debug)]
struct Artifact {
    kind: ArtifactKind,
    name: String, // TODO: Rc?
    path: path::PathBuf,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Elapsed {
    crate_name: String,
    time: u64,
    build_script: bool,
}

#[derive(Debug)]
enum Error {
    StdDirNotFound(path::PathBuf),
    RustcFailed,
    CargoError(String),
    CargoMetadataFailed,
    CargoBuildFailed,
    UnsupportedCrateType,
    OpenFailed(path::PathBuf),
    InvalidCargoOutput,
    NoArtifacts,
    Object(path::PathBuf, String),
    Goblin(path::PathBuf, String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::StdDirNotFound(ref path) => {
                write!(f, "failed to find a dir with std libraries. Expected location: {}",
                       path.to_str().unwrap())
            }
            Error::RustcFailed => {
                write!(f, "failed to execute 'rustc'. It should be in the PATH")
            }
            Error::CargoError(ref msg) => {
                write!(f, "{}", msg)
            }
            Error::CargoMetadataFailed => {
                write!(f, "failed to execute 'cargo'. It should be in the PATH")
            }
            Error::CargoBuildFailed => {
                write!(f, "failed to execute 'cargo build'. Probably a build error")
            }
            Error::UnsupportedCrateType => {
                write!(f, "only 'bin' and 'cdylib' crate types are supported")
            }
            Error::OpenFailed(ref path) => {
                write!(f, "failed to open a file '{}'", path.to_str().unwrap())
            }
            Error::InvalidCargoOutput => {
                write!(f, "failed to parse 'cargo' output")
            }
            Error::NoArtifacts => {
                write!(f, "'cargo' does not produce any build artifacts")
            }
            Error::Object(ref path, ref msg) => {
                write!(f, "'object' failed to parse '{}' cause '{}'",
                       path.to_str().unwrap(), msg)
            }
            Error::Goblin(ref path, ref msg) => {
                write!(f, "'goblin' failed to parse '{}' cause '{}'",
                       path.to_str().unwrap(), msg)
            }
        }
    }
}

impl std::error::Error for Error {}


fn main() {
    if cfg!(not(unix)) {
        eprintln!("This OS is not supported.");
        process::exit(1);
    }

    if let Ok(wrap) = std::env::var("RUSTC_WRAPPER") {
        if wrap.contains("cargo-bloat") {
            let args: Vec<_> = std::env::args().map(|a| a.to_string()).collect();
            match wrapper_mode(&args) {
                Ok(_) => return,
                Err(e) => {
                    eprintln!("Error: {}.", e);
                    process::exit(1);
                }
            }
        }
    }

    let Opts::Bloat(args) = Opts::from_args();

    let crate_data = match process_crate(&args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {}.", e);
            process::exit(1);
        }
    };

    if let Some(ref path) = crate_data.exe_path {
        println!("Analyzing {}", path);
    }

    let mut table = if args.crates {
        if args.time {
            Table::new(&["File", ".text", "Size", "Time", "Crate"])
        } else {
            Table::new(&["File", ".text", "Size", "Crate"])
        }
    } else if args.time {
        Table::new(&["Time", "Crate"])
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
    } else if args.time {
        print_times(crate_data, &mut table);
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

    if args.time && args.jobs != Some(1) {
        println!("Note: prefer using -j1 argument to disable a multithreaded build.");
    }
}

fn wrapper_mode(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let start = time::precise_time_ns();

    Command::new(&args[1])
        .args(&args[2..])
        .status()
        .map_err(|_| Error::CargoBuildFailed)?;

    let end = time::precise_time_ns();

    let mut crate_name = String::new();
    for (i, arg) in args.iter().enumerate() {
        if arg == "--crate-name" {
            crate_name = args[i + 1].clone();
            break;
        }
    }

    let mut build_script = false;

    if crate_name == "build_script_build" {
        build_script = true;

        let mut out_dir = String::new();
        let mut extra_filename = String::new();

        for (i, arg) in args.iter().enumerate() {
            if arg == "--out-dir" {
                out_dir = args[i + 1].clone();
            }

            if arg.starts_with("extra-filename") {
                extra_filename = arg[15..].to_string();
            }
        }

        if !out_dir.is_empty() {
            let path = std::path::Path::new(&out_dir);
            if let Some(name) = path.file_name() {
                let name = name.to_str().unwrap().to_string();
                let name = name.replace(&extra_filename, "");
                let name = name.replace("-", "_");
                crate_name = name;
            }
        }
    }

    // Still not resolved?
    if crate_name == "build_script_build" {
        crate_name = "?".to_string();
    }

    // TODO: the same crates but with different versions?

    let elapsed = Elapsed {
        crate_name,
        time: end - start,
        build_script,
    };

    // `cargo` will ignore raw JSON, so we have to use a prefix
    eprintln!("json-time {}", serde_json::to_string(&elapsed).unwrap());

    Ok(())
}

fn stdlibs_dir(target_triple: &str) -> Result<path::PathBuf, Error> {
    let output = Command::new("rustc")
        .arg("--print=sysroot").output()
        .map_err(|_| Error::RustcFailed)?;

    let stdout = str::from_utf8(&output.stdout).unwrap();

    // From the `cargo` itself (this is a one long link):
    // https://github.com/rust-lang/cargo/blob/065e3ef98d3edbce5c9e66d927d9ac9944cc6639
    // /src/cargo/core/compiler/build_context/target_info.rs#L130..L133
    let mut rustlib = path::PathBuf::from(stdout.trim());
    rustlib.push("lib");
    rustlib.push("rustlib");
    rustlib.push(target_triple);
    rustlib.push("lib");

    if !rustlib.exists() {
        return Err(Error::StdDirNotFound(rustlib));
    }

    Ok(rustlib)
}

fn get_default_target() -> Result<String, Error> {
    let output = Command::new("rustc").arg("-Vv").output().map_err(|_| Error::RustcFailed)?;

    let stdout = str::from_utf8(&output.stdout).unwrap();
    for line in stdout.lines() {
        if line.starts_with("host:") {
            return Ok(line[6..].to_owned())
        }
    }

    Err(Error::RustcFailed)
}

fn get_workspace_root() -> Result<String, Error> {
    let output = Command::new("cargo").args(&["metadata"])
        .output().map_err(|_| Error::CargoMetadataFailed)?;

    if !output.status.success() {
        let mut msg = str::from_utf8(&output.stderr).unwrap().trim();
        if msg.starts_with("error: ") {
            msg = &msg[7..];
        }

        return Err(Error::CargoError(msg.to_string()));
    }

    let stdout = str::from_utf8(&output.stdout).unwrap();
    for line in stdout.lines() {
        let meta: Metadata = serde_json::from_str(line).map_err(|_| Error::InvalidCargoOutput)?;
        return Ok(meta.workspace_root);
    }

    Err(Error::InvalidCargoOutput)
}

fn process_crate(args: &Args) -> Result<CrateData, Error> {
    let workspace_root = get_workspace_root()?;

    println!("Compiling ...");

    let output = if args.time {
        // To collect the build times we have to clean the repo first.

        // No need to check the output status.
        let _ = Command::new("cargo")
            .arg("clean")
            .output();

        Command::new("cargo")
            .args(&get_cargo_args(args))
            .env("RUSTC_WRAPPER", "cargo-bloat")
            .output()
            .map_err(|_| Error::CargoBuildFailed)?
    } else {
        Command::new("cargo")
            .args(&get_cargo_args(args))
            .output()
            .map_err(|_| Error::CargoBuildFailed)?
    };

    if !output.status.success() {
        return Err(Error::CargoBuildFailed);
    }

    let mut artifacts = Vec::new();
    let stdout = str::from_utf8(&output.stdout).unwrap();
    for line in stdout.lines() {
        let build: BuildOutput = serde_json::from_str(line).map_err(|_| Error::InvalidCargoOutput)?;

        if let Some(target) = build.target {
            if let Some(ref filenames) = build.filenames {
                for (path, crate_type) in filenames.iter().zip(target.crate_types) {
                    let kind = match crate_type.as_str() {
                        "bin" => ArtifactKind::Binary,
                        "lib" => ArtifactKind::Library,
                        "cdylib" => ArtifactKind::CDynLib,
                        _ => continue, // Simply ignore.
                    };

                    artifacts.push({
                        Artifact {
                            kind,
                            name: target.name.replace("-", "_"),
                            path: path::PathBuf::from(&path),
                        }
                    });
                }
            }
        }
    }

    let mut times = Vec::new();
    if args.time {
        let stderr = str::from_utf8(&output.stderr).unwrap();
        for line in stderr.lines() {
            if !line.starts_with("json-time {") {
                continue;
            }

            // Try to parse wrapper output first.
            if let Ok(elapsed) = serde_json::from_str::<Elapsed>(&line[10..]) {
                times.push(elapsed);
            }
        }
    }

    // Merge build script times into crate build times.
    while let Some(idx) = times.iter().position(|t| t.build_script) {
        let script_name = times[idx].crate_name.clone();
        let script_time = times[idx].time;

        for time in &mut times {
            if time.crate_name == script_name && !time.build_script {
                time.time += script_time;
            }
        }

        times.remove(idx);
    }

    if artifacts.is_empty() {
        return Err(Error::NoArtifacts);
    }

    if args.time && !args.crates {
        // We don't care about symbols if we only plan to print the build times.
        return Ok(CrateData {
            exe_path: None,
            data: Data { symbols: Vec::new(), file_size: 0, text_size: 0 },
            std_crates: Vec::new(),
            dep_crates: Vec::new(),
            deps_symbols: MultiMap::new(),
            times,
        });
    }

    let default_target = get_default_target()?;
    let target_triple = args.target.clone().unwrap_or_else(|| default_target);

    let target_dylib_path = stdlibs_dir(&target_triple)?;

    let mut rlib_paths = Vec::new();

    let mut dep_crates = Vec::new();
    for artifact in &artifacts {
        dep_crates.push(artifact.name.clone());

        if artifact.kind == ArtifactKind::Library {
            rlib_paths.push((artifact.name.clone(), artifact.path.clone()));
        }
    }

    dep_crates.dedup();
    dep_crates.sort();

    let std_paths = collect_rlib_paths(&target_dylib_path);
    let mut std_crates: Vec<String> = std_paths.iter().map(|v| v.0.clone()).collect();
    rlib_paths.extend_from_slice(&std_paths);
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

    let prepare_path = |path: &path::Path| {
        path.strip_prefix(workspace_root).unwrap_or(path).to_str().unwrap().to_string()
    };

    // The last artifact should be our binary/cdylib.
    if let Some(ref artifact) = artifacts.last() {
        if artifact.kind != ArtifactKind::Library {
            return Ok(CrateData {
                exe_path: Some(prepare_path(&artifact.path)),
                data: collect_self_data(&artifact.path)?,
                std_crates,
                dep_crates,
                deps_symbols,
                times,
            });
        }
    }

    Err(Error::UnsupportedCrateType)
}

fn get_cargo_args(args: &Args) -> Vec<String> {
    let mut list = Vec::new();
    list.push("build".to_string());
    list.push("--message-format=json".to_string());

    if args.release {
        list.push("--release".to_string());
    }

    if let Some(ref bin) = args.bin {
        list.push(format!("--bin={}", bin));
    } else if let Some(ref example) = args.example {
        list.push(format!("--example={}", example));
    }

    if let Some(ref features) = args.features {
        list.push(format!("--features={}", features));
    } else if args.all_features {
        list.push("--all-features".to_string());
    } else if args.no_default_features {
        list.push("--no-default-features".to_string());
    }

    if let Some(ref target) = args.target {
        list.push(format!("--target={}", target));
    }

    if let Some(ref target_dir) = args.target_dir {
        list.push(format!("--target-dir={}", target_dir));
    }

    if args.frozen {
        list.push("--frozen".to_string());
    }

    if args.locked {
        list.push("--locked".to_string());
    }

    if let Some(jobs) = args.jobs {
        list.push(format!("-j{}", jobs));
    }

    list
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

fn map_file(path: &path::Path) -> Result<memmap::Mmap, Error> {
    let file = fs::File::open(path).map_err(|_| Error::OpenFailed(path.to_owned()))?;
    let file = unsafe { memmap::Mmap::map(&file).map_err(|_| Error::OpenFailed(path.to_owned()))? };
    Ok(file)
}

fn collect_deps_symbols(
    libs: Vec<(String, path::PathBuf)>
) -> Result<MultiMap<String, String>, Error> {
    let mut map = MultiMap::new();

    for (name, path) in libs {
        let file = map_file(&path)?;
        let archive = goblin::archive::Archive::parse(&*file)
                          .map_err(|s| Error::Goblin(path.to_owned(), s.to_string()))?;
        for (_, _, symbols) in archive.summarize() {
            for sym in symbols {
                let sym = rustc_demangle::demangle(sym).to_string();
                map.insert(sym, name.clone());
            }
        }
    }

    for (_, v) in map.iter_all_mut() {
        v.dedup();
    }

    Ok(map)
}

fn collect_self_data(path: &path::Path) -> Result<Data, Error> {
    let file = map_file(&path)?;
    let file = object::File::parse(&*file)
                   .map_err(|s| Error::Object(path.to_owned(), s.to_owned()))?;

    let mut total_size = 0;
    let mut list = Vec::new();
    for symbol in file.symbol_map().symbols() {
        if symbol.is_undefined() || symbol.kind() != object::SymbolKind::Text {
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
        file_size: fs::metadata(path).unwrap().len(),
        text_size: total_size,
    };

    Ok(d)
}

fn print_methods(mut d: CrateData, args: &Args, table: &mut Table) {
    fn push_row(table: &mut Table, percent_file: f64, percent_text: f64, size: u64,
                crate_name: String, name: String)
    {
        let percent_file_s = format_percent(percent_file);
        let percent_text_s = format_percent(percent_text);
        let size_s = format_size(size);
        table.push(&[percent_file_s, percent_text_s, size_s, crate_name, name]);
    }

    d.data.symbols.sort_by_key(|v| v.size);

    let dd = &d.data;
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

    let has_filter = if let FilterBy::None = filter { false } else { true };

    let mut other_total = 0;
    let mut filter_total = 0;
    let mut matched_count = 0;

    for sym in dd.symbols.iter().rev() {
        let percent_file = sym.size as f64 / dd.file_size as f64 * 100.0;
        let percent_text = sym.size as f64 / dd.text_size as f64 * 100.0;

        let (mut crate_name, is_exact) = crate_from_sym(&d, args, &sym.name);

        if !is_exact {
            crate_name.push('?');
        }

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

        filter_total += sym.size;
        matched_count += 1;

        if n == 0 || table.rows_count() < n {
            push_row(table, percent_file, percent_text, sym.size, crate_name, name);
        } else {
            other_total += sym.size;
        }
    }

    {
        let others_count = if has_filter {
            matched_count - table.rows_count()
        } else {
            dd.symbols.len() - table.rows_count()
        };

        let percent_file_s = format_percent(other_total as f64 / dd.file_size as f64 * 100.0);
        let percent_text_s = format_percent(other_total as f64 / dd.text_size as f64 * 100.0);
        let size_s = format_size(other_total);
        let name_s = format!("[{} Others]", others_count);
        table.insert(0, &[&percent_file_s, &percent_text_s, &size_s, "", &name_s]);
    }

    if has_filter {
        let percent_file_s = filter_total as f64 / dd.file_size as f64 * 100.0;
        let percent_text_s = filter_total as f64 / dd.text_size as f64 * 100.0;
        let name = format!("filtered data size, the file size is {}", format_size(dd.file_size));
        push_row(table, percent_file_s, percent_text_s, filter_total, String::new(), name);
    } else {
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

fn crate_from_sym(d: &CrateData, args: &Args, sym: &str) -> (String, bool) {
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

            if let Some(names) = d.deps_symbols.get_vec(sym) {
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


    if !args.split_std {
        if d.std_crates.contains(&crate_name) {
            crate_name = "std".to_string();
        }
    }

    (crate_name, is_exact)
}

fn print_crates(d: CrateData, args: &Args, table: &mut Table) {
    let dd = &d.data;
    let mut sizes = HashMap::new();

    for sym in dd.symbols.iter() {
        let (crate_name, _) = crate_from_sym(&d, args, &sym.name);

        if let Some(v) = sizes.get(&crate_name).cloned() {
            sizes.insert(crate_name.to_string(), v + sym.size);
        } else {
            sizes.insert(crate_name.to_string(), sym.size);
        }
    }

    let mut list: Vec<(&String, &u64)> = sizes.iter().collect();
    list.sort_by_key(|v| v.1);

    fn push_row(table: &mut Table, percent_file: f64, percent_text: f64, size: u64,
                time: Option<String>, name: String)
    {
        let percent_file_s = format_percent(percent_file);
        let percent_text_s = format_percent(percent_text);
        let size_s = format_size(size);

        match time {
            Some(time) => {
                table.push(&[percent_file_s, percent_text_s, size_s, time, name]);
            }
            None => {
                table.push(&[percent_file_s, percent_text_s, size_s, name]);
            }
        }
    }

    for &(k, v) in list.iter().rev() {
        let percent_file = *v as f64 / dd.file_size as f64 * 100.0;
        let percent_text = *v as f64 / dd.text_size as f64 * 100.0;

        let time = if args.time {
            Some(match d.times.iter().find(|e| e.crate_name == *k) {
                Some(elapsed) => format_time(elapsed.time),
                None => "-".to_string(),
            })
        } else {
            None
        };

        push_row(table, percent_file, percent_text, *v, time, k.clone());
    }

    {
        let time = if args.time {
            Some(String::new())
        } else {
            None
        };

        let percent_file = dd.text_size as f64 / dd.file_size as f64 * 100.0;
        let name = format!(".text section size, the file size is {}", format_size(dd.file_size));
        push_row(table, percent_file, 100.0, dd.text_size, time, name);
    }
}

fn print_times(d: CrateData, table: &mut Table) {
    let mut times = d.times.clone();
    times.sort_by(|a, b| b.time.cmp(&a.time));

    for time in times {
        table.push(&[format_time(time.time), time.crate_name]);
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

fn format_time(ns: u64) -> String {
    format!("{:.2}s", ns as f64 / 1_000_000_000.0)
}
