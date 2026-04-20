//! semverx CLI — minimal surface matching the v0.1.0 shipping library.
//!
//! This binary is deliberately small: it exposes only what `src/lib.rs`
//! actually ships (Version, Component/SemverXResolver, BindingMode). The
//! richer CLI (`health`, `audit`, `polybuild`, REPL, etc.) is tracked in the
//! Roadmap section of `README.md` and depends on the SEI / StressZone /
//! VerbNounClass / BubblingError type system that has not yet landed in the
//! `lib` crate. See OBINexus milestone-based investment policy — surfaces
//! are shipped or the repo surfaces the gap, not hidden.
//!
//! Zero third-party deps beyond the library itself, so this target compiles
//! with the same `cargo build` as the lib.

use std::env;
use std::process::ExitCode;

use semverx::core::Version;
use semverx::ffi::BindingMode;
use semverx::resolver::{Component, DependencyResolver, SemverXResolver};

const USAGE: &str = "\
semverx 0.1.0 — Semantic Versioning eXtended (OBINexus Computing)

Usage:
    semverx <command> [args]

Commands:
    parse <version>                 Parse a SemVer 2.0 string and print its fields.
    compare <v1> <v2>               Compare two versions; prints -1 / 0 / 1.
    resolve <pkg> <version>         Resolve a package against an in-memory graph.
    binding-mode                    Print the default polyglot binding mode.
    help | --help | -h              Show this message.
    version | --version | -V        Print the CLI version.

Examples:
    semverx parse 1.2.3-beta.1+build.42
    semverx compare 1.0.0 1.0.1
    semverx resolve foo 1.0.0
    semverx binding-mode
";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }

    let cmd = args[0].as_str();
    let rest = &args[1..];

    match cmd {
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        "version" | "--version" | "-V" => {
            println!("semverx {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        "parse" => cmd_parse(rest),
        "compare" => cmd_compare(rest),
        "resolve" => cmd_resolve(rest),
        "binding-mode" => cmd_binding_mode(),
        other => {
            eprintln!("semverx: unknown command '{other}'\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn cmd_parse(args: &[String]) -> ExitCode {
    let Some(version) = args.first() else {
        eprintln!("semverx parse: missing <version>");
        return ExitCode::from(2);
    };

    match Version::parse(version) {
        Ok(v) => {
            println!("parsed:");
            println!("  major: {}", v.major);
            println!("  minor: {}", v.minor);
            println!("  patch: {}", v.patch);
            match &v.pre {
                Some(p) => println!("  pre:   {p}"),
                None => println!("  pre:   -"),
            }
            match &v.build {
                Some(b) => println!("  build: {b}"),
                None => println!("  build: -"),
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("semverx parse: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_compare(args: &[String]) -> ExitCode {
    if args.len() < 2 {
        eprintln!("semverx compare: usage: semverx compare <v1> <v2>");
        return ExitCode::from(2);
    }

    let v1 = match Version::parse(&args[0]) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("semverx compare: v1: {e}");
            return ExitCode::FAILURE;
        }
    };
    let v2 = match Version::parse(&args[1]) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("semverx compare: v2: {e}");
            return ExitCode::FAILURE;
        }
    };

    let ord = v1.cmp(&v2);
    let code = match ord {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    };
    println!("{code}");
    ExitCode::SUCCESS
}

fn cmd_resolve(args: &[String]) -> ExitCode {
    if args.len() < 2 {
        eprintln!("semverx resolve: usage: semverx resolve <pkg> <version>");
        return ExitCode::from(2);
    }
    let pkg = &args[0];
    let version = &args[1];

    // Minimal demo graph: the requested package resolves to itself until the
    // full constraint engine (Roadmap: SEI / StressZone / VerbNounClass)
    // lands in the `lib` crate.
    let mut resolver = SemverXResolver::new();
    resolver.add_package(Component {
        name: pkg.to_string(),
        version: version.to_string(),
        dependencies: Vec::new(),
    });

    match resolver.resolve_dependencies(pkg, version) {
        Ok(components) => {
            if components.is_empty() {
                println!("(no dependencies resolved)");
            } else {
                for c in &components {
                    println!("{}@{}", c.name, c.version);
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("semverx resolve: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_binding_mode() -> ExitCode {
    // Default binding mode for this build. Mirrors the tri-binding model in
    // README.md (monoglot / polyglot / hybrid).
    let mode = BindingMode::Hybrid;
    let name = match mode {
        BindingMode::Monoglot => "monoglot",
        BindingMode::Polyglot => "polyglot",
        BindingMode::Hybrid => "hybrid",
    };
    println!("{name}");
    ExitCode::SUCCESS
}
