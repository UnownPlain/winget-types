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
