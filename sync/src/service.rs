
use axum::body::Body;
use bytes::{BytesMut};
use futures::StreamExt;
use hyper::Request;
use crate::dto::{issue::IssueEventDto, pull_request::PullRequestEventDto};



pub async fn resolve_issue_event(
    req: Request<Body>,
)-> IssueEventDto {
    tracing::info!("req: {:?}", req);
    let (_parts, mut body) = req.into_parts();

    let mut request_body = BytesMut::new();

    while let Some(chunk) = body.next().await {
        tracing::info!("client sends :{:?}", chunk);
        let bytes = chunk.unwrap();
        request_body.extend_from_slice(&bytes);
    }

    let issue_dto: IssueEventDto = serde_json::from_slice(request_body.freeze().as_ref()).unwrap();
    println!("{:?}", issue_dto);

    issue_dto
    //Ok(resp.body(body).unwrap())
}

pub async fn resolve_pull_request_event(
    req: Request<Body>,
)-> PullRequestEventDto {
    tracing::info!("req: {:?}", req);
    let (_parts, mut body) = req.into_parts();

    let mut request_body = BytesMut::new();

    while let Some(chunk) = body.next().await {
        tracing::info!("client sends :{:?}", chunk);
        let bytes = chunk.unwrap();
        request_body.extend_from_slice(&bytes);
    }

    let pull_request_dto: PullRequestEventDto = serde_json::from_slice(request_body.freeze().as_ref()).unwrap();
    println!("{:?}", pull_request_dto);

    pull_request_dto
    //Ok(resp.body(body).unwrap())
}