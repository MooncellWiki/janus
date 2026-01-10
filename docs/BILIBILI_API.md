# Bilibili Dynamic Posting API

This document describes the Bilibili dynamic posting functionality implemented in this project.

## Overview

The `POST /api/bilibili/createDynamic` endpoint posts text and optional images to Bilibili as a dynamic.

- **Authentication:** Required. The route is protected by JWT middleware and expects `Authorization: Bearer <token>`.
- **Implementation:** Axum multipart parsing + `reqwest` calls to Bilibili’s web APIs.

## Configuration

Add the following sections to your `config.toml`:

```toml
[bilibili]
sessdata = "your_bilibili_sessdata_cookie"
bili_jct = "your_bilibili_bili_jct" # usually from `bili_jct`

[jwt]
private_key = """-----BEGIN EC PRIVATE KEY-----
YOUR_PRIVATE_KEY_HERE
-----END EC PRIVATE KEY-----"""
public_key = """-----BEGIN PUBLIC KEY-----
YOUR_PUBLIC_KEY_HERE
-----END PUBLIC KEY-----"""
```

### Configuration Fields

**Bilibili Config:**
- **sessdata** (required): Your Bilibili `SESSDATA` cookie value.
- **bili_jct** (required): Your Bilibili `bili_jct` cookie value. Used both as a request parameter and as a form field for image upload.

**JWT Config:**
- **private_key** (required): ES256 private key in PEM format for signing JWT tokens.
- **public_key** (required): ES256 public key in PEM format for verifying JWT tokens.

### Generating JWT Keys

Generate ES256 key pair using OpenSSL:

```bash
openssl ecparam -genkey -name prime256v1 -noout -out private.pem
openssl ec -in private.pem -pubout -out public.pem
```

### Obtaining Bilibili Credentials

1. **SESSDATA**: Log into Bilibili in your browser and extract the `SESSDATA` cookie value.
2. **bili_jct**: Log into Bilibili in your browser and extract the `bili_jct` cookie value.

## Generating JWT Tokens

Generate a JWT token using the CLI command:

```bash
cargo run -- generate-jwt --config config.toml --subject user_id
```

Options:
- `--config`: Path to config file (default: `config.toml`)
- `--subject`: Subject identifier (e.g., user ID or username)

Notes:
- Tokens are ES256 signed.
- This implementation does not validate `exp` (no expiration claim is required/checked).

## API Endpoint

### POST `/api/bilibili/createDynamic`

Creates a new Bilibili dynamic post with optional images.

**Authentication:** Required via `Authorization: Bearer <jwt_token>` header.

#### Request

**Headers:**
- **Authorization** (required): `Bearer <jwt_token>`

**Content-Type:** `multipart/form-data`

**Form Fields:**
- **msg** (required, string): JSON value that will be sent to Bilibili as `dyn_req.content.contents`.
- **file(s)** (optional): Any multipart field *with a filename* is treated as an uploaded image. The server does not require a specific field name like `files`, `image`, etc.

#### Request Examples

**Text-only dynamic:**

```bash
curl -X POST http://localhost:25150/api/bilibili/createDynamic \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -F 'msg=[{"type":1,"raw_text":"Hello from Rust API!","biz_id":""}]'
```

**Dynamic with a single image:**

```bash
curl -X POST http://localhost:25150/api/bilibili/createDynamic \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -F 'msg=[{"type":1,"raw_text":"Check out this image!","biz_id":""}]' \
  -F "image=@photo.jpg"
```

**Dynamic with multiple images:**

```bash
curl -X POST http://localhost:25150/api/bilibili/createDynamic \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -F 'msg=[{"type":1,"raw_text":"My photo gallery","biz_id":""}]' \
  -F "image1=@photo1.jpg" \
  -F "image2=@photo2.png" \
  -F "image3=@photo3.jpg"
```

#### Response Format

**Success Response (HTTP 200):**

```json
{
  "code": 0,
  "data": {
    "doc_id": 123,
    "dynamic_id": 456,
    "create_result": 0,
    "errmsg": null
  }
}
```

Notes:
- `data` may be omitted (`null`) even when Bilibili returns `code = 0`.

**Error Response:**

- Auth failures return **HTTP 401** with body `{ "code": 1 }`.
- Validation / Bilibili failures return JSON with body `{ "code": 1 }`. No additional error message fields are included in the response; detailed error information is only logged internally.

#### Error Codes (Conceptual)

| HTTP | code | Description |
|------|------|-------------|
| 401 | 1 | Missing/invalid Authorization header or JWT verification failed |
| 400 | 1 | Request validation failure (for example, missing or empty `msg` field) |
| 400 | 1 | Request validation failure (for example, invalid `msg` JSON format) |
| 500 | 1 | One or more images failed to upload to Bilibili |
| 200 | 1 | Bilibili returned non-zero `code` for dynamic creation (error details are only logged) |
| 500 | 1 | Failed to parse Bilibili create response |
| 500 | 1 | Network error or failed to read response (image-flow) |

Note:
- These descriptions explain when errors occur, but the actual HTTP response body is always `{ "code": 1 }` for failures.

## Message Format

The `msg` field must contain a valid JSON array representing the dynamic content. The structure follows Bilibili's dynamic content format:

### Basic Text Content

```json
[
  {
    "type": 1,
    "raw_text": "Your text content here",
    "biz_id": ""
  }
]
```

### Rich Text Example

```json
[
  {
    "type": 1,
    "raw_text": "Hello ",
    "biz_id": ""
  },
  {
    "type": 2,
    "raw_text": "@username",
    "biz_id": "user_id_here"
  },
  {
    "type": 1,
    "raw_text": " check this out!",
    "biz_id": ""
  }
]
```

**Content Types:**
- `type: 1` - Plain text
- `type: 2` - @mention (requires biz_id)
- Other types may be supported by Bilibili's API

## Implementation Details

### Image Upload Flow

1. Client sends multipart request with text (`msg`) and optional image files
2. Server validates JWT authentication (Authorization: Bearer `<token>`)
3. Server parses and validates the msg content
4. If images are present:
   - Each image is uploaded individually to Bilibili's BFS (Bilibili File System) via `/x/dynamic/feed/draw/upload_bfs`
   - Bilibili returns image URL, width, height for each uploaded image
5. Server creates the dynamic post via `/x/dynamic/feed/create/dyn`:
   - **Text-only**: Uses `scene: 1`
   - **With images**: Uses `scene: 2` and includes image metadata
6. Server returns success or error response

### Upload ID Generation

Each dynamic creation request includes a unique `upload_id` in the format:

```
{unix_timestamp_seconds}_{random_nonce}
```

Where:
- `unix_timestamp_seconds`: Current time in seconds since Unix epoch (floating-point, as produced by `as_secs_f64()`)
- `random_nonce`: Random 4-digit number (1000-9999)

### Authentication Flow

The endpoint uses multiple layers of authentication:

1. **JWT Authentication**: Protects the endpoint with ES256 signed tokens
   - Tokens must be included in the `Authorization: Bearer <token>` header
   - Tokens are long-lived and, in the current implementation, do **not** automatically expire; you must rotate/revoke tokens explicitly if they are leaked or no longer needed
   - Tokens are verified using the public key from configuration
2. **Bilibili SESSDATA**: Authenticates requests to Bilibili's API as your user
3. **CSRF Token**: Prevents cross-site request forgery attacks on Bilibili's API

### HTTP Client

The implementation uses a shared `reqwest::Client` instance stored in `AppState` for efficient connection pooling and reuse across requests.

## OpenAPI Documentation

The API is documented with OpenAPI. Access the interactive documentation at:

- **Scalar UI**: `http://localhost:25150/api/scalar`
- **OpenAPI JSON**: `http://localhost:25150/api/openapi.json`

## Notes on Current Implementation

A few details are intentionally aligned with (or differ from) the original reference implementations:

- Multipart parsing treats any part with `filename` as an image (field name does not matter).
- Image uploads are sent to Bilibili BFS endpoint `POST /x/dynamic/feed/draw/upload_bfs` with form fields: `file_up`, `biz=draw`, `category=daily`, `csrf`.
- Dynamic creation is sent to `POST /x/dynamic/feed/create/dyn?platform=web&csrf=...` with JSON body containing `dyn_req`.
- `upload_id` does not include UID (current format: `{timestamp_seconds}_{nonce}`).
- For Bilibili create failures (`code != 0`), this API returns HTTP 200 with `{ code: 1, exception: <bilibili response> }`.

## Deployment Considerations

### Security

- Keep your `config.toml` file secure and never commit it to version control
- Use environment-specific configuration files
- Consider adding rate limiting at the infrastructure level
- Monitor for failed authentication attempts

### Performance

- The shared HTTP client provides connection pooling for efficient Bilibili API calls
- Images are processed sequentially - for many images, consider parallel upload (future enhancement)
- Memory usage scales with uploaded image sizes (images kept in memory during upload)

### Error Handling

- All Bilibili API errors are logged using the tracing framework
- Failed uploads will return appropriate error messages
- Network timeouts are handled gracefully

## Troubleshooting

### JWT Token Issues
- Ensure the token is passed in the `Authorization: Bearer <token>` header format
- Check that the public/private key pair in config matches the keys used to generate the token
- Regenerate token using the CLI command if needed

### "need msg" error
- Ensure the msg field is present in the request
- Verify the msg field is not empty
- Check that you're using the correct form field name ("msg")

### "upload file fail" error
- Verify your SESSDATA and bili_jct tokens are valid and not expired
- Check that image files are not corrupted
- Ensure images are in supported formats (JPG, PNG, etc.)
- Check Bilibili API status

### Images not showing in dynamic
- Verify the Bilibili API returned valid image URLs
- Check server logs for upload errors
- Ensure uploaded images meet Bilibili's requirements

## Example Integration

### Using with JavaScript/TypeScript

```typescript
async function postToBilibili(text: string, jwtToken: string, images?: File[]) {
  const formData = new FormData();
  formData.append('msg', JSON.stringify([{
    type: 1,
    raw_text: text,
    biz_id: ''
  }]));
  
  if (images) {
    images.forEach((image, index) => {
      formData.append(`image${index}`, image);
    });
  }
  
  const response = await fetch('http://localhost:25150/api/bilibili/createDynamic', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${jwtToken}`
    },
    body: formData
  });
  
  return response.json();
}
```

### Using with Python

```python
import requests

def post_to_bilibili(text, jwt_token, images=None):
    url = 'http://localhost:25150/api/bilibili/createDynamic'
    
    headers = {
        'Authorization': f'Bearer {jwt_token}'
    }
    
    data = {
        'msg': '[{"type":1,"raw_text":"' + text + '","biz_id":""}]'
    }
    
    files = {}
    if images:
        for i, image_path in enumerate(images):
            files[f'image{i}'] = open(image_path, 'rb')
    
    response = requests.post(url, data=data, files=files, headers=headers)
    return response.json()
```

## Development

### Building

```bash
cargo build --release
```

### Running

```bash
cargo run -- server --config config.toml
```

### Testing

The endpoint can be tested with curl, Postman, or any HTTP client that supports multipart form data.

## License

This implementation follows the same license as the main project.
