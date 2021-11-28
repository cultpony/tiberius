use clap::ArgMatches;
use tiberius_core::config::Configuration;
use tiberius_core::error::TiberiusResult;


pub async fn verify_db(matches: &ArgMatches<'_>) -> TiberiusResult<()> {
        let table = matches.value_of("table");
        let table = match table {
            None => {
                error!("require to know which table to verify");
                return Ok(());
            }
            Some(t) => t,
        };
        let start_id = matches.value_of("start-id");
        let start_id: u64 = match start_id {
            None => {
                error!("require to know where to start verify");
                return Ok(());
            }
            Some(t) => t.parse().expect("can't parse start id"),
        };
        let stop_id = matches.value_of("stop-id");
        let stop_id: u64 = match stop_id {
            None => {
                error!("require to know where to stop verify");
                return Ok(());
            }
            Some(t) => t.parse().expect("can't parse stop id"),
        };
        let config = envy::from_env::<Configuration>().expect("could not parse config");
        info!("Starting with config {:?}", config);
        let db_conn = config
            .db_conn()
            .await
            .expect("could not establish db connection");
        use tiberius_models::*;
        let client = Client::new(db_conn.clone(), &config.search_dir);
        let mut table: Box<dyn tiberius_models::VerifiableTable> = match table {
            "images" => Image::verifier(client, db_conn, start_id, stop_id, 10240),
            v => {
                error!("table {} is invalid", v);
                return Ok(());
            }
        };
        table.verify().await?;
        Ok(())
}