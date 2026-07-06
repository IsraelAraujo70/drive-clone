use std::sync::Arc;
use std::time::{Duration, Instant};

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::adapters::object_storage::s3::S3ObjectStorage;
use crate::adapters::postgres::PostgresFileRepository;
use crate::application::files::{
    CleanupOrphanObjectsUseCase, DEFAULT_ORPHAN_MIN_AGE_SECONDS, ExpireResumableUploadsUseCase,
    PurgeTrashUseCase, ReconcileQuotaUseCase,
};
use crate::application::ports::clock::{Clock, SystemClock};
use crate::application::ports::files::FileRepository;
use crate::application::ports::object_storage::ObjectStorage;
use crate::bootstrap::config::Config;

const EXPIRE_BATCH_LIMIT: i64 = 100;
const PURGE_BATCH_LIMIT: i64 = 100;
const RECONCILE_BATCH_SIZE: i64 = 500;

pub async fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = Config::from_env();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("failed to connect to postgres");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let storage: Arc<dyn ObjectStorage> =
        Arc::new(S3ObjectStorage::from_env().expect("S3 storage env vars must be set"));
    let jobs = WorkerJobs::new(pool, storage, Arc::new(SystemClock), &config);
    let interval_seconds = config.worker_interval_seconds.max(1) as u64;

    tracing::info!(
        interval_seconds,
        trash_retention_days = config.trash_retention_days,
        "drive-clone-worker starting"
    );

    let mut ticker = tokio::time::interval(Duration::from_secs(interval_seconds));
    loop {
        ticker.tick().await;
        jobs.run_tick().await;
    }
}

pub struct WorkerJobs {
    expire_uploads: ExpireResumableUploadsUseCase,
    purge_trash: PurgeTrashUseCase,
    cleanup_orphans: CleanupOrphanObjectsUseCase,
    reconcile_quota: ReconcileQuotaUseCase,
}

impl WorkerJobs {
    pub fn new(
        pool: PgPool,
        storage: Arc<dyn ObjectStorage>,
        clock: Arc<dyn Clock>,
        config: &Config,
    ) -> Self {
        let file_repository: Arc<dyn FileRepository> = Arc::new(PostgresFileRepository::new(pool));
        Self {
            expire_uploads: ExpireResumableUploadsUseCase::new(
                file_repository.clone(),
                storage.clone(),
                clock.clone(),
            ),
            purge_trash: PurgeTrashUseCase::new(
                file_repository.clone(),
                storage.clone(),
                clock.clone(),
                config.trash_retention_days,
            ),
            cleanup_orphans: CleanupOrphanObjectsUseCase::new(
                file_repository.clone(),
                storage,
                clock,
                DEFAULT_ORPHAN_MIN_AGE_SECONDS,
            ),
            reconcile_quota: ReconcileQuotaUseCase::new(file_repository, RECONCILE_BATCH_SIZE),
        }
    }

    pub async fn run_tick(&self) {
        let start = Instant::now();
        self.run_expire_uploads().await;
        self.run_purge_trash().await;
        self.run_cleanup_orphans().await;
        self.run_reconcile_quota().await;
        tracing::info!(
            duration_ms = start.elapsed().as_millis() as u64,
            "worker tick complete"
        );
    }

    async fn run_expire_uploads(&self) {
        let start = Instant::now();
        match self.expire_uploads.execute(EXPIRE_BATCH_LIMIT).await {
            Ok(output) => tracing::info!(
                job = "expire_resumable_uploads",
                expired = output.expired_count,
                aborted = output.aborted_count,
                errors = 0,
                duration_ms = start.elapsed().as_millis() as u64,
                "job complete"
            ),
            Err(error) => tracing::error!(
                job = "expire_resumable_uploads",
                duration_ms = start.elapsed().as_millis() as u64,
                ?error,
                "job failed"
            ),
        }
    }

    async fn run_purge_trash(&self) {
        let start = Instant::now();
        match self.purge_trash.execute(PURGE_BATCH_LIMIT).await {
            Ok(output) => tracing::info!(
                job = "purge_trash",
                purged_files = output.purged_files,
                purged_folders = output.purged_folders,
                errors = output.failed_files,
                duration_ms = start.elapsed().as_millis() as u64,
                "job complete"
            ),
            Err(error) => tracing::error!(
                job = "purge_trash",
                duration_ms = start.elapsed().as_millis() as u64,
                ?error,
                "job failed"
            ),
        }
    }

    async fn run_cleanup_orphans(&self) {
        let start = Instant::now();
        match self.cleanup_orphans.execute().await {
            Ok(output) => tracing::info!(
                job = "cleanup_orphan_objects",
                scanned = output.scanned,
                deleted = output.deleted,
                errors = output.failed,
                duration_ms = start.elapsed().as_millis() as u64,
                "job complete"
            ),
            Err(error) => tracing::error!(
                job = "cleanup_orphan_objects",
                duration_ms = start.elapsed().as_millis() as u64,
                ?error,
                "job failed"
            ),
        }
    }

    async fn run_reconcile_quota(&self) {
        let start = Instant::now();
        match self.reconcile_quota.execute().await {
            Ok(output) => tracing::info!(
                job = "reconcile_quota",
                corrected = output.corrected(),
                errors = 0,
                duration_ms = start.elapsed().as_millis() as u64,
                "job complete"
            ),
            Err(error) => tracing::error!(
                job = "reconcile_quota",
                duration_ms = start.elapsed().as_millis() as u64,
                ?error,
                "job failed"
            ),
        }
    }
}
