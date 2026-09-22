mod installed_file;

use alloc::collections::BTreeSet;

use installed_file::InstalledFile;

use crate::PathBuf;

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
pub struct InstallationMetadata {
    /// The default install location for the package.
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            serialize_with = "crate::path_serde::serialize_optional"
        )
    )]
    default_install_location: Option<PathBuf>,

    /// The files installed for the package.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "BTreeSet::is_empty", default)
    )]
    files: BTreeSet<InstalledFile>,
}

impl InstallationMetadata {
    /// Creates a new [`InstallationMetadata`] from a default install location
    /// and an iterator of [`InstalledFile`].
    #[must_use]
    pub fn new<P, I>(default_install_location: P, files: I) -> Self
    where
        P: Into<Option<PathBuf>>,
        I: IntoIterator<Item = InstalledFile>,
    {
        Self {
            default_install_location: default_install_location.into(),
            files: files.into_iter().collect(),
        }
    }

    /// Creates a new [`InstallationMetadata`] from a default install location.
    #[must_use]
    pub fn new_install_location<P>(default_install_location: P) -> Self
    where
        P: Into<Option<PathBuf>>,
    {
        Self {
            default_install_location: default_install_location.into(),
            files: BTreeSet::new(),
        }
    }

    /// Returns `true` if `default_install_location` is `None` and `files` is
    /// empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.default_install_location.is_none() && self.files.is_empty()
    }
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use rstest::rstest;

    use super::{InstallationMetadata, installed_file::InstalledFile};
    use crate::PathBuf;

    #[rstest]
    #[case(
        "C:/Program Files/App",
        "bin/app.exe",
        r"C:\Program Files\App",
        r"bin\app.exe"
    )]
    #[case(
        r"C:\Program Files/App",
        r"bin/app.exe",
        r"C:\Program Files\App",
        r"bin\app.exe"
    )]
    #[case(
        r"C:\Program Files\App",
        r"bin\app.exe",
        r"C:\Program Files\App",
        r"bin\app.exe"
    )]
    fn installation_metadata_serializes_windows_paths(
        #[case] location: &str,
        #[case] file: &str,
        #[case] expected_location: &str,
        #[case] expected_file: &str,
    ) {
        let metadata = InstallationMetadata::new(
            PathBuf::from(location),
            [InstalledFile::builder().file_path(file.into()).build()],
        );
        let yaml = serde_yaml::to_string(&metadata).unwrap();
        let expected = alloc::format!(
            "DefaultInstallLocation: {expected_location}\nFiles:\n- RelativeFilePath: {expected_file}\n"
        );
        assert_eq!(yaml, expected);

        #[cfg(feature = "serde-saphyr")]
        assert_eq!(serde_saphyr::to_string(&metadata).unwrap(), expected);

        let decoded: InstallationMetadata = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(
            decoded.default_install_location.unwrap().as_str(),
            expected_location
        );
        let decoded_file: &str = decoded.files.first().unwrap().relative_file_path().as_ref();
        assert_eq!(decoded_file, expected_file);
    }

    #[test]
    fn absent_install_location_stays_omitted() {
        assert_eq!(
            serde_yaml::to_string(&InstallationMetadata::default())
                .unwrap()
                .trim_end(),
            "{}"
        );
    }
}
