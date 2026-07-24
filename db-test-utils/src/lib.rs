use diesel::{Connection, PgConnection, RunQueryDsl};
use std::env;

pub struct PostgresTestDB {
    pub conn: PgConnection,
    pub database_url: String,
    pub db_name: String,
}

impl PostgresTestDB {
    fn test_database_url() -> String {
        env::var("TEST_POSTGRES_DATABASE_URL")
            .unwrap_or_else(|_e| String::from("postgres://postgres:postgres@localhost:5432"))
    }

    pub fn new() -> Self {
        let database_url = Self::test_database_url();

        let mut conn =
            PgConnection::establish(&database_url).expect("Cannot connect to postgres database.");

        let db_name = format!("test_db_{}", rand::random::<u16>());

        // Create a new database for the test
        let query = diesel::sql_query(format!("CREATE DATABASE {db_name};").as_str());
        query.execute(&mut conn).unwrap();

        Self {
            conn,
            database_url,
            db_name,
        }
    }

    /// Execute a SQL query against the test database
    pub fn execute(&self, query: &str) {
        let mut conn = PgConnection::establish(&self.con_string()).unwrap();
        diesel::sql_query(query).execute(&mut conn).unwrap();
    }

    pub fn con_string(&self) -> String {
        format!("{}/{}", self.database_url, self.db_name)
    }

    pub fn drop_database(&mut self) {
        let mut admin_conn = PgConnection::establish(&self.database_url).unwrap();

        let disconnect_query = format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}';",
            self.db_name
        );

        let _ = diesel::sql_query(disconnect_query).execute(&mut admin_conn);

        let mut drop_conn = PgConnection::establish(&self.database_url).unwrap();

        let drop_query = format!("DROP DATABASE IF EXISTS {};", self.db_name);
        diesel::sql_query(drop_query)
            .execute(&mut drop_conn)
            .unwrap();
    }
}

impl Drop for PostgresTestDB {
    fn drop(&mut self) {
        self.drop_database();
    }
}

impl Default for PostgresTestDB {
    fn default() -> Self {
        Self::new()
    }
}
