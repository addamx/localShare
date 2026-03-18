use serde::Serialize;

#[derive(Debug)]
pub struct PersistenceLayer {
    database_path: String,
}

impl PersistenceLayer {
    pub fn new(database_path: String) -> Self {
        Self { database_path }
    }

    pub fn status(&self) -> PersistenceStatus {
        PersistenceStatus {
            database_path: self.database_path.clone(),
            migrations_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistenceStatus {
    pub database_path: String,
    pub migrations_enabled: bool,
}
