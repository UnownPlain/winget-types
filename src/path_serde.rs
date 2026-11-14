use serde::{Serialize, Serializer};
use typed_path::{Utf8WindowsPath, Utf8WindowsPathBuf};

use crate::{Path, PathBuf};

fn normalize(path: &Path) -> Utf8WindowsPathBuf {
    let path: &str = path.as_ref();
    Utf8WindowsPath::new(path).normalize()
}

pub(crate) fn serialize<S: Serializer>(path: &Path, serializer: S) -> Result<S::Ok, S::Error> {
    normalize(path).serialize(serializer)
}

pub(crate) fn serialize_optional<S: Serializer>(
    path: &Option<PathBuf>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    path.as_deref().map(normalize).serialize(serializer)
}

// Only repeated portable aliases need distinct spellings of the same executable.
pub(crate) fn serialize_nested_files<S: Serializer>(
    files: &alloc::collections::BTreeSet<crate::installer::NestedInstallerFiles>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use alloc::{
        collections::{BTreeMap, BTreeSet},
        string::String,
        vec::Vec,
    };

    use serde::ser::SerializeStruct;

    struct File<'a> {
        path: String,
        original: &'a crate::installer::NestedInstallerFiles,
    }
    impl Serialize for File<'_> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut state = serializer.serialize_struct("NestedInstallerFiles", 2)?;
            state.serialize_field("RelativeFilePath", &self.path)?;
            if let Some(alias) = &self.original.portable_command_alias {
                state.serialize_field("PortableCommandAlias", alias)?;
            }
            state.end()
        }
    }

    // Existing leading dot components opt this executable into the winget-pkgs workaround.
    let disambiguated_paths = files
        .iter()
        .filter(|file| {
            let path = file.relative_file_path.as_str();
            path.starts_with(r".\") || path.starts_with("./")
        })
        .map(|file| String::from(normalize(&file.relative_file_path).as_str()))
        .collect::<BTreeSet<_>>();
    let mut files = files.iter().collect::<Vec<_>>();
    // Retain the unprefixed entry first, then reuse existing prefix ordering.
    files.sort_by_key(|file| {
        let path = file.relative_file_path.as_str();
        let dots = path
            .split(['/', '\\'])
            .take_while(|part| *part == ".")
            .count();
        (
            normalize(&file.relative_file_path),
            dots,
            &file.portable_command_alias,
        )
    });
    let mut aliases = BTreeMap::new();
    let files = files
        .into_iter()
        .map(|file| {
            let path = String::from(normalize(&file.relative_file_path).as_str());
            let use_prefixes = disambiguated_paths.contains(&path);
            let group = aliases.entry(path).or_insert_with(Vec::new);
            let prefix_count = if let Some(alias) = &file.portable_command_alias {
                if let Some(index) = group.iter().position(|existing| *existing == alias) {
                    index
                } else {
                    group.push(alias);
                    group.len() - 1
                }
            } else {
                0
            };
            let path = alloc::format!(
                "{}{}",
                r".\".repeat(if use_prefixes { prefix_count } else { 0 }),
                normalize(&file.relative_file_path)
            );
            File {
                path,
                original: file,
            }
        })
        .collect::<Vec<_>>();
    files.serialize(serializer)
}

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeSet, vec::Vec};

    use crate::installer::{Installer, InstallerManifest, NestedInstallerFiles};

    #[rstest::rstest]
    #[case(true)]
    #[case(false)]
    fn nested_aliases_use_minimum_dot_prefixes(#[case] use_prefixes: bool) {
        for count in 1..=4 {
            let files = (0..count)
                .map(|index| NestedInstallerFiles {
                    relative_file_path: if use_prefixes {
                        alloc::format!("{}bin/nub.exe", "./".repeat(index + 2)).into()
                    } else {
                        "bin/./nub.exe".into()
                    },
                    portable_command_alias: Some(alloc::format!("nub{index}").parse().unwrap()),
                })
                .chain(core::iter::once(NestedInstallerFiles {
                    relative_file_path: r".\bin\other.exe".into(),
                    portable_command_alias: Some("other".parse().unwrap()),
                }))
                .collect::<BTreeSet<_>>();
            let installer = Installer {
                nested_installer_files: files.clone(),
                ..Installer::default()
            };
            let manifest = InstallerManifest {
                nested_installer_files: files,
                ..InstallerManifest::default()
            };
            for yaml in [
                serde_yaml::to_string(&installer).unwrap(),
                serde_yaml::to_string(&manifest).unwrap(),
            ] {
                let value: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
                let files = value["NestedInstallerFiles"].as_sequence().unwrap();
                let paths = files
                    .iter()
                    .filter(|file| {
                        file["PortableCommandAlias"]
                            .as_str()
                            .unwrap()
                            .starts_with("nub")
                    })
                    .map(|file| file["RelativeFilePath"].as_str().unwrap())
                    .collect::<Vec<_>>();
                assert_eq!(paths.len(), count);
                for (index, path) in paths.into_iter().enumerate() {
                    assert_eq!(
                        path,
                        alloc::format!(
                            "{}bin\\nub.exe",
                            r".\".repeat(if use_prefixes { index } else { 0 })
                        )
                    );
                }
                assert!(
                    files
                        .iter()
                        .any(|file| file["RelativeFilePath"].as_str() == Some(r"bin\other.exe"))
                );
            }
        }
    }
}
