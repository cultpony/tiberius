use tiberius_core::config::Configuration;
use tiberius_core::error::TiberiusResult;

pub async fn getconfres(config: Configuration) -> TiberiusResult<()> {
    println!("Configuration:");
    println!(
        "- Data Root: {:?}",
        config
            .data_root
            .as_ref()
            .map(|x| x.canonicalize().expect("data root invalid"))
    );
    println!(
        "- Image Base Path: {:?}",
        config
            .image_base()
            .canonicalize()
            .expect("image base path invalid")
            .display()
    );
    println!("- Sentry URL: {:?}", config.sentry_url);
    println!("- Sentry Ratio: {:?}", config.sentry_ratio);

    Ok(())
}
