#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
extern crate core;
mod manifest;
mod manifest_type;
mod manifest_version;
mod manifests;
mod package_identifier;
mod package_version;
pub mod url;
pub mod utils;
pub mod version;

#[cfg(feature = "std")]
pub use camino;
pub use icu_locale;
pub use manifest::Manifest;
pub use manifest_type::{ManifestType, ManifestTypeWithLocale};
pub use manifest_version::ManifestVersion;
pub use manifests::*;
pub use package_family_name;
pub use package_identifier::{PackageIdentifier, PackageIdentifierError};
pub use package_version::{PackageVersion, PackageVersionError};
pub use sha2;
pub use utils::{language_tag::LanguageTag, sha_256::Sha256String};
pub use version::Version;

#[cfg(feature = "std")]
pub type PathBuf = typed_path::Utf8WindowsPathBuf;

#[cfg(not(feature = "std"))]
pub type PathBuf = alloc::string::String;

#[cfg(feature = "std")]
pub type Path = typed_path::Utf8WindowsPath;

#[cfg(not(feature = "std"))]
pub type Path = str;

pub const DISALLOWED_CHARACTERS: [char; 9] = ['\\', '/', ':', '*', '?', '\"', '<', '>', '|'];

#[cfg(all(test, feature = "std"))]
mod path_tests {
    use super::{Path, PathBuf};

    #[test]
    fn manifest_paths_use_windows_components_on_all_platforms() {
        assert_eq!(
            Path::new(r"C:\Program Files\VideoLAN\vlc.exe").file_name(),
            Some("vlc.exe")
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn manifest_paths_round_trip_through_yaml() {
        let path = PathBuf::from(r"C:\Program Files\VideoLAN\vlc.exe");
        let yaml = serde_yaml::to_string(&path).unwrap();
        let decoded: PathBuf = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(decoded, path);
    }
}
