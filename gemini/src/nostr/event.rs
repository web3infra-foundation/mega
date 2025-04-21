use crate::nostr::kind::NostrKind;
use crate::nostr::tag::Tag;
use crate::util::get_utc_timestamp;
use callisto::relay_nostr_event;
use secp256k1::hashes::hex::HexToArrayError;
use secp256k1::hashes::{sha256, Hash};
use secp256k1::schnorr::Signature;
use secp256k1::{rand, Message, Secp256k1};
use secp256k1::{Keypair, XOnlyPublicKey};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt;
use std::str::FromStr;
use vault::init;

use super::GitEvent;

/// [`NostrEvent`] error
#[derive(Debug)]
pub enum Error {
    /// Invalid signature
    InvalidSignature,
    /// Invalid event id
    InvalidId,
    /// Error serializing or deserializing JSON data
    Json(serde_json::Error),
    /// Secp256k1 error
    Secp256k1(secp256k1::Error),
    /// Hex decoding error
    Hex(HexToArrayError),
}
#[derive(Debug)]
pub enum ConversionError {
    InvalidParas,
    InvalidPubkey,
    InvalidSignature,
    JsonError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSignature => write!(f, "Invalid signature"),
            Self::InvalidId => write!(f, "Invalid event id"),
            Self::Json(e) => write!(f, "Json: {e}"),
            Self::Secp256k1(e) => write!(f, "Secp256k1: {e}"),
            Self::Hex(e) => write!(f, "Hex: {e}"),
        }
    }
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSignature => write!(f, "Invalid signature"),
            Self::InvalidParas => write!(f, "Invalid paras"),
            Self::InvalidPubkey => write!(f, "Invalid pubkey"),
            Self::JsonError => write!(f, "Invalid json"),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

impl From<secp256k1::Error> for Error {
    fn from(e: secp256k1::Error) -> Self {
        Self::Secp256k1(e)
    }
}

impl From<HexToArrayError> for Error {
    fn from(e: HexToArrayError) -> Self {
        Self::Hex(e)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self)
    }
}

///  nostr event.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct NostrEvent {
    pub id: EventId,
    pub pubkey: XOnlyPublicKey,
    pub created_at: i64,
    pub kind: NostrKind,
    pub tags: Vec<Tag>,
    pub content: String,
    pub sig: Signature,
}
impl TryFrom<NostrEvent> for relay_nostr_event::Model {
    type Error = ConversionError;

    fn try_from(n: NostrEvent) -> Result<Self, Self::Error> {
        let tags_string = match serde_json::to_string(&n.tags) {
            Ok(s) => s,
            Err(_) => {
                return Err(ConversionError::JsonError);
            }
        };
        Ok(relay_nostr_event::Model {
            id: n.id.0,
            pubkey: n.pubkey.to_string(),
            created_at: n.created_at,
            kind: n.kind.as_u32() as i32,
            tags: tags_string,
            content: n.content,
            sig: n.sig.to_string(),
        })
    }
}

impl TryFrom<relay_nostr_event::Model> for NostrEvent {
    type Error = ConversionError;

    fn try_from(n: relay_nostr_event::Model) -> Result<Self, Self::Error> {
        let pk = match XOnlyPublicKey::from_str(&n.pubkey) {
            Ok(pk) => pk,
            Err(_) => {
                return Err(ConversionError::InvalidPubkey);
            }
        };
        let tags: Vec<Tag> = match serde_json::from_str(&n.tags) {
            Ok(tags) => tags,
            Err(_) => {
                return Err(ConversionError::JsonError);
            }
        };
        let sig = match Signature::from_str(&n.sig) {
            Ok(sig) => sig,
            Err(_) => {
                return Err(ConversionError::InvalidSignature);
            }
        };
        Ok(NostrEvent {
            id: EventId(n.id),
            pubkey: pk,
            created_at: n.created_at,
            kind: NostrKind::from(n.kind as u64),
            tags,
            content: n.content,
            sig,
        })
    }
}

impl NostrEvent {
    pub async fn new(tags: Vec<Tag>, content: String) -> Self {
        let created_at = get_utc_timestamp();

        let (_, sk) = init().await;
        let secp = Secp256k1::new();
        let keypair = secp256k1::Keypair::from_seckey_str(&secp, &sk).unwrap();

        Self::new_with_timestamp(keypair, created_at, NostrKind::Mega, tags, content)
    }

    pub fn new_with_keypair(
        keypair: Keypair,
        kind: NostrKind,
        tags: Vec<Tag>,
        content: String,
    ) -> Self {
        let created_at = get_utc_timestamp();
        Self::new_with_timestamp(keypair, created_at, kind, tags, content)
    }

    pub fn new_with_timestamp(
        keypair: Keypair,
        created_at: i64,
        kind: NostrKind,
        tags: Vec<Tag>,
        content: String,
    ) -> Self {
        let (pubkey, _) = XOnlyPublicKey::from_keypair(&keypair);
        let json: Value = json!([0, pubkey, created_at, kind, tags, content]);
        let event_str: String = json.to_string();
        let id = EventId(sha256::Hash::hash(event_str.as_bytes()).to_string());
        NostrEvent {
            id: id.clone(),
            pubkey,
            created_at,
            kind,
            tags,
            content,
            sig: sign_without_rng(id.inner(), &keypair),
        }
    }

    pub fn new_git_event(keypair: Keypair, git_event: GitEvent) -> Self {
        let tags: Vec<Tag> = git_event.to_tags();
        NostrEvent::new_with_keypair(keypair, NostrKind::Mega, tags, git_event.content)
    }
    pub fn new_git_event_with_timestamp(
        keypair: Keypair,
        created_at: i64,
        git_event: GitEvent,
    ) -> Self {
        let tags: Vec<Tag> = git_event.to_tags();
        NostrEvent::new_with_timestamp(
            keypair,
            created_at,
            NostrKind::Mega,
            tags,
            git_event.content,
        )
    }

    pub fn from_json<T>(json: T) -> Result<Self, Error>
    where
        T: AsRef<[u8]>,
    {
        Ok(serde_json::from_slice(json.as_ref())?)
    }
    pub fn as_json(&self) -> String {
        json!(self).to_string()
    }

    pub fn from_value(value: Value) -> Result<Self, Error> {
        Ok(serde_json::from_value(value)?)
    }

    /// Verify [`EventId`] and [`Signature`]
    pub fn verify(&self) -> Result<(), Error> {
        // Verify ID
        self.verify_id()?;

        // Verify signature
        self.verify_signature()
    }

    /// Verify if the [`EventId`] it's composed correctly
    pub fn verify_id(&self) -> Result<(), Error> {
        let id: EventId = EventId::new(
            self.pubkey,
            self.created_at,
            self.kind,
            self.tags.clone(),
            self.content.clone(),
        );
        if id == self.id {
            Ok(())
        } else {
            Err(Error::InvalidId)
        }
    }

    /// Verify event [`Signature`]
    pub fn verify_signature(&self) -> Result<(), Error> {
        let secp = Secp256k1::new();
        let hash = sha256::Hash::from_str(self.id.inner().clone().as_str())?;
        let message: Message = Message::from_digest(hash.to_byte_array());
        // let message = Message::from_slice(hash.as_ref())?;
        secp.verify_schnorr(&self.sig, message.as_ref(), &self.pubkey)
            .map_err(|_| Error::InvalidSignature)
    }
}

pub fn sign_with_rng(id: String, keypair: &Keypair) -> Signature {
    let secp = Secp256k1::new();
    let mut rng = rand::thread_rng();
    let hash = sha256::Hash::from_str(id.as_str()).unwrap();
    let message = Message::from_digest(hash.to_byte_array());

    secp.sign_schnorr_with_rng(message.as_ref(), keypair, &mut rng)
}

pub fn sign_without_rng(id: String, keypair: &Keypair) -> Signature {
    let secp = Secp256k1::new();
    let hash = sha256::Hash::from_str(id.as_str()).unwrap();
    let message: Message = Message::from_digest(hash.to_byte_array());
    secp.sign_schnorr_no_aux_rand(message.as_ref(), keypair)
}

/// Event Id
///
/// 32-bytes lowercase hex-encoded sha256 of the the serialized event data
///
/// <https://github.com/nostr-protocol/nips/blob/master/01.md>
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EventId(String);

impl EventId {
    ///Generate [`EventId`]
    pub fn new(
        pubkey: XOnlyPublicKey,
        created_at: i64,
        kind: NostrKind,
        tags: Vec<Tag>,
        content: String,
    ) -> Self {
        let json: Value = json!([0, pubkey, created_at, kind, tags, content]);
        let event_str: String = json.to_string();
        let id = sha256::Hash::hash(event_str.as_bytes()).to_string();
        Self(id)
    }

    pub fn empty() -> Self {
        EventId("".to_string())
    }

    /// Get [`EventId`] as [`String`]
    pub fn inner(&self) -> String {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {

    use secp256k1::{
        hashes::{sha256, Hash},
        rand::{self, rngs::OsRng},
        Keypair, Message, Secp256k1,
    };

    use crate::nostr::{
        event::{sign_without_rng, EventId, NostrEvent},
        kind::NostrKind,
        tag::{Tag, TagKind},
        GitEvent,
    };
    #[test]
    fn test_secp256k1() {
        let secp = Secp256k1::new();
        let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);
        let digest = sha256::Hash::hash("Hello World!".as_bytes());
        let message: Message = Message::from_digest(digest.to_byte_array());

        let sig = secp.sign_ecdsa(&message, &secret_key);
        assert!(secp.verify_ecdsa(&message, &sig, &public_key).is_ok());
    }

    #[test]
    fn test_new_nostr_id() {
        let sk = "6b911fd37cdf5c81d4c0adb1ab7fa822ed253ab0ad9aa18d77257c88b29b718e";
        let secp = Secp256k1::new();
        let keypair = secp256k1::Keypair::from_seckey_str(&secp, sk).unwrap();
        let pk = keypair.x_only_public_key().0;
        let tag = Tag::Generic(TagKind::P, Vec::new());
        let tags: Vec<Tag> = vec![tag];
        let event_id = EventId::new(pk, 123, NostrKind::Mega, tags.clone(), "123".to_string());
        let event_id_right = "bc0d7c8a5c00eff8719c68f6df7f7e31b2a118ba5221807fe850ba689854f467";
        assert_eq!(event_id.inner(), event_id_right);
    }

    #[test]
    fn test_new_nostr_event() {
        let sk = "6b911fd37cdf5c81d4c0adb1ab7fa822ed253ab0ad9aa18d77257c88b29b718e";
        let secp = Secp256k1::new();
        let keypair = secp256k1::Keypair::from_seckey_str(&secp, sk).unwrap();
        let tag = Tag::Generic(TagKind::P, Vec::new());
        let tags: Vec<Tag> = vec![tag];
        let content = "123".to_string();
        let event = NostrEvent::new_with_timestamp(keypair, 1111, NostrKind::Mega, tags, content);
        let s = r#"{
                "content": "123",
                "created_at": 1111,
                "id": "bc3cc3a1499ea0c618df6dc5a300e8ad05f4d3a9a96e40e8a237cec11cc78667",
                "kind": 111,
                "pubkey": "385c3a6ec0b9d57a4330dbd6284989be5bd00e41c535f9ca39b6ae7c521b81cd",
                "sig": "1285a0697eb44bf41f7a1bb063646ac47de8f6664335b791fe4982b1c9cdae1f56d0f1dd0564aeb053fd3cab16c4a0508cef70ac0bec9d79db3f2a2c28426f20",
                "tags": [
                    [
                        "p"
                    ]
                ]
            }"#;
        let value: NostrEvent = serde_json::from_str(s).unwrap();
        assert_eq!(event.as_json(), value.as_json());
    }

    #[test]
    fn test_verify_nostr_event() {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut rand::thread_rng());
        let keypair = Keypair::from_secret_key(&secp, &secret_key);
        let (xonly, _) = keypair.x_only_public_key();

        let kind = NostrKind::Mega;
        let tag = Tag::Generic(TagKind::P, Vec::from(["123".into(), "234".into()]));
        let tags: Vec<Tag> = vec![tag];
        let event_id = EventId::new(xonly, 123, kind, tags.clone(), "123".to_string());

        let signature = sign_without_rng(event_id.inner(), &keypair);
        let event = NostrEvent {
            id: event_id.clone(),
            pubkey: xonly,
            created_at: 123,
            kind,
            tags: tags.clone(),
            content: "123".into(),
            sig: signature,
        };
        let result = event.verify();
        assert!(result.is_ok());
    }

    #[test]
    fn test_nostr_event_git() {
        let sk = "6b911fd37cdf5c81d4c0adb1ab7fa822ed253ab0ad9aa18d77257c88b29b718e";
        let secp = Secp256k1::new();
        let keypair = secp256k1::Keypair::from_seckey_str(&secp, sk).unwrap();

        let git_event = GitEvent {
            peer: "yfeunFhgJGD83pcB4nXjif9eePeLEmQXP17XjQjFXN4c".to_string(),
            uri: "p2p://yfeunFhgJGD83pcB4nXjif9eePeLEmQXP17XjQjFXN4c/8000/third-part/test.git"
                .to_string(),
            action: "repo_update".to_string(),
            r#ref: "".to_string(),
            commit: "93e1191cdaa97cb6182fb906f4dea4300db3c734".to_string(),
            issue: "".to_string(),
            mr: "".to_string(),
            title: "hahaha".to_string(),
            content: "hello".to_string(),
        };
        let event = NostrEvent::new_git_event_with_timestamp(keypair, 123, git_event);
        let json = r#" {
                "content": "hello",
                "created_at": 123,
                "id": "829043ebaf5e8a5060b2f8bd0dc0fff2520607c4864c8dd99c7805c547f295ba",
                "kind": 111,
                "pubkey": "385c3a6ec0b9d57a4330dbd6284989be5bd00e41c535f9ca39b6ae7c521b81cd",
                "sig": "c1cad051321e89579e49e83907bd12021e45fac2dd5d63a4396d145f593b9ce339e8ecad771209059335c491661338ea9c58b17949c955d4be1d6d1ac82742f2",
                "tags": [
                    [
                        "peer",
                        "yfeunFhgJGD83pcB4nXjif9eePeLEmQXP17XjQjFXN4c"
                    ],
                    [
                        "uri",
                        "p2p://yfeunFhgJGD83pcB4nXjif9eePeLEmQXP17XjQjFXN4c/8000/third-part/test.git"
                    ],
                    [
                        "action",
                        "repo_update"
                    ],
                    [
                        "commit",
                        "93e1191cdaa97cb6182fb906f4dea4300db3c734"
                    ],
                    [
                        "title",
                        "hahaha"
                    ]
                ]
            } "#;
        let value: NostrEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.as_json(), value.as_json());
    }
}
