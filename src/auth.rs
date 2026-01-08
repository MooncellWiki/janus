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
use tracing::warn;

use crate::state::AppState;

/// JWT Claims structure using standard registered claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user identifier)
    pub sub: String,
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
            sub: subject,
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
    // Extract Authorization header
    let auth_header = match request.headers().get("Authorization") {
        Some(header) => match header.to_str() {
            Ok(h) => h,
            Err(_) => {
                warn!("Invalid authorization header format");
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "code": 1
                    })),
                )
                    .into_response();
            }
        },
        None => {
            warn!("Missing authorization header");
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "code": 1
                })),
            )
                .into_response();
        }
    };

    // Extract token from Bearer scheme
    let token = match extract_token_from_header(auth_header) {
        Some(t) => t,
        None => {
            warn!("Invalid authorization format, expected: Bearer <token>");
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "code": 1
                })),
            )
                .into_response();
        }
    };

    // Verify token
    match verify_token(token, &state.jwt_config.public_key) {
        Ok(_claims) => {
            // Token is valid, proceed with request
            next.run(request).await
        }
        Err(err) => {
            warn!("JWT verification failed: {:?}", err);
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "code": 1
                })),
            )
                .into_response()
        }
    }
}
