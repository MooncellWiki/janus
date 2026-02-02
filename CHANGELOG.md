# Changelog

All notable changes to this project will be documented in this file.

## [0.2.1] - 2026-02-02

[cd34405](cd3440570f49c1aba4b9ee551a77e9984e0f5f64)...[9769502](97695021fc36fc32aa01e682e23b7f41aa2d556c)

### 🐛 Bug Fixes

- Remove unnecessary percent encoding of object_key in OSS event handler (#16) ([9769502](https://github.com/MooncellWiki/ak-asset-storage/commit/97695021fc36fc32aa01e682e23b7f41aa2d556c)), Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

---------, Co-authored-by:copilot-swe-agent[bot] <198982749+Copilot@users.noreply.github.com>, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

### 📚 Documentation

- Update ([e36ae6e](https://github.com/MooncellWiki/ak-asset-storage/commit/e36ae6ea778bbb368a41d8135df876fc00251907))

## [0.2.0] - 2026-01-19

[02bdb32](02bdb32ad081973cbb8ed357750b4f22d068511b)...[cd34405](cd3440570f49c1aba4b9ee551a77e9984e0f5f64)

### ⚙️ Miscellaneous Tasks

- Configure cargo release ([af0725f](https://github.com/MooncellWiki/ak-asset-storage/commit/af0725f8fc434094e6122e706aeb30e0086b0f48))
- Release 0.2.0 ([cd34405](https://github.com/MooncellWiki/ak-asset-storage/commit/cd3440570f49c1aba4b9ee551a77e9984e0f5f64))

## [0.1.0] - 2026-01-19

### 🚀 Features

- Implement Bilibili dynamic posting API endpoint ([8967960](https://github.com/MooncellWiki/ak-asset-storage/commit/89679605024542d618210b5e33fe60c7d0cda624)), Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* Implement Bilibili dynamic posting API endpoint, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* Improve code quality: use shared HTTP client and better error handling, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* Add comprehensive documentation for Bilibili API endpoint, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* Implement JWT authentication with ES256 and update API routes

- Change route from /api/v1 to /api
- Replace API key auth with JWT authentication using ES256
- Add JWT configuration with public/private keys
- Create reusable JWT middleware
- Add CLI command to generate JWT tokens
- Remove api_key from Bilibili config
- Update example.toml with JWT configuration, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* Update documentation for JWT authentication changes

- Update API endpoint path from /api/v1 to /api
- Replace API key examples with JWT Bearer token auth
- Add JWT token generation instructions
- Update error messages for JWT auth
- Update code examples for JavaScript and Python
- Fix OpenAPI documentation URLs, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* Simplify JWT config and authentication error responses

- Remove Option wrapper from jwt field in AppSettings (now required)
- Remove Option wrapper from jwt_config in AppState
- Simplify auth error responses to only return {code: 1}
- Log detailed error messages instead of including in response
- Update example.toml to reflect JWT as required config, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* Refactor router to use single OpenApiRouter instead of merge

- Combine health and bilibili routes into one OpenApiRouter
- Use route_layer to apply JWT middleware only to routes added after it
- Health endpoints remain public (no auth)
- Bilibili endpoints protected by JWT (added after route_layer)
- Eliminates need for separate router creation and merge, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* Simplify JWT claims to use only sub and iat fields

- Remove exp (expiration) field from Claims struct
- Keep only sub (subject) and iat (issued at) fields
- Update generate_token to not require expires_in parameter
- Disable expiration validation in verify_token
- Update CLI generate-jwt command to remove expires-in argument
- Tokens now never expire (no expiration time), Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* fix

* update

* Update docs/BILIBILI_API.md, Co-authored-by:Copilot <175728472+Copilot@users.noreply.github.com>

* update

* Optimize bilibili_handlers.rs: reduce duplication and improve OpenAPI docs

- Extract duplicate error handling into helper function handle_create_dynamic_response
- Add UNAUTHORIZED (401) response to OpenAPI documentation
- Add bearer_auth security scheme to OpenAPI spec
- Apply security requirement to create_dynamic endpoint
- Reduce code from ~495 lines to ~385 lines (~22% reduction), Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* Add generic error handling with thiserror and IntoResponse

- Create error.rs module with AppError enum using thiserror
- Implement IntoResponse for AppError for automatic HTTP responses
- Add AppResult<T> type alias for handler return types
- Refactor bilibili_handlers.rs to use AppResult
- Refactor misc_handlers.rs to use AppResult
- Simplify error handling with ? operator throughout handlers
- Maintain consistent error response format with proper status codes
- All errors now logged automatically via IntoResponse implementation, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* Simplify error handling: use anyhow::Error internally

- Reduce AppError variants from 10 to just 2: BadRequest and InternalError
- Store anyhow::Error internally instead of String for better error context
- BadRequest for parameter/validation errors (400 status)
- InternalError for all business logic errors (500 status)
- Implement From traits for common error types (sqlx, serde_json, reqwest, anyhow)
- Update bilibili_handlers.rs to use anyhow::Context for error chain
- All errors still return {code: 1} response format
- Full error context logged server-side for debugging
- Cleaner, more maintainable error handling code, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* Add back Unauthorized error variant

- Re-add Unauthorized variant to AppError enum
- Maps to HTTP 401 UNAUTHORIZED status code
- Stores anyhow::Error for rich error context
- Can be used for authentication/authorization errors
- Now have 3 variants: BadRequest (400), Unauthorized (401), InternalError (500), Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* Refactor auth.rs to use AppError::Unauthorized

- Replace manual error responses with AppError::Unauthorized
- Remove unused imports (Json, StatusCode, serde_json, warn)
- Use anyhow::anyhow! to create error messages with context
- Return Result<Response, AppError> from jwt_auth_middleware
- Simplify error handling with ? operator
- All authentication errors now go through AppError's IntoResponse
- Consistent error logging and response format via AppError, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* Use AppResult type alias in jwt_auth_middleware

- Change return type from Result<Response, AppError> to AppResult<Response>
- Import AppResult from error module
- Consistent with other handler return types across the application, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* Update docs/BILIBILI_API.md, Co-authored-by:Copilot <175728472+Copilot@users.noreply.github.com>

* Update docs/BILIBILI_API.md, Co-authored-by:Copilot <175728472+Copilot@users.noreply.github.com>

* Update docs/BILIBILI_API.md, Co-authored-by:Copilot <175728472+Copilot@users.noreply.github.com>

* Update src/routes/bilibili_handlers.rs, Co-authored-by:Copilot <175728472+Copilot@users.noreply.github.com>

* Update example.toml, Co-authored-by:Copilot <175728472+Copilot@users.noreply.github.com>

* Extract duplicate dynamic creation logic into helper function

- Add create_dynamic_with_scene helper function
- Reduces code duplication between scene 1 (text-only) and scene 2 (with images)
- Maintains same functionality while improving maintainability
- Reduces bilibili_handlers.rs by ~60 lines
- Addresses code review feedback about DRY principle, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* update

* update

* update

---------, Co-authored-by:copilot-swe-agent[bot] <198982749+Copilot@users.noreply.github.com>, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>, Co-authored-by:夕舞八弦 <1677759063@qq.com>, Co-authored-by:Copilot <175728472+Copilot@users.noreply.github.com>
- Add EventBridge webhook endpoint for OSS events with configurable CDN refresh (#14) ([00b67f2](https://github.com/MooncellWiki/ak-asset-storage/commit/00b67f21269f95fe5919ec76c2728ef4dbf96d59))

### ⚙️ Miscellaneous Tasks

- Release ([f1e513b](https://github.com/MooncellWiki/ak-asset-storage/commit/f1e513bc2f63161ae9b4b6e2dba0b12e89c4250d))
- Update gitignore ([02bdb32](https://github.com/MooncellWiki/ak-asset-storage/commit/02bdb32ad081973cbb8ed357750b4f22d068511b))

<!-- generated by git-cliff -->
