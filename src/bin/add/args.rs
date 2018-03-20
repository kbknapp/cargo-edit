//! Handle `cargo add` arguments

use cargo_edit::Dependency;
use cargo_edit::{get_latest_dependency, CrateName};
use semver;
use std::path::PathBuf;
use clap;

use errors::*;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DepKind {  
    Build,      
    Dev,
    Optional,
    Normal
}

#[derive(Debug)]
/// clap input args.
pub struct Args {
    /// Crate name(s)
    pub arg_crates: Vec<String>,
    /// Dep kind
    pub dep_kind: DepKind,
    /// Version
    pub flag_vers: Option<String>,
    /// Git repo Path
    pub flag_git: Option<String>,
    /// Crate directory path
    pub flag_path: Option<PathBuf>,
    /// Crate directory path
    pub flag_target: Option<String>,
    /// `Cargo.toml` path
    pub flag_manifest_path: Option<PathBuf>,
    /// `---upgrade`
    pub flag_upgrade: String,
    /// '--fetch-prereleases'
    pub flag_allow_prerelease: bool,
    /// '--quiet'
    pub flag_quiet: bool,
}

impl Args {
    /// Get dependency section
    pub fn get_section(&self) -> Vec<String> {
        match self.dep_kind {
            DepKind::Dev => {
                vec!["dev-dependencies".to_owned()]
            },
            DepKind::Build => {
                vec!["build-dependencies".to_owned()]
            },
            DepKind::Normal | DepKind::Optional => {
                if let Some(ref target) = self.flag_target {
                    if target.is_empty() {
                        panic!("Target specification may not be empty");
                    }
                    vec![
                        "target".to_owned(),
                        target.clone(),
                        "dependencies".to_owned(),
                    ]
                } else {
                    vec!["dependencies".to_owned()]
                }
            }
        }
    }

    /// Build dependencies from arguments
    pub fn parse_dependencies(&self) -> Result<Vec<Dependency>> {
        if self.arg_crates.len() > 1 {
            return self.arg_crates
                .iter()
                .map(|crate_name| {
                    Ok(
                        if let Some(krate) = CrateName::new(crate_name).parse_as_version()? {
                            krate
                        } else {
                            get_latest_dependency(crate_name, self.flag_allow_prerelease)?
                        }.set_optional(self.dep_kind == DepKind::Optional),
                    )
                })
                .collect();
        }

        let crate_name = CrateName::new(&self.arg_crates[0]);

        let dependency = if let Some(krate) = crate_name.parse_as_version()? {
            krate
        } else if !crate_name.is_url_or_path() {
            let dependency = Dependency::new(&self.arg_crates[0]);

            if let Some(ref version) = self.flag_vers {
                semver::VersionReq::parse(version)
                    .chain_err(|| "Invalid dependency version requirement")?;
                dependency.set_version(version)
            } else if let Some(ref repo) = self.flag_git {
                dependency.set_git(repo)
            } else if let Some(ref path) = self.flag_path {
                dependency.set_path(path.to_str().unwrap())
            } else {
                let dep = get_latest_dependency(&self.arg_crates[0], self.flag_allow_prerelease)?;
                let v = format!(
                    "{prefix}{version}",
                    prefix = self.get_upgrade_prefix(),
                    // If version is unavailable `get_latest_dependency` must have
                    // returned `Err(FetchVersionError::GetVersion)`
                    version = dep.version().unwrap_or_else(|| unreachable!())
                );
                dep.set_version(&v)
            }
        } else {
            crate_name.parse_crate_name_from_uri()?
        }.set_optional(self.dep_kind == DepKind::Optional);

        Ok(vec![dependency])
    }

    fn get_upgrade_prefix(&self) -> &'static str {
        match &*(&*self.flag_upgrade).to_uppercase() {
            "NONE" => "=",
            "PATCH" => "~",
            "MINOR" => "^",
            "ALL" => ">=",
            _ => unreachable!()
        }
    }
}

impl Default for Args {
    fn default() -> Args {
        Args {
            arg_crates: vec!["demo".to_owned()],
            dep_kind: DepKind::Normal,
            flag_vers: None,
            flag_git: None,
            flag_path: None,
            flag_target: None,
            flag_manifest_path: None,
            flag_upgrade: "^".to_owned(),
            flag_allow_prerelease: false,
            flag_quiet: false,
        }
    }
}

impl<'a> From<&'a clap::ArgMatches<'a>> for Args {
    fn from(m: &'a clap::ArgMatches<'a>) -> Self {
        Args {
            arg_crates: m.values_of("crates").unwrap().map(ToOwned::to_owned).collect(),
            dep_kind: if m.is_present("dev") { 
                DepKind::Dev 
            } else if m.is_present("optional") { 
                DepKind::Optional 
            } else if m.is_present("build") { 
                DepKind::Build 
            } else {
                DepKind::Normal
            },
            flag_vers: m.value_of("vers").map(ToOwned::to_owned),
            flag_git: m.value_of("git").map(ToOwned::to_owned),
            flag_path: m.value_of("path").map(PathBuf::from),
            flag_target: m.value_of("target").map(ToOwned::to_owned),
            flag_manifest_path: m.value_of("manifest-path").map(PathBuf::from),
            flag_upgrade: m.value_of("upgrade").map(ToOwned::to_owned).unwrap(),
            flag_allow_prerelease: m.is_present("allow-prerelease"),
            flag_quiet: m.is_present("quiet"),
        }
    }
}

#[cfg(test)]
mod tests {
    use cargo_edit::Dependency;
    use super::*;

    #[test]
    fn test_dependency_parsing() {
        let args = Args {
            flag_vers: Some("0.4.2".to_owned()),
            ..Args::default()
        };

        assert_eq!(
            args.parse_dependencies().unwrap(),
            vec![Dependency::new("demo").set_version("0.4.2")]
        );
    }

    #[test]
    #[cfg(feature = "test-external-apis")]
    fn test_repo_as_arg_parsing() {
        let github_url = "https://github.com/killercup/cargo-edit/";
        let args_github = Args {
            arg_crate: github_url.to_owned(),
            ..Args::default()
        };
        assert_eq!(
            args_github.parse_dependencies().unwrap(),
            vec![Dependency::new("cargo-edit").set_git(github_url)]
        );

        let gitlab_url = "https://gitlab.com/Polly-lang/Polly.git";
        let args_gitlab = Args {
            arg_crate: gitlab_url.to_owned(),
            ..Args::default()
        };
        assert_eq!(
            args_gitlab.parse_dependencies().unwrap(),
            vec![Dependency::new("polly").set_git(gitlab_url)]
        );
    }

    #[test]
    fn test_path_as_arg_parsing() {
        let self_path = ".";
        let args_path = Args {
            arg_crates: vec![self_path.to_owned()],
            ..Args::default()
        };
        assert_eq!(
            args_path.parse_dependencies().unwrap(),
            vec![Dependency::new("cargo-edit").set_path(self_path)]
        );
    }

}
