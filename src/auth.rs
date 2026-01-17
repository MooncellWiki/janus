use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, time::{SystemTime, UNIX_EPOCH}};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// JWT Claims structure using standard registered claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user identifier) - optional since EventBridge tokens may not include it
    pub sub: Option<String>,
    /// Issued at (as Unix timestamp)
    pub iat: u64,
}

impl Claims {
    /// Create new claims with given subject
    pub fn new(subject: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        Self {
            sub: Some(subject),
            iat: now,
        }
    }
}

/// Generate a JWT token using ES256 algorithm
pub fn generate_token(
    subject: String,
    private_key_pem: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = Claims::new(subject);
    let private_key_pem = private_key_pem.trim();
    let encoding_key = EncodingKey::from_ec_pem(private_key_pem.as_bytes())?;
    let header = Header::new(Algorithm::ES256);
    encode(&header, &claims, &encoding_key)
}

/// Verify a JWT token using ES256 algorithm
pub fn verify_token(
    token: &str,
    public_key_pem: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let public_key_pem = public_key_pem.trim();
    let decoding_key = DecodingKey::from_ec_pem(public_key_pem.as_bytes())?;
    let mut validation = Validation::new(Algorithm::ES256);
    validation.validate_exp = false; // No expiration validation
    validation.required_spec_claims = HashSet::new(); // don't validate “exp”, “nbf”, “aud”, “iss”, “sub”

    let token_data = decode::<Claims>(token, &decoding_key, &validation)?;
    Ok(token_data.claims)
}

/// Extract JWT token from Authorization header
fn extract_token_from_header(auth_header: &str) -> Option<&str> {
    auth_header.strip_prefix("Bearer ")
}

/// JWT authentication middleware
pub async fn jwt_auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> AppResult<Response> {
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .ok_or_else(|| AppError::Unauthorized(anyhow::anyhow!("Missing authorization header")))?
        .to_str()
        .map_err(|_| {
            AppError::Unauthorized(anyhow::anyhow!("Invalid authorization header format"))
        })?;

    // Extract token from Bearer scheme
    let token = extract_token_from_header(auth_header).ok_or_else(|| {
        AppError::Unauthorized(anyhow::anyhow!(
            "Invalid authorization format, expected: Bearer <token>"
        ))
    })?;

    // Verify token
    verify_token(token, &state.jwt_config.public_key).map_err(|err| {
        AppError::Unauthorized(anyhow::anyhow!("JWT verification failed: {}", err))
    })?;

    // Token is valid, proceed with request
    Ok(next.run(request).await)
}
