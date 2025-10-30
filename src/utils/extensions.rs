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
    pub const ALL: [&'static str; 12] = [
        "msix",
        "msi",
        "appx",
        "exe",
        "zip",
        "msixbundle",
        "appxbundle",
        "otf",
        "ttf",
        "fnt",
        "ttc",
        "otc",
    ];

    #[must_use]
    pub fn as_str(&self) -> &'static str {
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
        match s {
            "msix" => Ok(Self::Msix),
            "msi" => Ok(Self::Msi),
            "appx" => Ok(Self::Appx),
            "exe" => Ok(Self::Exe),
            "zip" => Ok(Self::Zip),
            "msixbundle" => Ok(Self::MsixBundle),
            "appxbundle" => Ok(Self::AppxBundle),
            "otf" => Ok(Self::Otf),
            "ttf" => Ok(Self::Ttf),
            "fnt" => Ok(Self::Fnt),
            "ttc" => Ok(Self::Ttc),
            "otc" => Ok(Self::Otc),
            _ => Err(ValidFileExtensionsError {
                extension: s.to_string(),
            }),
        }
    }
}
