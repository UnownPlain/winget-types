use alloc::string::{String, ToString};
use core::{fmt, str::FromStr};

use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum ValidFileExtensions {
    Msix,
    Msi,
    Appx,
    Exe,
    Zip,
    MsixBundle,
    AppxBundle,
    Otf,
    Ttf,
    Fnt,
    Ttc,
    Otc,
}

#[derive(Error, Debug, Eq, PartialEq)]
#[error("Invalid file extension: {extension}")]
pub struct ValidFileExtensionsError {
    pub extension: String,
}

impl ValidFileExtensions {
    pub const ALL: [Self; 12] = [
        Self::Msix,
        Self::Msi,
        Self::Appx,
        Self::Exe,
        Self::Zip,
        Self::MsixBundle,
        Self::AppxBundle,
        Self::Otf,
        Self::Ttf,
        Self::Fnt,
        Self::Ttc,
        Self::Otc,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Msix => "msix",
            Self::Msi => "msi",
            Self::Appx => "appx",
            Self::Exe => "exe",
            Self::Zip => "zip",
            Self::MsixBundle => "msixbundle",
            Self::AppxBundle => "appxbundle",
            Self::Otf => "otf",
            Self::Ttf => "ttf",
            Self::Fnt => "fnt",
            Self::Ttc => "ttc",
            Self::Otc => "otc",
        }
    }

    /// Returns whether the file type can be used as a nested installer in an archive.
    #[must_use]
    pub const fn is_valid_nested_installer(self) -> bool {
        !matches!(self, Self::Zip)
    }

    /// Extracts a valid installer extension from a path.
    #[cfg(feature = "std")]
    pub fn from_path(path: &camino::Utf8Path) -> Result<Self, ValidFileExtensionsError> {
        path.extension().unwrap_or_default().parse()
    }

    /// Finds the last path segment in a URL with a valid installer extension.
    ///
    /// Looking through all path segments supports download-wrapper URLs such as
    /// `https://example.com/file.msi/download`.
    #[must_use]
    pub fn from_url(url: &url::Url) -> Option<Self> {
        url.path_segments()?.rev().find_map(|segment| {
            segment.rsplit_once('.').and_then(|(file_stem, extension)| {
                if file_stem.is_empty() {
                    None
                } else {
                    extension.parse().ok()
                }
            })
        })
    }
}

impl AsRef<str> for ValidFileExtensions {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for ValidFileExtensions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl FromStr for ValidFileExtensions {
    type Err = ValidFileExtensionsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let extension = s.rsplit_once('.').map_or(s, |(_, extension)| extension);

        Self::ALL
            .into_iter()
            .find(|valid_extension| valid_extension.as_str().eq_ignore_ascii_case(extension))
            .ok_or_else(|| ValidFileExtensionsError {
                extension: s.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::ValidFileExtensions;

    #[test]
    fn parses_extensions_case_insensitively() {
        assert_eq!("MSIXBUNDLE".parse(), Ok(ValidFileExtensions::MsixBundle));
        assert_eq!(".Exe".parse(), Ok(ValidFileExtensions::Exe));
        assert_eq!("installer.x64.MSI".parse(), Ok(ValidFileExtensions::Msi));
    }

    #[test]
    fn only_zip_is_not_a_valid_nested_installer() {
        for extension in ValidFileExtensions::ALL {
            assert_eq!(
                extension.is_valid_nested_installer(),
                extension != ValidFileExtensions::Zip
            );
        }
    }

    #[test]
    fn extracts_extension_from_download_wrapper_url() {
        let url = url::Url::parse("https://example.com/releases/installer.MSI/download").unwrap();

        assert_eq!(
            ValidFileExtensions::from_url(&url),
            Some(ValidFileExtensions::Msi)
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn extracts_extension_from_path() {
        assert_eq!(
            ValidFileExtensions::from_path(camino::Utf8Path::new("dir/installer.EXE")),
            Ok(ValidFileExtensions::Exe)
        );
    }
}
