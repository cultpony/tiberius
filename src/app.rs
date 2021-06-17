pub mod common;
//pub mod cookie_check;
pub mod jobs;

pub type DBPool = sqlx::PgPool;
pub type DBConnection = sqlx::PgConnection;
pub type DBTx<'a> = &'a mut sqlx::Transaction<'a, sqlx::Postgres>;
pub type DBTxOwned<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

pub type HTTPReq<'a> = rocket::Request<'a>;

#[derive(Clone)]
pub struct PageTitle(String);
impl std::convert::Into<String> for PageTitle {
    fn into(self) -> String {
        self.0
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
