use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};

use callisto::{relay_nostr_event, relay_nostr_req};
use gemini::nostr::{
    client_message::ClientMessage, event::NostrEvent, relay_message::RelayMessage, Req,
};
use serde_json::Value;
use uuid::Uuid;

use crate::service::relay_server::AppState;

pub fn routers() -> Router<AppState> {
    Router::new()
        .route("/nostr", post(receive))
        .route("/nostr/event_list", get(event_list))
        .route("/nostr/req_list", get(req_list))
}

async fn receive(
    state: State<AppState>,
    body: String,
) -> Result<Json<Value>, (StatusCode, String)> {
    // ["EVENT", <event JSON as defined above>], used to publish events.
    // ["REQ", <subscription_id>, <filters1>, <filters2>, ...], used to request events and subscribe to new updates.
    // ["CLOSE", <subscription_id>], used to stop previous subscriptions.
    tracing::info!("relay nostr receive:{}", body);
    let client_msg: ClientMessage = match serde_json::from_str(&body) {
        Ok(client_msg) => client_msg,
        Err(e) => {
            return Err((StatusCode::BAD_REQUEST, e.to_string()));
        }
    };
    match client_msg {
        ClientMessage::Event(nostr_event) => {
            //event message
            match nostr_event.verify() {
                Ok(_) => {}
                Err(e) => {
                    return Err((StatusCode::UNAUTHORIZED, e.to_string()));
                }
            }
            let ztm_nostr_event: relay_nostr_event::Model = match nostr_event.clone().try_into() {
                Ok(n) => n,
                Err(_) => {
                    return Err((StatusCode::BAD_REQUEST, "Invalid paras".to_string()));
                }
            };
            //save
            let storage = state.storage.relay_storage();
            if storage
                .get_nostr_event_by_id(&ztm_nostr_event.id)
                .await
                .unwrap()
                .is_some()
            {
                return Err((StatusCode::BAD_REQUEST, "Duplicate submission".to_string()));
            }
            storage.insert_nostr_event(ztm_nostr_event).await.unwrap();

            //Event is forwarded to subscribed nodes
            // let nostr_event_clone = nostr_event.clone();
            // let storage_clone = storage.clone();
            // let ztm_agent_port = state.relay_option.clone().ztm_agent_port;
            // task::spawn(async move {
            //     transfer_event_to_subscribed_nodes(storage_clone, nostr_event_clone, ztm_agent_port)
            //         .await
            // });

            let res = RelayMessage::new_ok(nostr_event.id, true, "ok".to_string());
            let value = serde_json::to_value(res).unwrap();
            Ok(Json(value))
        }
        ClientMessage::Req {
            subscription_id,
            filters,
        } => {
            //subscribe message
            //save
            let filters_json = serde_json::to_string(&filters).unwrap();
            let ztm_nostr_req = relay_nostr_req::Model {
                subscription_id: subscription_id.to_string(),
                filters: filters_json.clone(),
                id: Uuid::new_v4().to_string(),
            };
            let storage = state.storage.relay_storage();
            let req_list: Vec<Req> = storage
                .get_all_nostr_req_by_subscription_id(&subscription_id.to_string())
                .await
                .unwrap()
                .iter()
                .map(|x| x.clone().into())
                .collect();
            match req_list.iter().find(|&x| x.filters_json() == filters_json) {
                Some(_) => {}
                None => {
                    storage.insert_nostr_req(ztm_nostr_req).await.unwrap();
                }
            }
            let value = serde_json::to_value("ok").unwrap();
            Ok(Json(value))
        }
    }
}

pub async fn event_list(
    Query(_query): Query<HashMap<String, String>>,
    state: State<AppState>,
) -> Result<Json<Vec<NostrEvent>>, (StatusCode, String)> {
    let storage = state.storage.relay_storage();
    let event_list: Vec<NostrEvent> = storage
        .get_all_nostr_event()
        .await
        .unwrap()
        .into_iter()
        .map(|x| x.try_into().unwrap())
        .collect();
    Ok(Json(event_list))
}

pub async fn req_list(
    Query(_query): Query<HashMap<String, String>>,
    state: State<AppState>,
) -> Result<Json<Vec<Req>>, (StatusCode, String)> {
    let storage = state.storage.relay_storage();
    let req_list: Vec<Req> = storage
        .get_all_nostr_req()
        .await
        .unwrap()
        .into_iter()
        .map(|x| x.into())
        .collect();
    Ok(Json(req_list))
}

// async fn transfer_event_to_subscribed_nodes(
//     storage: ZTMStorage,
//     nostr_event: NostrEvent,
//     ztm_agent_port: u16,
// ) {
//     // only support p2p_uri subscription
//     let mut uri = String::new();
//     for tag in nostr_event.clone().tags {
//         if let gemini::nostr::tag::Tag::Generic(TagKind::URI, t) = tag {
//             if !t.is_empty() {
//                 uri = t.first().unwrap().to_string();
//             }
//         }
//     }
//     if uri.is_empty() {
//         return;
//     }
//     let req_list: Vec<Req> = storage
//         .get_all_nostr_req()
//         .await
//         .unwrap()
//         .iter()
//         .map(|x| x.clone().into())
//         .collect();
//     let mut subscription_id_set: HashSet<String> = HashSet::new();
//     for req in req_list {
//         for filter in req.clone().filters {
//             if let Some(uri_vec) = filter.generic_tags.get(&TagKind::URI.to_string()) {
//                 if uri_vec.is_empty() {
//                     continue;
//                 }
//                 let req_uri = uri_vec.first().unwrap();
//                 if *req_uri == uri {
//                     subscription_id_set.insert(req.subscription_id.clone());
//                 }
//             }
//         }
//     }
//
//     for subscription_id in subscription_id_set {
//         //send event
//         let msg = RelayMessage::new_event(
//             SubscriptionId::new(subscription_id.clone()),
//             nostr_event.clone(),
//         )
//         .as_json();
//         match gemini::ztm::send_post_request_to_peer_by_tunnel(
//             ztm_agent_port,
//             subscription_id.clone(),
//             "api/v1/mega/nostr".to_string(),
//             msg,
//         )
//         .await
//         {
//             Ok(_) => {
//                 tracing::info!("send event msg to {} successfully", subscription_id)
//             }
//             Err(e) => {
//                 tracing::error!("send event msg to {} failed:{}", subscription_id, e)
//             }
//         };
//     }
// }
