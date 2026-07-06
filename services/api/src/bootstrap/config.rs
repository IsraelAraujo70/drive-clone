use std::env;

use tower_http::cors::{AllowOrigin, Any, CorsLayer};

pub const DEFAULT_MAX_FILE_SIZE_BYTES: i64 = 15 * 1024 * 1024 * 1024;
pub const DEFAULT_PRESIGNED_URL_TTL_SECONDS: i64 = 900;
pub const DEFAULT_PUBLIC_WEB_URL: &str = "http://localhost:3000";

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: String,
    pub database_url: String,
    pub max_file_size_bytes: i64,
    pub presigned_url_ttl_seconds: i64,
    pub public_web_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT").unwrap_or_else(|_| "8080".to_string()),
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            max_file_size_bytes: env_i64("MAX_FILE_SIZE_BYTES", DEFAULT_MAX_FILE_SIZE_BYTES),
            presigned_url_ttl_seconds: env_i64(
                "PRESIGNED_URL_TTL_SECONDS",
                DEFAULT_PRESIGNED_URL_TTL_SECONDS,
            ),
            public_web_url: env::var("PUBLIC_WEB_URL")
                .unwrap_or_else(|_| DEFAULT_PUBLIC_WEB_URL.to_string()),
        }
    }

    pub fn from_env_defaults() -> Self {
        Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT").unwrap_or_else(|_| "8080".to_string()),
            database_url: env::var("DATABASE_URL").unwrap_or_default(),
            max_file_size_bytes: env_i64("MAX_FILE_SIZE_BYTES", DEFAULT_MAX_FILE_SIZE_BYTES),
            presigned_url_ttl_seconds: env_i64(
                "PRESIGNED_URL_TTL_SECONDS",
                DEFAULT_PRESIGNED_URL_TTL_SECONDS,
            ),
            public_web_url: env::var("PUBLIC_WEB_URL")
                .unwrap_or_else(|_| DEFAULT_PUBLIC_WEB_URL.to_string()),
        }
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone)]
pub struct CorsConfig {
    allowed_origins: Option<String>,
}

impl CorsConfig {
    pub fn from_env() -> Self {
        Self {
            allowed_origins: env::var("CORS_ALLOWED_ORIGINS").ok(),
        }
    }

    pub fn layer(&self) -> CorsLayer {
        let origin = match &self.allowed_origins {
            Some(list) => AllowOrigin::list(list.split(',').filter_map(|o| o.trim().parse().ok())),
            // ponytail: permissive default for local dev; production must set CORS_ALLOWED_ORIGINS
            None => AllowOrigin::any(),
        };
        CorsLayer::new()
            .allow_origin(origin)
            .allow_methods(Any)
            .allow_headers(Any)
    }
}

fn env_i64(name: &str, default: i64) -> i64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
