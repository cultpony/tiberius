use crate::state::State;
use anyhow::Result;
use log::trace;
use log::warn;
use reqwest::header::HeaderMap;
use reqwest::StatusCode;
use tide::Request;

pub type ProxyRet = (StatusCode, HeaderMap, Vec<u8>);
pub async fn raw_proxy(req: &mut Request<State>) -> Result<ProxyRet> {
    /*let forward_header_list = [
        "host",
        "cookie",
        "set-cookie",
        "x-csrf-token",
        "user-agent",
        "referer",
        "content-type",
        "origin",
    ];*/
    trace!("creating client");
    let client: reqwest::Client = crate::http_client()?;
    let mut url = req.url().clone();
    trace!("proxying to {}", url);
    let new_host = &req.state().config.forward_to;
    assert!(
        &new_host.to_string() != &req.state().config.listen_on.to_string(),
        "must not redir to original host"
    );
    trace!("new host is {:?}", new_host);
    url.set_host(Some(&new_host.ip().to_string()))?;
    assert!(url
        .set_port(Some(new_host.port().to_string().parse()?))
        .is_ok());
    trace!("Forwarding to {:?} ({:?})", url.to_string(), url.host_str());
    assert!(
        url.host_str().expect("must have host") != req.state().config.listen_on.to_string(),
        "must not redir to original host url"
    );
    let method = req.method();
    use std::str::FromStr;
    let method = reqwest::Method::from_str(&method.to_string())?;
    let breq = client.request(method, url.clone());
    let body_bytes = req.body_bytes().await;
    let body_bytes = match body_bytes {
        Ok(v) => v,
        Err(e) => anyhow::bail!(e),
    };
    let breq = breq.body(body_bytes);
    let fwd_headers = {
        let mut hdrs = reqwest::header::HeaderMap::new();
        for name in req.header_names() {
            //if forward_header_list.contains(&name.to_string().to_lowercase().as_str()) {
            let vals = req.header(name).expect("header listed but non-present");
            let name = reqwest::header::HeaderName::from_str(&name.to_string())?;
            for val in vals {
                let val = reqwest::header::HeaderValue::from_str(&val.to_string())?;
                hdrs.append(name.clone(), val);
            }
            //}
        }
        hdrs
    };
    trace!("parsed result, forwarding to remote");
    let breq = breq.headers(fwd_headers);
    let res = breq.send().await?;
    Ok((
        res.status(),
        res.headers().clone(),
        res.bytes().await?.to_vec(),
    ))
}

pub async fn forward(mut req: Request<State>) -> tide::Result {
    trace!("forwarding request to {}", req.url());
    match raw_proxy(&mut req).await {
        Err(e) => {
            warn!("proxy error: {}", e);
            todo!("cannot handle proxy errors yet");
        }
        Ok((status, headers, body)) => {
            let mut tres = tide::Response::new(status.as_u16());
            trace!("result ok: {:?}", status);
            for (key, value) in headers {
                if let Some(key) = key {
                    tres.append_header(
                        tide::http::headers::HeaderName::from_string(key.to_string())?,
                        value.to_str()?,
                    )
                }
            }
            tres.set_body(tide::Body::from_bytes(body));
            trace!("data passed, returning result");
            Ok(tres)
        }
    }
}
