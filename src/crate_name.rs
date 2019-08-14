use crate::{CrateData, Args};
use crate::demangle::{self, SymbolName};

const UNKNOWN: &str = "[Unknown]";

pub(crate) fn from_sym(d: &CrateData, args: &Args, sym: &SymbolName) -> (String, bool) {
    let (mut name, is_exact) = from_sym_impl(d, sym);

    if !args.split_std {
        if d.std_crates.contains(&name) {
            name = "std".to_string();
        }
    }

    (name, is_exact)
}

fn from_sym_impl(d: &CrateData, sym: &SymbolName) -> (String, bool) {
    if let Some(name) = d.deps_symbols.get(&sym.complete) {
        return (name.to_string(), true);
    }

    match sym.kind {
        demangle::Kind::Legacy => {
            parse_sym(d, &sym.complete)
        }
        demangle::Kind::V0 => {
            match sym.crate_name {
                Some(ref name) => (name.to_string(), true),
                None => parse_sym_v0(d, &sym.trimmed),
            }
        }
        demangle::Kind::Unknown => {
            (UNKNOWN.to_string(), true)
        }
    }
}

// A simple stupid symbol parser.
// Should be replaced by something better later.
fn parse_sym(d: &CrateData, sym: &str) -> (String, bool) {
    // TODO: ` for `

    let mut is_exact = true;
    let name = if sym.contains(" as ") {
        let parts: Vec<_> = sym.split(" as ").collect();
        let crate_name1 = parse_crate_from_sym(parts[0]);
        let crate_name2 = parse_crate_from_sym(parts[1]);

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
                    if names.contains(&crate_name1) {
                        crate_name1.clone()
                    } else if names.contains(&crate_name2) {
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
    } else {
        parse_crate_from_sym(&sym)
    };

    (name, is_exact)
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

        while crate_name.starts_with("&") {
            crate_name.remove(0);
        }

        crate_name = crate_name.split_whitespace().last().unwrap().to_owned();
    }

    crate_name
}

fn parse_sym_v0(d: &CrateData, sym: &str) -> (String, bool) {
    let name = parse_crate_from_sym(sym);

    // Check that such crate name is an actual dependency.
    // This is required to filter some obscure symbols like:
    // <str>::replace::<&str>
    if d.std_crates.contains(&name) || d.dep_crates.contains(&name) {
        (name, false)
    } else {
        (UNKNOWN.to_string(), true)
    }
}
