use crate::package_full;

#[get("/.well-known/imageboard-type")]
pub async fn imageboardtype() -> String {
    format!(
        "{},min-api:{},max-api:{},api-flavor:{},flavor-philomena-int:{},flavor-philomena:{}",
        package_full(),
        1,
        1,
        "tiberius",
        "!1",
        "!1"
    )
}

#[get("/.well-known/imageboard-api/flavor-tiberius")]
pub async fn imageboardapiflavor() -> String {
    format!("/api/v1")
}

#[get("/.well-known/imageboard-api/flavor-philomena-int")]
pub async fn imageboardapiflavor_philomena_int() -> String {
    format!("!")
}

#[get("/.well-known/imageboard-api/flavor-philomena")]
pub async fn imageboardapiflavor_philomena_v1() -> String {
    format!("/api/philomena/v1")
}
