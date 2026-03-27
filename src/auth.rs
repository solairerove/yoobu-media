// Rust key concept: FromRequestParts is a trait that makes a type "extractable"
// from an HTTP request. Axum sees `_auth: AuthenticatedRequest` in a handler
// signature and knows how to obtain it by calling this trait.
//
// This is a zero-cost abstraction: no reflection, no runtime overhead.
// Everything is resolved at compile time.

use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use subtle::ConstantTimeEq;

use crate::{error::AppError, AppState};

/// Marker type — if a handler received it, the request is authenticated.
/// Carries no data itself (unit struct), only proves the fact.
pub struct AuthenticatedRequest;

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedRequest {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;

        if verify_token(token, &state.config.internal_api_key) {
            Ok(AuthenticatedRequest)
        } else {
            Err(AppError::Unauthorized)
        }
    }
}

// Constant-time comparison: a plain == short-circuits on the first mismatched byte,
// allowing a timing attack (measure how many bytes matched).
// subtle::ConstantTimeEq always walks the full length of the string.
fn verify_token(provided: &str, expected: &str) -> bool {
    let p = provided.as_bytes();
    let e = expected.as_bytes();

    // Different lengths — definitely false, but perform a dummy comparison
    // so the attacker cannot infer the key length from response timing.
    if p.len() != e.len() {
        let _ = p.ct_eq(p);
        return false;
    }

    p.ct_eq(e).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_token_passes() {
        assert!(verify_token("secret-key-abc", "secret-key-abc"));
    }

    #[test]
    fn wrong_token_fails() {
        assert!(!verify_token("wrong-key-xxx", "secret-key-abc"));
    }

    #[test]
    fn empty_token_fails() {
        assert!(!verify_token("", "secret-key-abc"));
    }

    #[test]
    fn different_length_fails() {
        assert!(!verify_token("short", "secret-key-abc"));
    }

    #[test]
    fn prefix_match_fails() {
        assert!(!verify_token("secret-key-ab", "secret-key-abc"));
    }
}
