use super::portable_command_alias::PortableCommandAlias;
use crate::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
pub struct NestedInstallerFiles {
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "crate::path_serde::serialize")
    )]
    pub relative_file_path: PathBuf,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub portable_command_alias: Option<PortableCommandAlias>,
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use rstest::rstest;

    use super::NestedInstallerFiles;

    #[rstest]
    #[case("app/bin/app.exe", r"app\bin\app.exe")]
    #[case(r"app\bin/app.exe", r"app\bin\app.exe")]
    #[case(r"app\bin\app.exe", r"app\bin\app.exe")]
    #[case("app/./bin/../app.exe/", r"app\app.exe")]
    #[case("app//bin/app.exe", r"app\bin\app.exe")]
    fn nested_installer_files_serialize_windows_paths(#[case] input: &str, #[case] expected: &str) {
        let file = NestedInstallerFiles {
            relative_file_path: input.into(),
            portable_command_alias: None,
        };
        let yaml = serde_yaml::to_string(&file).unwrap();
        assert_eq!(
            yaml.trim_end(),
            alloc::format!("RelativeFilePath: {expected}")
        );

        #[cfg(feature = "serde-saphyr")]
        assert_eq!(serde_saphyr::to_string(&file).unwrap(), yaml);

        let decoded: NestedInstallerFiles = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(decoded.relative_file_path.as_str(), expected);
    }

    #[cfg(feature = "std")]
    #[test]
    fn native_paths_serialize_as_windows_paths() {
        let native = camino::Utf8PathBuf::from("app").join("bin").join("app.exe");
        let file = NestedInstallerFiles {
            relative_file_path: native.as_str().into(),
            portable_command_alias: None,
        };

        assert_eq!(
            serde_yaml::to_string(&file).unwrap().trim_end(),
            r"RelativeFilePath: app\bin\app.exe"
        );
    }
}
