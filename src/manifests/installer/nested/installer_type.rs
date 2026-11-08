use core::fmt;

use crate::installer::InstallerType;

/// Enumeration of supported nested installer shared contained inside an archive file
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[non_exhaustive]
pub enum NestedInstallerType {
    Msix,
    Msi,
    Appx,
    Exe,
    Inno,
    Nullsoft,
    Wix,
    Burn,
    AdvancedInstaller,
    Squirrel,
    Velopack,
    Portable,
    Font,
}

impl NestedInstallerType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Msix => "msix",
            Self::Msi => "msi",
            Self::Appx => "appx",
            Self::Exe => "exe",
            Self::Inno => "inno",
            Self::Nullsoft => "nullsoft",
            Self::Wix => "wix",
            Self::Burn => "burn",
            Self::AdvancedInstaller | Self::Squirrel | Self::Velopack => "exe",
            Self::Portable => "portable",
            Self::Font => "font",
        }
    }

    #[must_use]
    pub const fn installer_technology(self) -> Option<&'static str> {
        match self {
            Self::AdvancedInstaller => Some("Advanced Installer"),
            Self::Squirrel => Some("Squirrel"),
            Self::Velopack => Some("Velopack"),
            _ => None,
        }
    }
}

impl From<NestedInstallerType> for InstallerType {
    fn from(value: NestedInstallerType) -> Self {
        match value {
            NestedInstallerType::Msix => Self::Msix,
            NestedInstallerType::Msi => Self::Msi,
            NestedInstallerType::Appx => Self::Appx,
            NestedInstallerType::Exe => Self::Exe,
            NestedInstallerType::Inno => Self::Inno,
            NestedInstallerType::Nullsoft => Self::Nullsoft,
            NestedInstallerType::Wix => Self::Wix,
            NestedInstallerType::Burn => Self::Burn,
            NestedInstallerType::AdvancedInstaller => Self::AdvancedInstaller,
            NestedInstallerType::Squirrel => Self::Squirrel,
            NestedInstallerType::Velopack => Self::Velopack,
            NestedInstallerType::Portable => Self::Portable,
            NestedInstallerType::Font => Self::Font,
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for NestedInstallerType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[cfg(feature = "serde-saphyr")]
        if let Some(comment) = self.installer_technology() {
            return serde::Serialize::serialize(
                &serde_saphyr::Commented(self.as_str(), comment.to_owned()),
                serializer,
            );
        }

        serializer.serialize_str(self.as_str())
    }
}

impl fmt::Display for NestedInstallerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::NestedInstallerType;

    #[test]
    fn specialized_exe_types_have_schema_value() {
        assert_eq!(NestedInstallerType::AdvancedInstaller.as_str(), "exe");
        assert_eq!(NestedInstallerType::Squirrel.as_str(), "exe");
        assert_eq!(NestedInstallerType::Velopack.as_str(), "exe");
    }

    #[cfg(feature = "serde-saphyr")]
    #[test]
    fn specialized_exe_types_serialize_as_commented_exe() {
        assert_eq!(
            serde_saphyr::to_string(&NestedInstallerType::Velopack).unwrap(),
            "exe # Velopack\n"
        );
    }
}
