use std::collections::HashMap;
use std::io::prelude::*;

use anyhow::Result;
use axum::body::Body;
use axum::http::{Response, StatusCode};
use bytes::{BufMut, BytesMut};
use chrono::{prelude::*, Duration};
use database::driver::lfs::storage::{ContentStore, MetaObject};
use database::driver::lfs::structs::BatchResponse;
use database::driver::lfs::structs::*;
use futures::StreamExt;
use hyper::Request;
use rand::prelude::*;
use crate::dto::issue::IssueEventDto;

pub async fn resolve_issue_event(
    req: Request<Body>,
) {
    tracing::info!("req: {:?}", req);
    let mut resp = Response::builder();
    resp = resp.header("Content-Type", "application/vnd.git-lfs+json");

    let (_parts, mut body) = req.into_parts();

    let mut request_body = BytesMut::new();

    while let Some(chunk) = body.next().await {
        tracing::info!("client sends :{:?}", chunk);
        let bytes = chunk.unwrap();
        request_body.extend_from_slice(&bytes);
    }
    
    let issue_dto: IssueEventDto = serde_json::from_slice(request_body.freeze().as_ref()).unwrap();
    println!("{:?}", issue_dto);
    

    //Ok(resp.body(body).unwrap())
}
