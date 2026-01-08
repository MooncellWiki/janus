use axum::{
    Json,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::state::AppState;

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user identifier)
    pub sub: String,
    /// Expiration time (as Unix timestamp)
    pub exp: u64,
    /// Issued at (as Unix timestamp)
    pub iat: u64,
}

impl Claims {
    /// Create new claims with given subject and expiration duration in seconds
    pub fn new(subject: String, expires_in_secs: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        Self {
            sub: subject,
            iat: now,
            exp: now + expires_in_secs,
        }
    }
}

/// Generate a JWT token using ES256 algorithm
pub fn generate_token(
    subject: String,
    private_key_pem: &str,
    expires_in_secs: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = Claims::new(subject, expires_in_secs);
    let encoding_key = EncodingKey::from_ec_pem(private_key_pem.as_bytes())?;
    let header = Header::new(Algorithm::ES256);
    encode(&header, &claims, &encoding_key)
}

/// Verify a JWT token using ES256 algorithm
pub fn verify_token(
    token: &str,
    public_key_pem: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let decoding_key = DecodingKey::from_ec_pem(public_key_pem.as_bytes())?;
    let mut validation = Validation::new(Algorithm::ES256);
    validation.validate_exp = true;

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
) -> Response {
    // Get JWT config
    let jwt_config = match &state.jwt_config {
        Some(config) => config,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "code": 1,
                    "msg": "JWT not configured"
                })),
            )
                .into_response();
        }
    };

    // Extract Authorization header
    let auth_header = match request.headers().get("Authorization") {
        Some(header) => match header.to_str() {
            Ok(h) => h,
            Err(_) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "code": 1,
                        "msg": "Invalid authorization header"
                    })),
                )
                    .into_response();
            }
        },
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "code": 1,
                    "msg": "Missing authorization header"
                })),
            )
                .into_response();
        }
    };

    // Extract token from Bearer scheme
    let token = match extract_token_from_header(auth_header) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "code": 1,
                    "msg": "Invalid authorization format, expected: Bearer <token>"
                })),
            )
                .into_response();
        }
    };

    // Verify token
    match verify_token(token, &jwt_config.public_key) {
        Ok(_claims) => {
            // Token is valid, proceed with request
            next.run(request).await
        }
        Err(err) => {
            let msg = match err.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => "Token expired",
                jsonwebtoken::errors::ErrorKind::InvalidToken => "Invalid token",
                jsonwebtoken::errors::ErrorKind::InvalidSignature => "Invalid signature",
                _ => "Token verification failed",
            };

            (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "code": 1,
                    "msg": msg
                })),
            )
                .into_response()
        }
    }
}
