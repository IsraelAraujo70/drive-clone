# API Service

Rust backend for the Google Drive clone.

Current scope:

- Provide a deployable HTTP service.
- Expose `/health` for Railway health checks.
- Establish the backend service boundary before product endpoints are added.

Local commands:

```bash
cargo test
cargo run
```

The server reads:

- `HOST`, default `0.0.0.0`
- `PORT`, default `8080`
