//! Handle `cargo rm` arguments

use clap;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DepKind {
    Build,
    Dev,
    Normal
}

#[derive(Debug)]
/// Docopts input args.
pub struct Args {
    /// Crate name
    pub arg_crate: String,
    /// dep kind
    pub dep_kind: DepKind,
    /// `Cargo.toml` path
    pub flag_manifest_path: Option<String>,
    /// '--quiet'
    pub flag_quiet: bool,
}

impl Args {
    /// Get depenency section
    pub fn get_section(&self) -> &'static str {
        match self.dep_kind {
            DepKind::Dev => "dev-dependencies",
            DepKind::Build => "build-dependencies",
            DepKind::Normal =>  "dependencies",
        }
    }
}

impl Default for Args {
    fn default() -> Args {
        Args {
            arg_crate: "demo".to_owned(),
            dep_kind: DepKind::Normal,
            flag_manifest_path: None,
            flag_quiet: false,
        }
    }
}

impl<'a> From<&'a clap::ArgMatches<'a>> for Args {
    fn from(m: &'a clap::ArgMatches<'a>) -> Self {
        Args {
            arg_crate: m.value_of("crate").unwrap().to_owned(),
            dep_kind: if m.is_present("build") {
                DepKind::Build
            } else if m.is_present("dev") {
                DepKind::Dev
            } else {
                DepKind::Normal
            },
            flag_manifest_path: m.value_of("manifest-path").map(ToOwned::to_owned),
            flag_quiet: m.is_present("quiet"),
        }
    }
}
