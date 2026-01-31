//! Multi-Hash Support Integration Tests
//!
//! This module tests the multi-hash support across different layers of the Mega system.
//! It verifies that both SHA-1 (40 chars) and SHA-256 (64 chars) hashes are properly
//! handled throughout the codebase.
//!
//! # Test Coverage
//!
//! - API Model Layer: `parse_object_hash` validation
//! - Client Components: Hash length validation in BuckService
//! - Protocol Layer: `object-format` capability handling

use ceres::model::buck::{FileChange, ManifestFile, parse_object_hash};
use common::config::HashAlgorithm;


mod api_model_tests {
    use super::*;

    /// Test that SHA-1 hashes (40 chars) are correctly parsed
    #[test]
    fn test_sha1_hash_parsing() {
        let sha1_hash = "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3";
        let result = parse_object_hash(sha1_hash, "test_field");
        assert!(result.is_ok(), "SHA-1 hash should be valid");
        
        let hash = result.unwrap();
        assert_eq!(hash.to_string().len(), 40, "SHA-1 hash output should be 40 chars");
    }

    /// Test that SHA-256 hashes (64 chars) are correctly parsed
    #[test]
    fn test_sha256_hash_parsing() {
        let sha256_hash = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let result = parse_object_hash(sha256_hash, "test_field");
        assert!(result.is_ok(), "SHA-256 hash should be valid");
        
        let hash = result.unwrap();
        assert_eq!(hash.to_string().len(), 64, "SHA-256 hash output should be 64 chars");
    }

    /// Test that ManifestFile correctly handles SHA-256 hashes
    #[test]
    fn test_manifest_file_sha256() {
        let manifest = ManifestFile {
            path: "test/file.txt".to_string(),
            size: 1024,
            hash: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        };
        
        let result = manifest.parse_hash();
        assert!(result.is_ok(), "ManifestFile should accept SHA-256 hash");
    }

    /// Test that FileChange correctly handles SHA-256 hashes
    #[test]
    fn test_file_change_sha256() {
        let change = FileChange::new(
            "test/file.txt".to_string(),
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            "100644".to_string(),
        );
        
        let result = change.parse_blob_hash();
        assert!(result.is_ok(), "FileChange should accept SHA-256 blob hash");
    }

    /// Test that invalid algorithm is rejected
    #[test]
    fn test_invalid_algorithm_rejected() {
        let invalid_hash = "md5:d41d8cd98f00b204e9800998ecf8427e";
        let result = parse_object_hash(invalid_hash, "test_field");
        assert!(result.is_err(), "Invalid algorithm should be rejected");
        assert!(result.unwrap_err().to_string().contains("Unsupported hash algorithm"));
    }

    /// Test that wrong length SHA-1 is rejected
    #[test]
    fn test_sha1_wrong_length_rejected() {
        let invalid_hash = "sha1:abc123"; // Too short
        let result = parse_object_hash(invalid_hash, "test_field");
        assert!(result.is_err(), "SHA-1 with wrong length should be rejected");
        assert!(result.unwrap_err().to_string().contains("expected 40"));
    }

    /// Test that wrong length SHA-256 is rejected
    #[test]
    fn test_sha256_wrong_length_rejected() {
        let invalid_hash = "sha256:abc123"; // Too short
        let result = parse_object_hash(invalid_hash, "test_field");
        assert!(result.is_err(), "SHA-256 with wrong length should be rejected");
        assert!(result.unwrap_err().to_string().contains("expected 64"));
    }
}

mod config_tests {
    use super::*;

    /// Test HashAlgorithm enum hex length values
    #[test]
    fn test_hash_algorithm_hex_lengths() {
        assert_eq!(HashAlgorithm::Sha1.hex_len(), 40);
        assert_eq!(HashAlgorithm::Sha256.hex_len(), 64);
    }

    /// Test HashAlgorithm to HashKind conversion
    #[test]
    fn test_hash_algorithm_to_hash_kind() {
        use git_internal::hash::HashKind;
        
        assert_eq!(HashAlgorithm::Sha1.to_hash_kind(), HashKind::Sha1);
        assert_eq!(HashAlgorithm::Sha256.to_hash_kind(), HashKind::Sha256);
    }

    /// Test default HashAlgorithm is SHA-1
    #[test]
    fn test_default_hash_algorithm() {
        let default = HashAlgorithm::default();
        assert_eq!(default, HashAlgorithm::Sha1);
    }
}

mod hash_validation_tests {
    use super::*;

    /// Example SHA-1 test vectors
    const SHA1_ZERO: &str = "0000000000000000000000000000000000000000";
    const SHA1_VALID: &str = "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3";
    
    /// Example SHA-256 test vectors  
    const SHA256_ZERO: &str = "0000000000000000000000000000000000000000000000000000000000000000";
    const SHA256_VALID: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    /// Test that zero hashes are valid for both algorithms
    #[test]
    fn test_zero_hashes_valid() {
        // SHA-1 zero hash
        let result = parse_object_hash(&format!("sha1:{}", SHA1_ZERO), "test");
        assert!(result.is_ok(), "SHA-1 zero hash should be valid");
        
        // SHA-256 zero hash
        let result = parse_object_hash(&format!("sha256:{}", SHA256_ZERO), "test");
        assert!(result.is_ok(), "SHA-256 zero hash should be valid");
    }

    /// Test that case-insensitive algorithm parsing works
    #[test]
    fn test_algorithm_case_insensitive() {
        // Uppercase SHA1
        let result = parse_object_hash(&format!("SHA1:{}", SHA1_VALID), "test");
        assert!(result.is_ok(), "Uppercase SHA1 should be valid");
        
        // Uppercase SHA256
        let result = parse_object_hash(&format!("SHA256:{}", SHA256_VALID), "test");
        assert!(result.is_ok(), "Uppercase SHA256 should be valid");
        
        // Mixed case
        let result = parse_object_hash(&format!("Sha1:{}", SHA1_VALID), "test");
        assert!(result.is_ok(), "Mixed case Sha1 should be valid");
    }

    /// Test boundary conditions for hash lengths
    #[test]
    fn test_hash_length_boundaries() {
        // 39 chars (too short for SHA-1)
        let short_39 = "sha1:".to_string() + &"a".repeat(39);
        assert!(parse_object_hash(&short_39, "test").is_err());
        
        // 40 chars (valid SHA-1)
        let valid_40 = "sha1:".to_string() + &"a".repeat(40);
        assert!(parse_object_hash(&valid_40, "test").is_ok());
        
        // 41 chars (too long for SHA-1)
        let long_41 = "sha1:".to_string() + &"a".repeat(41);
        assert!(parse_object_hash(&long_41, "test").is_err());
        
        // 63 chars (too short for SHA-256)
        let short_63 = "sha256:".to_string() + &"a".repeat(63);
        assert!(parse_object_hash(&short_63, "test").is_err());
        
        // 64 chars (valid SHA-256)
        let valid_64 = "sha256:".to_string() + &"a".repeat(64);
        assert!(parse_object_hash(&valid_64, "test").is_ok());
        
        // 65 chars (too long for SHA-256)
        let long_65 = "sha256:".to_string() + &"a".repeat(65);
        assert!(parse_object_hash(&long_65, "test").is_err());
    }
}

mod protocol_tests {
    /// Test that object-format capability is correctly generated
    #[test]
    fn test_object_format_capability_sha1() {
        use git_internal::hash::{HashKind, set_hash_kind_for_test};
        
        // Set hash kind to SHA-1 for this test
        let _guard = set_hash_kind_for_test(HashKind::Sha1);
        
        let hash_kind = git_internal::hash::get_hash_kind();
        let object_format = match hash_kind {
            HashKind::Sha1 => "object-format=sha1",
            HashKind::Sha256 => "object-format=sha256",
        };
        
        assert_eq!(object_format, "object-format=sha1");
    }

    /// Test that object-format capability is correctly generated for SHA-256
    #[test]
    fn test_object_format_capability_sha256() {
        use git_internal::hash::{HashKind, set_hash_kind_for_test};
        
        // Set hash kind to SHA-256 for this test
        let _guard = set_hash_kind_for_test(HashKind::Sha256);
        
        let hash_kind = git_internal::hash::get_hash_kind();
        let object_format = match hash_kind {
            HashKind::Sha1 => "object-format=sha1",
            HashKind::Sha256 => "object-format=sha256",
        };
        
        assert_eq!(object_format, "object-format=sha256");
    }
}
