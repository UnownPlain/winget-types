use core::{fmt, str::FromStr};

use thiserror::Error;

use super::nested::installer_type::NestedInstallerType;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[non_exhaustive]
pub enum InstallerType {
    Msix,
    Msi,
    Appx,
    Exe,
    Inno,
    Nullsoft,
    Wix,
    Burn,
    Pwa,
    Zip,
    AdvancedInstaller,
    Squirrel,
    Velopack,
    Portable,
    Font,
}

impl InstallerType {
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
            Self::Pwa => "pwa",
            Self::Zip => "zip",
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

    #[must_use]
    pub const fn is_exe(self) -> bool {
        matches!(
            self,
            Self::Exe | Self::AdvancedInstaller | Self::Squirrel | Self::Velopack
        )
    }
}

impl AsRef<str> for InstallerType {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<InstallerType> for NestedInstallerType {
    type Error = ();

    fn try_from(value: InstallerType) -> Result<Self, Self::Error> {
        match value {
            InstallerType::Msix => Ok(Self::Msix),
            InstallerType::Msi => Ok(Self::Msi),
            InstallerType::Appx => Ok(Self::Appx),
            InstallerType::Exe => Ok(Self::Exe),
            InstallerType::Inno => Ok(Self::Inno),
            InstallerType::Nullsoft => Ok(Self::Nullsoft),
            InstallerType::Wix => Ok(Self::Wix),
            InstallerType::Burn => Ok(Self::Burn),
            InstallerType::AdvancedInstaller => Ok(Self::AdvancedInstaller),
            InstallerType::Squirrel => Ok(Self::Squirrel),
            InstallerType::Velopack => Ok(Self::Velopack),
            InstallerType::Portable => Ok(Self::Portable),
            InstallerType::Font => Ok(Self::Font),
            InstallerType::Zip | InstallerType::Pwa => Err(()),
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for InstallerType {
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

impl fmt::Display for InstallerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

#[derive(Error, Debug, Eq, PartialEq)]
#[error("Installer type did not match a valid lowercase installer type")]
pub struct InstallerTypeParseError;

impl FromStr for InstallerType {
    type Err = InstallerTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "msix" => Ok(Self::Msix),
            "msi" => Ok(Self::Msi),
            "appx" => Ok(Self::Appx),
            "exe" => Ok(Self::Exe),
            "zip" => Ok(Self::Zip),
            "inno" => Ok(Self::Inno),
            "nullsoft" => Ok(Self::Nullsoft),
            "wix" => Ok(Self::Wix),
            "burn" => Ok(Self::Burn),
            "pwa" => Ok(Self::Pwa),
            "advancedinstaller" | "advanced-installer" => Ok(Self::AdvancedInstaller),
            "squirrel" => Ok(Self::Squirrel),
            "velopack" => Ok(Self::Velopack),
            "portable" => Ok(Self::Portable),
            "font" => Ok(Self::Font),
            _ => Err(InstallerTypeParseError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InstallerType;

    #[test]
    fn specialized_exe_types_have_schema_value() {
        assert_eq!(InstallerType::AdvancedInstaller.as_str(), "exe");
        assert_eq!(InstallerType::Squirrel.as_str(), "exe");
        assert_eq!(InstallerType::Velopack.as_str(), "exe");
    }

    #[cfg(feature = "serde-saphyr")]
    #[test]
    fn specialized_exe_types_serialize_as_commented_exe() {
        assert_eq!(
            serde_saphyr::to_string(&InstallerType::Velopack).unwrap(),
            "exe # Velopack\n"
        );
    }
}
