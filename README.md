# marginilia

DIY e-ink display that rotates saved quotes. An ESP32 display detects your phone via BLE, checks if its quotes are stale, and refreshes from a backend API.

## Repo Structure

```
marginilia/
├── server/     # Rust/Axum backend — serves quotes over HTTP
├── app/        # React Native iOS app — BLE bridge between server and device
└── device/     # ESP32 embedded code (e-ink display)
```

## Server

### Running locally

```bash
cd server
API_KEY=your-secret-key cargo run
```

The server reads `quotes.yaml` from the current directory by default. Override with `QUOTES_PATH`.

### Environment variables

| Variable      | Default        | Description                          |
|---------------|----------------|--------------------------------------|
| `API_KEY`     | (required)     | Pre-shared key sent in `x-api-key` header |
| `QUOTES_PATH` | `quotes.yaml`  | Path to the quotes YAML file         |
| `PORT`        | `3000`         | Port to listen on                    |

### API

#### `GET /quotes`

Returns the full quote list with a SHA-256 content checksum.

**Headers:** `x-api-key: <your-key>`

**Response:**
```json
{
  "checksum": "abc123...",
  "quotes": [
    {
      "body": "Quote text here.",
      "author": "Author Name",
      "work": "Book or Article Title"
    }
  ]
}
```

### Quote format (`quotes.yaml`)

```yaml
- body: "The quote text."
  author: "Author Name"
  work: "Source title (book, poem, article)"
```

`work` can be empty string if the source is unknown.
