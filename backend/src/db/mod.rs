use std::time::Duration;

use sqlx::{PgPool, postgres::PgPoolOptions};

pub fn create_pool(database_url: Option<&str>) -> Result<Option<PgPool>, sqlx::Error> {
    database_url
        .map(|url| {
            PgPoolOptions::new()
                .max_connections(5)
                .acquire_timeout(Duration::from_secs(5))
                .connect_lazy(url)
        })
        .transpose()
}

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
