//! PKCE (Proof Key for Code Exchange) support.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::Rng;
use sha2::{Digest, Sha256};

/// PKCE challenge method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PkceChallengeMethod {
    /// S256 (SHA-256) method.
    S256,
}

impl PkceChallengeMethod {
    /// Returns the string representation for OAuth parameters.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::S256 => "S256",
        }
    }
}

/// PKCE challenge and verifier pair.
#[derive(Debug, Clone)]
pub struct PkceChallenge {
    /// The code verifier (secret, sent when exchanging the code).
    pub verifier: String,
    /// The code challenge (derived from verifier, sent in auth URL).
    pub challenge: String,
    /// The challenge method.
    pub method: PkceChallengeMethod,
}

impl PkceChallenge {
    /// Generates a new PKCE challenge.
    pub fn new() -> Self {
        let verifier = generate_code_verifier();
        let challenge = generate_code_challenge(&verifier);

        Self {
            verifier,
            challenge,
            method: PkceChallengeMethod::S256,
        }
    }

    /// Creates a PKCE challenge from an existing verifier.
    pub fn from_verifier(verifier: impl Into<String>) -> Self {
        let verifier = verifier.into();
        let challenge = generate_code_challenge(&verifier);

        Self {
            verifier,
            challenge,
            method: PkceChallengeMethod::S256,
        }
    }

    /// Verifies that a code verifier matches a challenge.
    pub fn verify(verifier: &str, challenge: &str) -> bool {
        let computed_challenge = generate_code_challenge(verifier);
        computed_challenge == challenge
    }
}

impl Default for PkceChallenge {
    fn default() -> Self {
        Self::new()
    }
}

/// Generates a cryptographically random code verifier.
fn generate_code_verifier() -> String {
    let mut rng = rand::rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.random::<u8>()).collect();
    URL_SAFE_NO_PAD.encode(&bytes)
}

/// Generates a code challenge from a verifier using S256 method.
fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_generation() {
        let pkce = PkceChallenge::new();

        assert!(!pkce.verifier.is_empty());
        assert!(!pkce.challenge.is_empty());
        assert_ne!(pkce.verifier, pkce.challenge);
        assert_eq!(pkce.method, PkceChallengeMethod::S256);
    }

    #[test]
    fn test_pkce_verification() {
        let pkce = PkceChallenge::new();

        assert!(PkceChallenge::verify(&pkce.verifier, &pkce.challenge));
        assert!(!PkceChallenge::verify("wrong-verifier", &pkce.challenge));
    }

    #[test]
    fn test_pkce_from_verifier() {
        let original = PkceChallenge::new();
        let reconstructed = PkceChallenge::from_verifier(&original.verifier);

        assert_eq!(original.verifier, reconstructed.verifier);
        assert_eq!(original.challenge, reconstructed.challenge);
    }

    #[test]
    fn test_pkce_uniqueness() {
        let pkce1 = PkceChallenge::new();
        let pkce2 = PkceChallenge::new();

        assert_ne!(pkce1.verifier, pkce2.verifier);
        assert_ne!(pkce1.challenge, pkce2.challenge);
    }
}
