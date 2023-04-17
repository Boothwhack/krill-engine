use std::fmt::Debug;
use std::str::FromStr;
use once_cell::sync::Lazy;
use regex::Regex;
use thiserror::Error;

#[derive(Clone, Debug, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct AssetPath(String);

impl AssetPath {
    pub const ALLOWED_CHARS: &'static str = r"^[/a-zA-Z\.\-_0-9]+$";
    pub const SEPARATOR: &'static str = "/";

    pub fn new(path: &str) -> Result<Self, InvalidCharacters> {
        Self::from_str(path)
    }

    pub fn is_absolute(&self) -> bool {
        self.0.starts_with(AssetPath::SEPARATOR)
    }

    pub fn is_relative(&self) -> bool {
        !self.is_absolute()
    }

    pub fn resolve(&self, path: AssetPath) -> Option<AssetPath> {
        if path.is_absolute() {
            return Some(path);
        } else if self.is_relative() {
            return None;
        }

        let path = match self.0.rfind(AssetPath::SEPARATOR)
            .expect("self is absolute so always starts with '/'") {
            0 => format!("/{}", path.0),
            index => {
                let (base, _) = self.0.split_at(index);
                format!("{}/{}", base, path.0)
            }
        };
        Some(AssetPath(path))
    }

    pub fn path_string(&self) -> &str {
        &self.0
    }

    pub fn append(self, segment: &str) -> AssetPath {
        AssetPath(self.0 + segment)
    }
}

#[derive(Error, Debug, Eq, PartialEq)]
#[error("asset path {} contains invalid characters.", .0)]
pub struct InvalidCharacters(String);

impl FromStr for AssetPath {
    type Err = InvalidCharacters;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        static ALLOWED_CHARS: Lazy<Regex> = Lazy::new(|| {
            Regex::new(AssetPath::ALLOWED_CHARS).expect("should be valid regex")
        });

        if ALLOWED_CHARS.is_match(s) {
            Ok(AssetPath(s.to_owned()))
        } else {
            Err(InvalidCharacters(s.to_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::path::{AssetPath, InvalidCharacters};

    #[test]
    fn from_string() {
        assert!("/test".parse::<AssetPath>().expect("should successfully parse").is_absolute());
        assert!("test".parse::<AssetPath>().expect("should successfully parse").is_relative());
        assert_eq!(
            "test_file$"
                .parse::<AssetPath>()
                .expect_err("should fail to parse because of invalid character"),
            InvalidCharacters("test_file$".to_owned()),
        );
    }

    #[test]
    fn resolve() {
        // Represents an asset file, 'manifest.json', which refers to a dependency 'file.txt'.
        let base = AssetPath::new("/test/manifest.json")
            .expect("absolute path");
        let relative = AssetPath::new("file.txt")
            .expect("relative path");
        let resolved = AssetPath::new("/test/file.txt")
            .expect("resolved path");
        assert_eq!(base.resolve(relative), Some(resolved))
    }

    #[test]
    fn resolve_relative_to_relative() {
        let base = AssetPath::new("relative/file.txt").unwrap();
        let relative = AssetPath::new("another/rel/file.jpeg").unwrap();
        assert_eq!(base.resolve(relative), None);
    }
}
