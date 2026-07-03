use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::adapters::postgres::tx::map_sqlx_error;
use crate::application::ports::RepositoryError;
use crate::application::ports::auth::{AuthRepository, CreateUserRecord};
use crate::domain::auth::{User, UserWithPassword};

const USER_COLUMNS: &str =
    "id, email, display_name, storage_quota_bytes, storage_used_bytes, created_at";

#[derive(Debug, Clone)]
pub struct PostgresAuthRepository {
    pool: PgPool,
}

impl PostgresAuthRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    email: String,
    display_name: String,
    storage_quota_bytes: i64,
    storage_used_bytes: i64,
    created_at: DateTime<Utc>,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        Self {
            id: row.id,
            email: row.email,
            display_name: row.display_name,
            storage_quota_bytes: row.storage_quota_bytes,
            storage_used_bytes: row.storage_used_bytes,
            created_at: row.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct UserWithPasswordRow {
    id: Uuid,
    email: String,
    display_name: String,
    storage_quota_bytes: i64,
    storage_used_bytes: i64,
    created_at: DateTime<Utc>,
    password_hash: String,
}

impl From<UserWithPasswordRow> for UserWithPassword {
    fn from(row: UserWithPasswordRow) -> Self {
        Self {
            user: User {
                id: row.id,
                email: row.email,
                display_name: row.display_name,
                storage_quota_bytes: row.storage_quota_bytes,
                storage_used_bytes: row.storage_used_bytes,
                created_at: row.created_at,
            },
            password_hash: row.password_hash,
        }
    }
}

#[async_trait]
impl AuthRepository for PostgresAuthRepository {
    async fn create_user(&self, input: CreateUserRecord) -> Result<User, RepositoryError> {
        let query = format!(
            "INSERT INTO users (email, password_hash, display_name) VALUES ($1, $2, $3) RETURNING {USER_COLUMNS}"
        );
        match sqlx::query_as::<_, UserRow>(&query)
            .bind(&input.email)
            .bind(&input.password_hash)
            .bind(&input.display_name)
            .fetch_one(&self.pool)
            .await
        {
            Ok(user) => Ok(user.into()),
            Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
                Err(RepositoryError::DuplicateEmail)
            }
            Err(error) => Err(map_sqlx_error(error)),
        }
    }

    async fn find_user_with_password_by_email(
        &self,
        email: &str,
    ) -> Result<Option<UserWithPassword>, RepositoryError> {
        let query = format!("SELECT {USER_COLUMNS}, password_hash FROM users WHERE email = $1");
        sqlx::query_as::<_, UserWithPasswordRow>(&query)
            .bind(email)
            .fetch_optional(&self.pool)
            .await
            .map(|row| row.map(Into::into))
            .map_err(map_sqlx_error)
    }

    async fn create_session(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query("INSERT INTO sessions (token_hash, user_id, expires_at) VALUES ($1, $2, $3)")
            .bind(token_hash)
            .bind(user_id)
            .bind(expires_at)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(map_sqlx_error)
    }

    async fn find_user_by_session_hash(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<User>, RepositoryError> {
        let query = format!(
            "SELECT {} FROM sessions s JOIN users u ON u.id = s.user_id \
             WHERE s.token_hash = $1 AND s.expires_at > $2",
            USER_COLUMNS
                .split(", ")
                .map(|column| format!("u.{column}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        sqlx::query_as::<_, UserRow>(&query)
            .bind(token_hash)
            .bind(now)
            .fetch_optional(&self.pool)
            .await
            .map(|row| row.map(Into::into))
            .map_err(map_sqlx_error)
    }

    async fn delete_session(&self, token_hash: &str) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(token_hash)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(map_sqlx_error)
    }
}
