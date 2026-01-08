# Bilibili Dynamic Posting API

This document describes the Bilibili dynamic posting functionality implemented in this project.

## Overview

The `/api/createDynamic` endpoint allows posting text and image content to Bilibili as dynamic posts. This implementation is functionally equivalent to the Node.js reference implementation, using Rust with Axum, multipart form handling, and reqwest for HTTP requests.

**Authentication:** This endpoint requires JWT authentication using ES256 algorithm.

## Configuration

Add the following sections to your `config.toml`:

```toml
[bilibili]
sessdata = "your_bilibili_sessdata_cookie"
csrf = "your_bilibili_csrf_token"
uid = "your_bilibili_user_id"

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
- **sessdata** (required): Your Bilibili SESSDATA cookie value. This is used for authentication with Bilibili's API.
- **csrf** (required): Your Bilibili CSRF token. This is required for all POST requests to Bilibili's API.
- **uid** (required): Your Bilibili user ID. Used to generate unique upload IDs.

**JWT Config:**
- **private_key** (required): ES256 private key in PEM format for signing JWT tokens.
- **public_key** (required): ES256 public key in PEM format for verifying JWT tokens.

### Generating JWT Keys

Generate ES256 key pair using OpenSSL:

```bash
# Generate private key
openssl ecparam -genkey -name prime256v1 -noout -out private.pem

# Extract public key
openssl ec -in private.pem -pubout -out public.pem

# View keys to copy into config
cat private.pem
cat public.pem
```

### Obtaining Bilibili Credentials

1. **SESSDATA**: Log into Bilibili in your browser and extract the SESSDATA cookie value from your browser's developer tools (Application/Storage > Cookies)
2. **CSRF**: This is typically available in the bili_jct cookie
3. **UID**: Your Bilibili user ID, visible in your profile URL

## Generating JWT Tokens

Generate a JWT token using the CLI command:

```bash
cargo run -- generate-jwt --config config.toml --subject user_id --expires-in 2592000
```

Options:
- `--config`: Path to config file (default: config.toml)
- `--subject`: Subject identifier (e.g., user ID or username)
- `--expires-in`: Token expiration time in seconds (default: 2592000 = 30 days)

## API Endpoint

### POST `/api/createDynamic`

Creates a new Bilibili dynamic post with optional images.

**Authentication:** Required via `Authorization: Bearer <jwt_token>` header

#### Request

**Headers:**
- **Authorization** (required): `Bearer <jwt_token>`

**Content-Type:** `multipart/form-data`

**Form Fields:**

- **msg** (required, string): JSON string containing the dynamic content structure
- **files** (optional, files): One or more image files to attach to the dynamic

#### Request Examples

**Text-only dynamic:**

```bash
curl -X POST http://localhost:25150/api/createDynamic \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -F 'msg=[{"type":1,"raw_text":"Hello from Rust API!","biz_id":""}]'
```

**Dynamic with a single image:**

```bash
curl -X POST http://localhost:25150/api/createDynamic \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -F 'msg=[{"type":1,"raw_text":"Check out this image!","biz_id":""}]' \
  -F "image=@photo.jpg"
```

**Dynamic with multiple images:**

```bash
curl -X POST http://localhost:25150/api/createDynamic \
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
  "code": 0
}
```

**Error Response (HTTP 200/400/500):**

```json
{
  "code": 1,
  "msg": "error description",
  "exception": { /* optional error details */ }
}
```

#### Error Codes and Messages

| code | msg | Description |
|------|-----|-------------|
| 1 | "Missing authorization header" | Authorization header not provided |
| 1 | "Invalid authorization header" | Authorization header format is invalid |
| 1 | "Invalid authorization format, expected: Bearer <token>" | Authorization scheme is not Bearer |
| 1 | "Token expired" | JWT token has expired |
| 1 | "Invalid token" | JWT token is malformed or invalid |
| 1 | "Invalid signature" | JWT signature verification failed |
| 1 | "need msg" | The msg field is missing or empty |
| 1 | "upload file fail" | One or more images failed to upload to Bilibili |
| 1 | "create dynamic fail" | Dynamic creation request failed |
| 1 | "create dynamic fail with network fatal" | Network error during dynamic creation |

**Note:** Error messages for Bilibili API operations are kept compatible with the Node.js reference implementation.

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
2. Server validates API key
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
{user_id}_{unix_timestamp}_{random_nonce}
```

Where:
- `user_id`: The configured Bilibili UID
- `unix_timestamp`: Current time in seconds since Unix epoch
- `random_nonce`: Random 4-digit number (1000-9999)

### Authentication Flow

The endpoint uses multiple layers of security:

1. **JWT Authentication**: Protects the endpoint with ES256 signed tokens
   - Tokens must be included in the `Authorization: Bearer <token>` header
   - Tokens expire after the configured duration (default: 30 days)
   - Tokens are verified using the public key from configuration
2. **Bilibili SESSDATA**: Authenticates requests to Bilibili's API as your user
3. **CSRF Token**: Prevents cross-site request forgery attacks on Bilibili's API

### HTTP Client

The implementation uses a shared `reqwest::Client` instance stored in `AppState` for efficient connection pooling and reuse across requests.

## OpenAPI Documentation

The API is fully documented using OpenAPI/Swagger. Access the interactive documentation at:

- **Scalar UI**: `http://localhost:25150/api/scalar`
- **OpenAPI JSON**: `http://localhost:25150/api/openapi.json`

## Compatibility with Node.js Reference

This implementation is functionally equivalent to the Node.js reference implementation:

✅ Supports multipart form data with text and images  
✅ JWT authentication (replaces API key for better security)  
✅ Bilibili SESSDATA and CSRF validation  
✅ Two-step process: upload images first, then create dynamic  
✅ Compatible error response format and codes  
✅ Support for both text-only and image dynamics  
✅ Proper scene selection (1 for text, 2 for images)  

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
- Verify your token is not expired
- Ensure the token is passed in the `Authorization: Bearer <token>` header format
- Check that the public/private key pair in config matches the keys used to generate the token
- Regenerate token using the CLI command if needed

### "need msg" error
- Ensure the msg field is present in the request
- Verify the msg field is not empty
- Check that you're using the correct form field name ("msg")

### "upload file fail" error
- Verify your SESSDATA and CSRF tokens are valid and not expired
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
  
  const response = await fetch('http://localhost:25150/api/createDynamic', {
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
    url = 'http://localhost:25150/api/createDynamic'
    
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
