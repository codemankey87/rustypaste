use crate::util;
use actix_web::{error, Error as ActixError};
use glob::glob;
use std::convert::TryFrom;
use std::fs::File as OsFile;
use std::path::{Path, PathBuf};

/// [`PathBuf`] wrapper for storing checksums.
#[derive(Debug)]
pub struct File {
    /// Path of the file.
    pub path: PathBuf,
    /// SHA256 checksum.
    pub sha256sum: String,
}

/// Directory that contains [`File`]s.
pub struct Directory {
    /// Files in the directory.
    pub files: Vec<File>,
    /// Total size of the files in bytes.
    pub total_size: u64,
}

impl<'a> TryFrom<&'a Path> for Directory {
    type Error = ActixError;
    fn try_from(directory: &'a Path) -> Result<Self, Self::Error> {
        let mut total_size: u64 = 0;
        let files = glob(directory.join("**").join("*").to_str().ok_or_else(|| {
            error::ErrorInternalServerError("directory contains invalid characters")
        })?)
        .map_err(error::ErrorInternalServerError)?
        .filter_map(Result::ok)
        .filter(|path| !path.is_dir())
        .filter_map(|path| match OsFile::open(&path) {
            Ok(file) => {
                let size = file.metadata().ok()?.len();
                total_size += size;
                Some((path, file))
            }
            _ => None,
        })
        .filter_map(|(path, file)| match util::sha256_digest(file) {
            Ok(sha256sum) => Some(File { path, sha256sum }),
            _ => None,
        })
        .collect();
        Ok(Self { files, total_size })
    }
}

impl Directory {
    /// Returns the file that matches the given checksum.
    pub fn get_file<S: AsRef<str>>(&self, sha256sum: S) -> Option<&File> {
        self.files.iter().find(|file| {
            file.sha256sum == sha256sum.as_ref()
                && !util::TIMESTAMP_EXTENSION_REGEX.is_match(&file.path.to_string_lossy())
        })
    }

    /// Checks if the total size of the files exceeds the maximum allowed size.
    pub fn is_over_size_limit(&self, max_size: Byte) -> bool {
        self.total_size > max_size.get_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn test_file_checksum() -> Result<(), ActixError> {
        assert_eq!(
            Some(OsString::from("rustypaste_logo.png").as_ref()),
            Directory::try_from(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("img")
                    .as_path()
            )?
            .get_file("2073f6f567dcba3b468c568d29cf8ed2e9d3f0f7305b9ab1b5a22861f5922e61")
            .expect("cannot get file with checksum")
            .path
            .file_name()
        );
        Ok(())
    }
}
