use std::{fs, fmt, path, str};
use std::collections::HashMap;
use std::process::{self, Command};

use multimap::MultiMap;

use json::object;

mod ar;
mod crate_name;
mod demangle;
mod elf32;
mod elf64;
mod macho;
mod parser;
mod table;

use crate::table::Table;

struct SymbolData {
    name: demangle::SymbolName,
    size: u64,
}

struct Data {
    symbols: Vec<SymbolData>,
    file_size: u64,
    text_size: u64,
}

pub(crate) struct CrateData {
    exe_path: Option<String>,
    data: Data,
    std_crates: Vec<String>,
    dep_crates: Vec<String>,
    deps_symbols: MultiMap<String, String>, // symbol, crate
    times: Vec<Elapsed>,
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

#[derive(Clone)]
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
    UnsupportedFileFormat(path::PathBuf),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::StdDirNotFound(ref path) => {
                write!(f, "failed to find a dir with std libraries. Expected location: {}",
                       path.display())
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
                write!(f, "failed to open a file '{}'", path.display())
            }
            Error::InvalidCargoOutput => {
                write!(f, "failed to parse 'cargo' output")
            }
            Error::NoArtifacts => {
                write!(f, "'cargo' does not produce any build artifacts")
            }
            Error::UnsupportedFileFormat(ref path) => {
                write!(f, "'{}' has an unsupported file format", path.display())
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

    let mut args: Vec<_> = std::env::args_os().collect();
    args.remove(0); // file path
    if args.get(0).and_then(|s| s.to_str()) == Some("bloat") {
        args.remove(0);
    } else {
        eprintln!("Error: can be run only via `cargo bloat`.");
        process::exit(1);
    }

    let args = match parse_args(args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {}.", e);
            process::exit(1);
        }
    };

    if args.help {
        println!("{}", HELP);
        return;
    }

    if args.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return;
    }

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
        println!("Note: prefer using `-j 1` argument to disable a multithreaded build.");
    }
}

const HELP: &str = "\
Find out what takes most of the space in your executable

USAGE:
    cargo bloat [OPTIONS]

OPTIONS:
    -h, --help                      Prints help information
    -V, --version                   Prints version information
        --bin <NAME>                Build only the specified binary
        --example <NAME>            Build only the specified example
        --test <NAME>               Build only the specified test target
    -p, --package <SPEC>            Package to build
        --release                   Build artifacts in release mode, with optimizations
    -j, --jobs <N>                  Number of parallel jobs, defaults to # of CPUs
        --features <FEATURES>       Space-separated list of features to activate
        --all-features              Activate all available features
        --no-default-features       Do not activate the `default` feature
        --target <TARGET>           Build for the target triple
        --target-dir <DIRECTORY>    Directory for all generated artifacts
        --frozen                    Require Cargo.lock and cache are up to date
        --locked                    Require Cargo.lock is up to date
        --crates                    Per crate bloatedness
        --time                      Per crate build time. Will run `cargo clean` first
        --filter <CRATE|REGEXP>     Filter functions by crate
        --split-std                 Split the 'std' crate to original crates like core, alloc, etc.
        --full-fn                   Print full function name with hash values
    -n <NUM>                        Number of lines to show, 0 to show all [default: 20]
    -w, --wide                      Do not trim long function names
";

pub(crate) struct Args {
    help: bool,
    version: bool,
    bin: Option<String>,
    example: Option<String>,
    test: Option<String>,
    package: Option<String>,
    release: bool,
    jobs: Option<u32>,
    features: Option<String>,
    all_features: bool,
    no_default_features: bool,
    target: Option<String>,
    target_dir: Option<String>,
    frozen: bool,
    locked: bool,
    crates: bool,
    time: bool,
    filter: Option<String>,
    split_std: bool,
    full_fn: bool,
    n: usize,
    wide: bool,
}

fn parse_args(raw_args: Vec<std::ffi::OsString>) -> Result<Args, pico_args::Error> {
    let mut input = pico_args::Arguments::from_vec(raw_args);
    let args = Args {
        help:                   input.contains(["-h", "--help"]),
        version:                input.contains(["-V", "--version"]),
        bin:                    input.value_from_str("--bin")?,
        example:                input.value_from_str("--example")?,
        test:                   input.value_from_str("--test")?,
        package:                input.value_from_str(["-p", "--package"])?,
        release:                input.contains("--release"),
        jobs:                   input.value_from_str(["-j", "--jobs"])?,
        features:               input.value_from_str("--features")?,
        all_features:           input.contains("--all-features"),
        no_default_features:    input.contains("--no-default-features"),
        target:                 input.value_from_str("--target")?,
        target_dir:             input.value_from_str("--target-dir")?,
        frozen:                 input.contains("--frozen"),
        locked:                 input.contains("--locked"),
        crates:                 input.contains("--crates"),
        time:                   input.contains("--time"),
        filter:                 input.value_from_str("--filter")?,
        split_std:              input.contains("--split-std"),
        full_fn:                input.contains("--full-fn"),
        n:                      input.value_from_str("-n")?.unwrap_or(20),
        wide:                   input.contains(["-w", "--wide"]),
    };

    input.finish()?;

    Ok(args)
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

    // `cargo` will ignore raw JSON, so we have to use a prefix
    eprintln!("json-time {}", object!{
        "crate_name" => crate_name,
        "time" => end - start,
        "build_script" => build_script
    }.dump());

    Ok(())
}

fn stdlibs_dir(target_triple: &str) -> Result<path::PathBuf, Error> {
    // Support xargo by applying the rustflags
    // This is meant to match how cargo handles the RUSTFLAG environment
    // variable.
    // See https://github.com/rust-lang/cargo/blob/69aea5b6f69add7c51cca939a79644080c0b0ba0/src/cargo/core/compiler/build_context/target_info.rs#L434-L441
    let rustflags = std::env::var("RUSTFLAGS")
        .unwrap_or("".to_string());

    let rustflags = rustflags
        .split(' ')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(AsRef::<std::ffi::OsStr>::as_ref);

    let output = Command::new("rustc")
        .args(rustflags)
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
        let meta = json::parse(line).map_err(|_| Error::InvalidCargoOutput)?;
        let root = meta["workspace_root"].as_str().ok_or_else(|| Error::InvalidCargoOutput)?;
        return Ok(root.to_string());
    }

    Err(Error::InvalidCargoOutput)
}

fn process_crate(args: &Args) -> Result<CrateData, Error> {
    let workspace_root = get_workspace_root()?;

    println!("Compiling ...");

    let output = if args.time {
        // To collect the build times we have to clean the repo first.

        let clean_args = if args.release {
            // Remove only `target/release` in the release mode.
            vec!["clean", "--release"]
        } else {
            // We can't remove only `target/debug` in debug mode yet.
            // See https://github.com/rust-lang/cargo/pull/6989
            vec!["clean"]
        };

        // No need to check the output status.
        let _ = Command::new("cargo")
            .args(&clean_args)
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
        let build = json::parse(line).map_err(|_| Error::InvalidCargoOutput)?;
        if let Some(target_name) = build["target"]["name"].as_str() {
            if !build["filenames"].is_null() {
                let filenames = build["filenames"].members();
                let crate_types = build["target"]["crate_types"].members();
                for (path, crate_type) in filenames.zip(crate_types) {
                    let kind = match crate_type.as_str().unwrap() {
                        "bin" => ArtifactKind::Binary,
                        "lib" => ArtifactKind::Library,
                        "cdylib" => ArtifactKind::CDynLib,
                        _ => continue, // Simply ignore.
                    };

                    artifacts.push({
                        Artifact {
                            kind,
                            name: target_name.replace("-", "_"),
                            path: path::PathBuf::from(&path.as_str().unwrap()),
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
            let value = json::parse(&line[10..]).unwrap();

            times.push(Elapsed {
                crate_name: value["crate_name"].as_str().unwrap().to_string(),
                time: value["time"].as_u64().unwrap(),
                build_script: value["build_script"].as_bool().unwrap(),
            });
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
    } else if let Some(ref test) = args.test {
        list.push(format!("--test={}", test));
    }

    if let Some(ref package) = args.package {
        list.push(format!("--package={}", package));
    }

    if args.all_features {
        list.push("--all-features".to_string());
    } else {

        if args.no_default_features {
            list.push("--no-default-features".to_string());
        }

        if let Some(ref features) = args.features {
            list.push(format!("--features={}", features));
        }
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
        for sym in ar::parse(&file) {
            map.insert(sym, name.clone());
        }
    }

    for (_, v) in map.iter_all_mut() {
        v.dedup();
    }

    Ok(map)
}

fn collect_self_data(path: &path::Path) -> Result<Data, Error> {
    let file = map_file(&path)?;
    let data = &file;

    match data.get(0..4) {
        Some(b"\x7fELF") if data.len() >= 8 => {
            collect_elf_data(path, data)
        }
        Some(&[0xCF, 0xFA, 0xED, 0xFE]) => {
            collect_macho_data(path, data)
        }
        _ => {
            Err(Error::UnsupportedFileFormat(path.to_owned()))
        }
    }
}

fn collect_elf_data(path: &path::Path, data: &[u8]) -> Result<Data, Error> {
    let is_64_bit = match data[4] {
        1 => false,
        2 => true,
        _ => return Err(Error::UnsupportedFileFormat(path.to_owned())),
    };

    let byte_order = match data[5] {
        1 => parser::ByteOrder::LittleEndian,
        2 => parser::ByteOrder::BigEndian,
        _ => return Err(Error::UnsupportedFileFormat(path.to_owned())),
    };

    let symbols = if is_64_bit {
        elf64::parse(data, byte_order)
    } else {
        elf32::parse(data, byte_order)
    };

    let total_size = symbols.iter().map(|s| s.size).sum();

    let d = Data {
        symbols,
        file_size: fs::metadata(path).unwrap().len(),
        text_size: total_size,
    };

    Ok(d)
}

fn collect_macho_data(path: &path::Path, data: &[u8]) -> Result<Data, Error> {
    let symbols = macho::parse(data);
    let total_size = symbols.iter().map(|s| s.size).sum();

    let d = Data {
        symbols,
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
        #[cfg(feature = "regex-filter")]
        Regex(regex::Regex),
        #[cfg(not(feature = "regex-filter"))]
        Substring(String),
    }

    let filter = if let Some(ref text) = args.filter {
        if d.std_crates.contains(text) || d.dep_crates.contains(text) {
            FilterBy::Crate(text.clone())
        } else {
            #[cfg(feature = "regex-filter")]
            {
                match regex::Regex::new(text) {
                    Ok(re) => FilterBy::Regex(re),
                    Err(_) => {
                        eprintln!("Warning: the filter value contains an unknown crate \
                                   or an invalid regexp. Ignored.");
                        FilterBy::None
                    }
                }
            }

            #[cfg(not(feature = "regex-filter"))]
            {
                FilterBy::Substring(text.clone())
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

        let (mut crate_name, is_exact) = crate_name::from_sym(&d, args, &sym.name);

        if !is_exact {
            crate_name.push('?');
        }

        let name = if args.full_fn {
            sym.name.complete.clone()
        } else {
            sym.name.trimmed.clone()
        };

        match filter {
            FilterBy::None => {}
            FilterBy::Crate(ref crate_name_f) => {
                if crate_name_f != &crate_name {
                    continue;
                }
            }
            #[cfg(feature = "regex-filter")]
            FilterBy::Regex(ref re) => {
                if !re.is_match(&name) {
                    continue;
                }
            }
            #[cfg(not(feature = "regex-filter"))]
            FilterBy::Substring(ref s) => {
                if !name.contains(s) {
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

        if others_count > 0 {
            let percent_file_s = other_total as f64 / dd.file_size as f64 * 100.0;
            let percent_text_s = other_total as f64 / dd.text_size as f64 * 100.0;
            let name_s = format!("And {} smaller methods. Use -n N to show more.", others_count);
            push_row(table, percent_file_s, percent_text_s, other_total, String::new(), name_s);
        }
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

fn print_crates(d: CrateData, args: &Args, table: &mut Table) {
    let dd = &d.data;
    let mut sizes = HashMap::new();

    for sym in dd.symbols.iter() {
        let (crate_name, _) = crate_name::from_sym(&d, args, &sym.name);

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

    let n = if args.n == 0 { list.len() } else { args.n };
    for &(k, v) in list.iter().rev().take(n) {
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

    if n < list.len() {
        let mut other_total = 0;
        for &(_, v) in list.iter().rev().skip(n) {
            other_total += *v;
        }

        let time = if args.time {
            Some(String::new())
        } else {
            None
        };

        let percent_file_s = other_total as f64 / dd.file_size as f64 * 100.0;
        let percent_text_s = other_total as f64 / dd.text_size as f64 * 100.0;

        let name = format!("And {} more crates. Use -n N to show more.", list.len() - n);
        push_row(table, percent_file_s, percent_text_s, other_total, time, name);
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
