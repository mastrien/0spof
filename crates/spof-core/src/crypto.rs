use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

/// Computes the hexadecimal SHA-256 checksum of a byte slice.
pub fn hash_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Computes the hexadecimal SHA-256 checksum of a file on disk.
pub fn hash_file<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Verifies whether a byte slice matches an expected SHA-256 checksum.
pub fn verify_bytes_hash(data: &[u8], expected_hash: &str) -> bool {
    let actual = hash_bytes(data);
    actual.eq_ignore_ascii_case(expected_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_hash_bytes_empty() {
        // Known SHA-256 for empty string
        let empty_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(hash_bytes(b""), empty_hash);
    }

    #[test]
    fn test_hash_bytes_known_value() {
        // Known SHA-256 for "hello world"
        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        assert_eq!(hash_bytes(b"hello world"), expected);
        assert!(verify_bytes_hash(b"hello world", expected));
        assert!(!verify_bytes_hash(b"hello world", "invalid_hash"));
    }

    #[test]
    fn test_hash_file() {
        let mut temp = NamedTempFile::new().expect("failed to create temp file");
        temp.write_all(b"hello world").expect("failed to write");
        temp.flush().expect("failed to flush");

        let file_hash = hash_file(temp.path()).expect("failed to hash file");
        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        assert_eq!(file_hash, expected);
    }

    #[test]
    fn test_hash_file_non_existent() {
        let res = hash_file(Path::new("non_existent_file_xyz_123.tmp"));
        assert!(res.is_err());
    }
}
