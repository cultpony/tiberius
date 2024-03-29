use std::convert::Infallible;

pub type DBPool = sqlx::PgPool;
pub type DBConnection = sqlx::PgConnection;
pub type DBTx<'a> = &'a mut sqlx::Transaction<'a, sqlx::Postgres>;
pub type DBTxOwned<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

#[derive(Clone, Debug)]
pub struct PageTitle(String);

impl std::convert::From<PageTitle> for String {
    fn from(value: PageTitle) -> Self {
        value.0
    }
}

impl std::convert::From<String> for PageTitle {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::convert::From<&str> for PageTitle {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}
