
#[get("/v3/manage/keys")]
pub async fn manage_keys_page(state: State<TiberiusState>, rstate: TiberiusRequestState<'_>) -> TiberiusResult<()> {
    let body = html!{
    };
    let mut client = state.get_db_client().await?;
    let app = crate::pages::common::frontmatter::app(
        state,
        &rstate,
        Some(PageTitle::from("API - Manage API Keys")),
        &mut client,
        body,
        None,
    )
    .await?;
    Ok(TiberiusResponse::Html(HtmlResponse {
        content: app.into_string(),
    }))
}

#[post("/v3/manage/keys/create")]
pub async fn create_api_key(state: State<TiberiusState>, rstate: TiberiusRequestState<'_>) -> TiberiusResult<()> {
    todo!("implement API page")
}

#[post("/v3/manage/keys/delete")]
pub async fn delete_api_key(state: State<TiberiusState>, rstate: TiberiusRequestState<'_>) -> TiberiusResult<()> {
    todo!("implement API page")
}