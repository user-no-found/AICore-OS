use std::{path::Path, str::FromStr};

use super::*;
use sqlx::Connection;

fn connect_options(db_path: &Path) -> Result<SqliteConnectOptions, MemoryError> {
    SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))
        .map_err(|error| MemoryError(error.to_string()))
        .map(|options| options.create_if_missing(true))
}

pub async fn connect(db_path: &Path) -> Result<SqliteConnection, MemoryError> {
    let options = connect_options(db_path)?;
    SqliteConnection::connect_with(&options)
        .await
        .map_err(|error| MemoryError(error.to_string()))
}
