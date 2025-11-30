use diesel::pg::PgConnection;
use diesel::r2d2::{self, ConnectionManager, Pool, PoolError};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use dotenv::dotenv;
use tracing::info;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Database connection error: {0}")]
    ConnectionError(#[from] PoolError),

    #[error("Diesel error: {0}")]
    DieselError(#[from] diesel::result::Error),

    #[error("DATABASE_URL not set")]
    DatabaseUrlNotSet,

    #[error("Error running migrations: {0}")]
    MigrationError(String),

    #[error("HTLC not found: {0}")]
    HTLCNotFound(String),

    #[error("Operation not found: {0}")]
    OperationNotFound(String),
}

#[derive(Clone)]
pub struct Database {
    pool: DbPool,
}


impl Database {
    pub fn new(database_url: &str, max_connections: u32) -> Result<Self, DatabaseError> {
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder().max_size(max_connections).build(manager)?;

        Ok(Database { pool })
    }

    pub fn from_env() -> Result<Self, DatabaseError> {
        dotenv().ok();

        let database_url =
            std::env::var("DATABASE_URL").map_err(|_| DatabaseError::DatabaseUrlNotSet)?;

        let max_connections = std::env::var("DATABASE_MAX_CONNECTIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);

        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder().max_size(max_connections).build(manager)?;

        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "prod".into());
        if env == "dev" {
            Self::run_migrations_static(&pool)?;
        }

        Ok(Database { pool })
    }

    pub fn get_connection(
        &self,
    ) -> Result<r2d2::PooledConnection<ConnectionManager<PgConnection>>, DatabaseError> {
        Ok(self.pool.get()?)
    }

    fn run_migrations_static(pool: &DbPool) -> Result<(), DatabaseError> {
        info!("🔄 Running database migrations...");
        let mut conn = pool.get()?;
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|e| DatabaseError::MigrationError(e.to_string()))?;
        info!("✅ Migrations completed");
        Ok(())
    }

    pub fn run_migrations(&self) -> Result<(), DatabaseError> {
        Self::run_migrations_static(&self.pool)
    }
}