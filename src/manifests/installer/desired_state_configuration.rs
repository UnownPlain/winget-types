use alloc::{collections::BTreeSet, string::String, vec::Vec};

use crate::url::DecodedUrl;

/// References to desired state configuration (DSC) resources related to a package.
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DesiredStateConfiguration {
    /// DSC resources contained in PowerShell modules.
    #[cfg_attr(
        feature = "serde",
        serde(rename = "PowerShell", skip_serializing_if = "Option::is_none")
    )]
    pub powershell: Option<BTreeSet<PowerShellModule>>,

    /// DSC resources contained in the package using the DSC v3 specification.
    #[cfg_attr(
        feature = "serde",
        serde(rename = "DSCv3", skip_serializing_if = "Option::is_none")
    )]
    pub dsc_v3: Option<DscV3>,
}

/// A PowerShell module containing DSC resources.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
pub struct PowerShellModule {
    pub repository_url: DecodedUrl,
    pub module_name: String,
    pub resources: Vec<PowerShellResource>,
}

/// A DSC resource in a PowerShell module.
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
pub struct PowerShellResource {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub name: Option<String>,
}

/// DSC v3 resources contained in a package.
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
pub struct DscV3 {
    pub resources: Vec<DscV3Resource>,
}

/// A DSC v3 resource, named as `Publisher.Product.Component/ResourceName`.
///
/// The product and component portions of the name are optional.
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
pub struct DscV3Resource {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub r#type: Option<String>,
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use alloc::vec;

    use indoc::indoc;
    use rstest::rstest;

    use super::DesiredStateConfiguration;
    use crate::{
        Manifest, ManifestVersion, VersionManifest,
        installer::{Installer, InstallerManifest},
        locale::{DefaultLocaleManifest, LocaleManifest},
    };

    const DSC: &str = indoc! {"
        PowerShell:
        - RepositoryUrl: https://www.powershellgallery.com/api/v2
          ModuleName: Example.Module
          Resources:
          - Name: ExampleResource
        DSCv3:
          Resources:
          - Type: Example.Package/Resource
    "};

    #[test]
    fn round_trip_dsc_metadata() {
        let dsc: DesiredStateConfiguration = serde_yaml::from_str(DSC).unwrap();
        assert_eq!(serde_yaml::to_string(&dsc).unwrap(), DSC);

        let manifest = InstallerManifest {
            package_identifier: "Example.Package".parse().unwrap(),
            package_version: "1.0.0".parse().unwrap(),
            desired_state_configuration: Some(dsc.clone()),
            installers: vec![Installer {
                url: "https://example.com/installer.exe".parse().unwrap(),
                desired_state_configuration: Some(dsc),
                ..Installer::default()
            }],
            ..InstallerManifest::default()
        };
        let yaml = serde_yaml::to_string(&manifest).unwrap();
        assert_eq!(
            serde_yaml::from_str::<InstallerManifest>(&yaml).unwrap(),
            manifest
        );
        assert!(yaml.contains("ManifestVersion: 1.28.0"));
    }

    #[rstest]
    #[case("{}")]
    #[case("PowerShell: null\nDSCv3: null")]
    #[case("PowerShell: []\nDSCv3:\n  Resources: []")]
    #[case("DSCv3:\n  Resources:\n  - {}")]
    fn accepts_optional_and_empty_resources(#[case] yaml: &str) {
        let dsc: DesiredStateConfiguration = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            serde_yaml::from_str::<DesiredStateConfiguration>(
                &serde_yaml::to_string(&dsc).unwrap()
            )
            .unwrap(),
            dsc
        );
    }

    #[rstest]
    #[case("PowerShell:\n- ModuleName: Example.Module\n  Resources: []")]
    #[case("PowerShell:\n- RepositoryUrl: https://example.com\n  Resources: []")]
    #[case("PowerShell:\n- RepositoryUrl: https://example.com\n  ModuleName: Example.Module")]
    #[case("DSCv3: {}")]
    fn requires_module_fields_and_resource_lists(#[case] yaml: &str) {
        assert!(serde_yaml::from_str::<DesiredStateConfiguration>(yaml).is_err());
    }

    #[test]
    fn inherit_merge_and_optimize_preserve_dsc() {
        let dsc: DesiredStateConfiguration = serde_yaml::from_str(DSC).unwrap();
        let mut manifest = InstallerManifest {
            desired_state_configuration: Some(dsc.clone()),
            ..InstallerManifest::default()
        };
        let inherited = Installer::default().inherit_from(&manifest);
        assert_eq!(inherited.desired_state_configuration, Some(dsc.clone()));
        assert_eq!(
            Installer::default().merge_with(inherited.clone()),
            inherited
        );

        let override_installer = Installer {
            desired_state_configuration: Some(DesiredStateConfiguration::default()),
            ..Installer::default()
        };
        assert_eq!(
            override_installer.clone().inherit_from(&manifest),
            override_installer
        );
        assert_eq!(
            override_installer.clone().merge_with(inherited.clone()),
            override_installer
        );

        manifest.installers = vec![inherited];
        manifest.optimize();
        assert_eq!(manifest.desired_state_configuration, Some(dsc));
        assert!(manifest.installers[0].desired_state_configuration.is_none());

        let mut different = InstallerManifest {
            installers: vec![override_installer, Installer::default()],
            ..InstallerManifest::default()
        };
        different.optimize();
        assert!(different.desired_state_configuration.is_none());
        assert_eq!(different.installers.len(), 2);
    }

    #[test]
    fn manifest_versions_and_schemas_match() {
        fn check<M: Manifest + Default>() {
            assert_eq!(
                M::default().manifest_version(),
                ManifestVersion::new(1, 28, 0)
            );
            assert!(M::SCHEMA.ends_with(".1.28.0.schema.json"));
        }
        check::<InstallerManifest>();
        check::<DefaultLocaleManifest>();
        check::<LocaleManifest>();
        check::<VersionManifest>();
    }
}
