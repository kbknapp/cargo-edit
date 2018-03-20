//! `cargo rm`
#![warn(missing_docs, missing_debug_implementations, missing_copy_implementations, trivial_casts,
        trivial_numeric_casts, unsafe_code, unstable_features, unused_import_braces,
        unused_qualifications)]

extern crate atty;
extern crate clap;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;
extern crate termcolor;

use std::process;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

extern crate cargo_edit;
use cargo_edit::Manifest;

mod args;
use args::Args;

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

static ARGS: &'static str = r#"
    -D --dev                'Remove crate as development dependency.'
    -B --build              'Remove crate as build dependency.'
    --manifest-path=[path]  'Path to the manifest to remove a dependency from.'
    -q --quiet              'Do not print any output in case of success.'
    <crate>                 'The crate to remove'"#;

static USAGE: &'static str = r#"cargo rm <crate> [--dev|--build] [options]
    cargo rm (-h|--help)
    cargo rm --version
"#;

fn print_msg(name: &str, section: &str) -> Result<()> {
    let colorchoice = if atty::is(atty::Stream::Stdout) {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    };
    let mut output = StandardStream::stdout(colorchoice);
    output.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
    write!(output, "{:>12}", "Removing")?;
    output.reset()?;
    writeln!(output, " {} from {}", name, section)?;
    Ok(())
}

fn handle_rm(args: &Args) -> Result<()> {
    let manifest_path = args.flag_manifest_path.as_ref().map(From::from);
    let mut manifest = Manifest::open(&manifest_path)?;

    if !args.flag_quiet {
        print_msg(&args.arg_crate, args.get_section())?;
    }

    manifest
        .remove_from_table(args.get_section(), args.arg_crate.as_ref())
        .map_err(From::from)
        .and_then(|_| {
            let mut file = Manifest::find_file(&manifest_path)?;
            manifest.write_to_file(&mut file)?;

            Ok(())
        })
}

fn main() {
    let args: Args = clap::App::new("cargo-edit-rm")
        .bin_name("cargo")
        .setting(clap::AppSettings::SubcommandRequired)
        .subcommand(clap::SubCommand::with_name("rm")
            .usage(USAGE)
            .about("Remove a dependency from a Cargo.toml manifest file.")
            .version(&*format!("version {}", env!("CARGO_PKG_VERSION")))
            .setting(clap::AppSettings::UnifiedHelpMessage)
            .args_from_usage(ARGS)
            .group(clap::ArgGroup::with_name("type")
                .args(&["dev", "build"]))
        )
        .get_matches()
        .subcommand_matches("rm")
        .unwrap()
        .into();

    if let Err(err) = handle_rm(&args) {
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
