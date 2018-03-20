//! `cargo add`
#![warn(missing_docs, missing_debug_implementations, missing_copy_implementations, trivial_casts,
        trivial_numeric_casts, unsafe_code, unstable_features, unused_import_braces,
        unused_qualifications)]

extern crate atty;
extern crate clap;
#[macro_use]
extern crate error_chain;
extern crate semver;
#[macro_use]
extern crate serde_derive;
extern crate termcolor;

use std::process;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

extern crate cargo_edit;
use cargo_edit::{Dependency, Manifest};

mod args;
use args::Args;
use args::DepKind;

mod errors {
    error_chain!{
        links {
            CargoEditLib(::cargo_edit::Error, ::cargo_edit::ErrorKind);
        }
        foreign_links {
            Io(::std::io::Error);
        }
    }
}
use errors::*;

static USAGE: &'static str = r#"cargo add <crate> [--dev|--build|--optional] [--vers=<ver>|--git=<uri>|--path=<uri>] [options]
    cargo add <crates>... [--dev|--build|--optional] [options]
    cargo add (-h|--help)
    cargo add --version
"#;

static ABOUT: &'static str = r#"This command allows you to add a dependency to a Cargo.toml manifest file. If <crate> is a github
or gitlab repository URL, or a local path, `cargo add` will try to automatically get the crate name
and set the appropriate `--git` or `--path` value."#;

static AFTER_HELP: &'static str = r#"

Please note that Cargo treats versions like "1.2.3" as "^1.2.3" (and that "^1.2.3" is specified
as ">=1.2.3 and <2.0.0"). By default, `cargo add` will use this format, as it is the one that the
crates.io registry suggests. One goal of `cargo add` is to prevent you from using wildcard
dependencies (version set to "*").
"#;

static ARGS: &'static str = "
--vers [ver]            'Specify the version to grab from the registry (crates.io). \
                        You can also specify versions as part of the name, e.g \
                        `cargo add bitflags@0.3.2`.'
--git [uri]             'Specify a git repository to download the crate from.'
-D --dev                'Add crate as development dependency.'
-B --build              'Add crate as build dependency.'
--optional              'Add as an optional dependency (for use in features). This does not work \
                        for `dev-dependencies` or `build-dependencies`.'
--target [target]       'Add as dependency to the given target platform. This does not work \
                        for `dev-dependencies` or `build-dependencies`.'
--manifest-path [path]  'Path to the manifest to add a dependency to.'
--allow-prerelease      'Include prerelease versions when fetching from crates.io (e.g. \
                        \"0.6.0-alpha\"). Defaults to false.'
-q --quiet              'Do not print any output in case of success.'
<crates>...             'The crate(s) to add'";

fn print_msg(dep: &Dependency, section: &[String], optional: bool) -> Result<()> {
    let colorchoice = if atty::is(atty::Stream::Stdout) {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    };
    let mut output = StandardStream::stdout(colorchoice);
    output.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
    write!(output, "{:>12}", "Adding")?;
    output.reset()?;
    write!(output, " {}", dep.name)?;
    if let Some(version) = dep.version() {
        write!(output, " v{}", version)?;
    } else {
        write!(output, " (unknown version)")?;
    }
    write!(output, " to")?;
    if optional {
        write!(output, " optional")?;
    }
    let section = if section.len() == 1 {
        section[0].clone()
    } else {
        format!("{} for target `{}`", &section[2], &section[1])
    };
    writeln!(output, " {}", section)?;
    Ok(())
}

fn handle_add(args: &Args) -> Result<()> {
    let manifest_path = args.flag_manifest_path.as_ref().map(From::from);
    let mut manifest = Manifest::open(&manifest_path)?;
    let deps = &args.parse_dependencies()?;

    deps.iter()
        .map(|dep| {
            if !args.flag_quiet {
                print_msg(dep, &args.get_section(), args.dep_kind == DepKind::Optional)?;
            }
            manifest
                .insert_into_table(&args.get_section(), dep)
                .map_err(Into::into)
        })
        .collect::<Result<Vec<_>>>()
        .map_err(|err| {
            eprintln!("Could not edit `Cargo.toml`.\n\nERROR: {}", err);
            err
        })?;

    let mut file = Manifest::find_file(&manifest_path)?;
    manifest.write_to_file(&mut file)?;

    Ok(())
}

fn main() {
    let args: Args = clap::App::new("cargo-edit-add")
        .bin_name("cargo")
        .setting(clap::AppSettings::SubcommandRequired)
        .subcommand(clap::SubCommand::with_name("add")
            .usage(USAGE)
            .about(ABOUT)
            .version(&*format!("version {}", env!("CARGO_PKG_VERSION")))
            .setting(clap::AppSettings::UnifiedHelpMessage)
            .args_from_usage(ARGS)
            .arg(clap::Arg::from_usage("--upgrade=[method]   'Choose method of semantic version upgrade.'")
                .long_help(
                    "Choose a method of semantic version upgrades. The modifiers for the various \
                    methods are\n\t\
                        '=' (none) which is an exact match\n\t\
                        '~' (patch)\n\t\
                        '^' (minor)\n\t\
                        '>=' (all)).")
                .case_insensitive(true)
                .possible_values(&["none", "patch", "minor", "all"])
                .default_value("minor"))
            .arg(
                clap::Arg::from_usage(
                    "--path [uri]  'Specify the path the crate should be loaded from.'"
                )
                .conflicts_with("git"))
            .group(clap::ArgGroup::with_name("type")
                .args(&["dev", "build", "optional"]))
            .after_help(AFTER_HELP)
        )
        .get_matches()
        .subcommand_matches("add")
        .unwrap()
        .into();

    if let Err(err) = handle_add(&args) {
        eprintln!("Command failed due to unhandled error: {}\n", err);

        for e in err.iter().skip(1) {
            eprintln!("Caused by: {}", e);
        }

        if let Some(backtrace) = err.backtrace() {
            eprintln!("Backtrace: {:?}", backtrace);
        }

        process::exit(1);
    }
}
