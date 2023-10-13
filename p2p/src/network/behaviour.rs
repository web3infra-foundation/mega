use entity::git_obj;
use libp2p::kad::store::MemoryStore;
use libp2p::kad::{Kademlia, KademliaEvent};
use libp2p::swarm::NetworkBehaviour;
use libp2p::{dcutr, identify, relay, rendezvous, request_response};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "Event")]
pub struct Behaviour {
    pub relay_client: relay::client::Behaviour,
    pub identify: identify::Behaviour,
    pub dcutr: dcutr::Behaviour,
    pub kademlia: Kademlia<MemoryStore>,
    pub rendezvous: rendezvous::client::Behaviour,
    pub git_upload_pack: request_response::cbor::Behaviour<GitUploadPackReq, GitUploadPackRes>,
    pub git_info_refs: request_response::cbor::Behaviour<GitInfoRefsReq, GitInfoRefsRes>,
    pub git_object: request_response::cbor::Behaviour<GitObjectReq, GitObjectRes>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitUploadPackReq(
    //want
    pub HashSet<String>,
    //have
    pub HashSet<String>,
    //path
    pub String,
);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitUploadPackRes(pub Vec<u8>, pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitInfoRefsReq(pub String, pub Vec<String>);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitInfoRefsRes(pub String, pub Vec<String>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitObjectReq(pub String, pub Vec<String>);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitObjectRes(pub Vec<git_obj::Model>);

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Event {
    Identify(identify::Event),
    RelayClient(relay::client::Event),
    Dcutr(dcutr::Event),
    Kademlia(KademliaEvent),
    Rendezvous(rendezvous::client::Event),
    GitUploadPack(request_response::Event<GitUploadPackReq, GitUploadPackRes>),
    GitInfoRefs(request_response::Event<GitInfoRefsReq, GitInfoRefsRes>),
    GitObject(request_response::Event<GitObjectReq, GitObjectRes>),
}

impl From<identify::Event> for Event {
    fn from(e: identify::Event) -> Self {
        Event::Identify(e)
    }
}

impl From<relay::client::Event> for Event {
    fn from(e: relay::client::Event) -> Self {
        Event::RelayClient(e)
    }
}

impl From<dcutr::Event> for Event {
    fn from(e: dcutr::Event) -> Self {
        Event::Dcutr(e)
    }
}

impl From<KademliaEvent> for Event {
    fn from(e: KademliaEvent) -> Self {
        Event::Kademlia(e)
    }
}

impl From<rendezvous::client::Event> for Event {
    fn from(e: rendezvous::client::Event) -> Self {
        Event::Rendezvous(e)
    }
}

impl From<request_response::Event<GitUploadPackReq, GitUploadPackRes>> for Event {
    fn from(event: request_response::Event<GitUploadPackReq, GitUploadPackRes>) -> Self {
        Event::GitUploadPack(event)
    }
}

impl From<request_response::Event<GitInfoRefsReq, GitInfoRefsRes>> for Event {
    fn from(event: request_response::Event<GitInfoRefsReq, GitInfoRefsRes>) -> Self {
        Event::GitInfoRefs(event)
    }
}

impl From<request_response::Event<GitObjectReq, GitObjectRes>> for Event {
    fn from(event: request_response::Event<GitObjectReq, GitObjectRes>) -> Self {
        Event::GitObject(event)
    }
}
