use std::fmt;
use std::str::FromStr;
use secp256k1::{hashes, KeyPair, Message, rand, Secp256k1};
use secp256k1::hashes::{Hash};
use secp256k1::XOnlyPublicKey;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::nostr::kind::NostrKind;
use crate::nostr::tag::{Tag, TagKind};
use secp256k1::hashes::sha256::Hash as Sha256Hash;
use secp256k1::schnorr::Signature;
use crate::get_utc_timestamp;

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
    Hex(hashes::hex::Error),
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

impl From<hashes::hex::Error> for Error {
    fn from(e: hashes::hex::Error) -> Self {
        Self::Hex(e)
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

impl NostrEvent {
    pub fn new(
        keypair: KeyPair,
        kind: NostrKind,
        tags: Vec<Tag>,
        content: String,
    ) -> Self {
        let created_at = get_utc_timestamp();
        Self::new_with_timestamp(keypair, created_at, kind, tags, content)
    }

    pub fn new_with_timestamp(
        keypair: KeyPair,
        created_at: i64,
        kind: NostrKind,
        tags: Vec<Tag>,
        content: String,
    ) -> Self {
        let (pubkey, _) = XOnlyPublicKey::from_keypair(&keypair);
        let json: Value = json!([0, pubkey, created_at, kind, tags, content]);
        let event_str: String = json.to_string();
        let id = EventId(Sha256Hash::hash(event_str.as_bytes()).to_string());
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

    pub fn new_git_event(
        keypair: KeyPair,
        git_event: GitEvent) -> Self {
        let tags: Vec<Tag> = git_event.to_tags();
        NostrEvent::new(keypair, NostrKind::Mega, tags, String::new())
    }
    pub fn new_git_event_with_timestamp(
        keypair: KeyPair,
        created_at: i64,
        git_event: GitEvent) -> Self {
        let tags: Vec<Tag> = git_event.to_tags();
        NostrEvent::new_with_timestamp(keypair, created_at, NostrKind::Mega, tags, String::new())
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
        let hash = Sha256Hash::from_str(self.id.inner().clone().as_str())?;
        let message = Message::from_slice(hash.as_ref())?;
        secp.verify_schnorr(&self.sig, &message, &self.pubkey)
            .map_err(|_| Error::InvalidSignature)
    }
}

pub fn sign_with_rng(id: String, keypair: &KeyPair) -> Signature {
    let secp = Secp256k1::new();
    let mut rng = rand::thread_rng();
    let sha256 = Sha256Hash::from_str(id.as_str()).unwrap();
    let message = Message::from_slice(sha256.as_ref()).unwrap();
    secp.sign_schnorr_with_rng(&message, keypair, &mut rng)
}

pub fn sign_without_rng(id: String, keypair: &KeyPair) -> Signature {
    let secp = Secp256k1::new();
    let sha256 = Sha256Hash::from_str(id.as_str()).unwrap();
    let message = Message::from_slice(sha256.as_ref()).unwrap();
    secp.sign_schnorr_no_aux_rand(&message, keypair)
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
        Self(Sha256Hash::hash(event_str.as_bytes()).to_string())
    }

    /// Get [`EventId`] as [`String`]
    pub fn inner(&self) -> String {
        self.0.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GitEvent {
    pub peer_id: String,
    pub repo_name: String,
    pub repo_target: String,
    pub repo_action: String,
    pub repo_url: String,
    pub repo_commit_id: String,
    pub repo_issue_content: String,
}

impl GitEvent {
    pub fn to_tags(&self) -> Vec<Tag> {
        let mut tags: Vec<Tag> = Vec::new();
        let tag = Tag::Generic(TagKind::PeerId, vec![self.peer_id.clone()]);
        tags.push(tag);
        let tag = Tag::Generic(TagKind::RepoName, vec![self.repo_name.clone()]);
        tags.push(tag);
        let tag = Tag::Generic(TagKind::RepoAction, vec![self.repo_action.clone()]);
        tags.push(tag);
        let tag = Tag::Generic(TagKind::RepoTarget, vec![self.repo_target.clone()]);
        tags.push(tag);
        let tag = Tag::Generic(TagKind::RepoUrl, vec![self.repo_url.clone()]);
        tags.push(tag);
        let tag = Tag::Generic(TagKind::RepoCommitId, vec![self.repo_commit_id.clone()]);
        tags.push(tag);
        if !self.repo_issue_content.is_empty() {
            let tag = Tag::Generic(TagKind::RepoIssueContent, vec![self.repo_issue_content.clone()]);
            tags.push(tag);
        }
        tags
    }

    pub fn from_tags(tags: Vec<Tag>) -> Self {
        let mut git_event = Self {
            peer_id: "".to_string(),
            repo_name: "".to_string(),
            repo_target: "".to_string(),
            repo_action: "".to_string(),
            repo_url: "".to_string(),
            repo_commit_id: "".to_string(),
            repo_issue_content: "".to_string(),
        };
        for x in tags {
            let vec = x.as_vec();
            if vec.len() > 1 {
                let kind = TagKind::from(vec[0].clone());
                let content = vec[1].clone();
                match kind {
                    TagKind::RepoName => { git_event.repo_name = content }
                    TagKind::RepoTarget => { git_event.repo_target = content }
                    TagKind::RepoAction => { git_event.repo_action = content }
                    TagKind::RepoUrl => { git_event.repo_url = content }
                    TagKind::RepoCommitId => { git_event.repo_commit_id = content }
                    TagKind::RepoIssueContent => { git_event.repo_issue_content = content }
                    _ => {}
                }
            }
        }
        git_event
    }
}


#[cfg(test)]
mod tests {
    use secp256k1::{KeyPair, Secp256k1, rand, SecretKey};
    use crate::nostr::tag::TagKind;
    use super::*;

    #[test]
    fn test_new_nostr_id() {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_str("6b911fd37cdf5c81d4c0adb1ab7fa822ed253ab0ad9aa18d77257c88b29b718e")
            .unwrap();
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);
        let (xonly, _) = XOnlyPublicKey::from_keypair(&key_pair);
        let tag = Tag::Generic(TagKind::P, Vec::new());
        let tags: Vec<Tag> = vec![tag];
        let event_id = EventId::new(xonly, 123, NostrKind::Mega, tags.clone(), "123".to_string());
        let event_id_right = "bc0d7c8a5c00eff8719c68f6df7f7e31b2a118ba5221807fe850ba689854f467";
        assert_eq!(event_id.inner(), event_id_right);
    }

    #[test]
    fn test_new_nostr_event() {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_str("6b911fd37cdf5c81d4c0adb1ab7fa822ed253ab0ad9aa18d77257c88b29b718e")
            .unwrap();
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);
        let tag = Tag::Generic(TagKind::P, Vec::new());
        let tags: Vec<Tag> = vec![tag];
        let content = "123".to_string();
        let event = NostrEvent::new_with_timestamp(key_pair, 1111, NostrKind::Mega, tags, content);
        assert_eq!(
            event.as_json(),
            r#"{"content":"123","created_at":1111,"id":"bc3cc3a1499ea0c618df6dc5a300e8ad05f4d3a9a96e40e8a237cec11cc78667","kind":111,"pubkey":"385c3a6ec0b9d57a4330dbd6284989be5bd00e41c535f9ca39b6ae7c521b81cd","sig":"1285a0697eb44bf41f7a1bb063646ac47de8f6664335b791fe4982b1c9cdae1f56d0f1dd0564aeb053fd3cab16c4a0508cef70ac0bec9d79db3f2a2c28426f20","tags":[["p"]]}"#
        );
    }

    #[test]
    fn test_verify_nostr_event() {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut rand::thread_rng());
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);
        let (xonly, _) = XOnlyPublicKey::from_keypair(&key_pair);

        let kind = NostrKind::Mega;
        let tag = Tag::Generic(TagKind::P, Vec::from(["123".into(), "234".into()]));
        let tags: Vec<Tag> = vec![tag];
        let event_id = EventId::new(xonly, 123, kind, tags.clone(), "123".to_string());

        let signature = sign_without_rng(event_id.inner(), &key_pair);
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
    fn test_sign_verify_with_rng() {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut rand::thread_rng());
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);
        let (xonly, _) = XOnlyPublicKey::from_keypair(&key_pair);

        let sha256 = Sha256Hash::hash("abc".as_bytes());
        let message = Message::from_slice(sha256.as_ref()).unwrap();

        //sign
        let rng = &mut rand::thread_rng();
        let signature = secp.sign_schnorr_with_rng(&message, &key_pair, rng);

        //verify
        let secp2 = Secp256k1::new();
        let result = secp2.verify_schnorr(&signature, &message, &xonly)
            .map_err(|_| Error::InvalidSignature);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sign_verify_with_libp2p() {
        let secp = Secp256k1::new();
        //libp2p Secp256k1
        let sk = libp2p::identity::secp256k1::SecretKey::generate();
        let secret_key = SecretKey::from_slice(&sk.to_bytes()).unwrap();
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);
        let (xonly, _) = XOnlyPublicKey::from_keypair(&key_pair);

        let sha256 = Sha256Hash::hash("abc".as_bytes());
        let message = Message::from_slice(sha256.as_ref()).unwrap();

        //sign
        let rng = &mut rand::thread_rng();
        let signature = secp.sign_schnorr_with_rng(&message, &key_pair, rng);

        //verify
        let secp2 = Secp256k1::new();
        let result = secp2.verify_schnorr(&signature, &message, &xonly)
            .map_err(|_| Error::InvalidSignature);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sha256() {
        let msg = "aaa";
        let hash = Sha256Hash::hash(msg.as_ref());
        let hash_string = hash.to_string();
        let hash2 = Sha256Hash::from_str(hash_string.as_str()).unwrap();
        assert!(hash.eq(&hash2));
    }

    #[test]
    fn test_nostr_event_json() {
        let sample_event = r#"{"content":"uRuvYr585B80L6rSJiHocw==?iv=oh6LVqdsYYol3JfFnXTbPA==","created_at":1640839235,"id":"2be17aa3031bdcb006f0fce80c146dea9c1c0268b0af2398bb673365c6444d45","kind":4,"pubkey":"f86c44a2de95d9149b51c6a29afeabba264c18e2fa7c49de93424a0c56947785","sig":"a5d9290ef9659083c490b303eb7ee41356d8778ff19f2f91776c8dc4443388a64ffcf336e61af4c25c05ac3ae952d1ced889ed655b67790891222aaa15b99fdd","tags":[["p","13adc511de7e1cfcf1c6b7f6365fb5a03442d7bcacf565ea57fa7770912c023d"]]}"#;
        let ev_ser = NostrEvent::from_json(sample_event).unwrap();
        assert_eq!(ev_ser.as_json(), sample_event);
    }

    #[test]
    fn test_nostr_event_git() {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_str("6b911fd37cdf5c81d4c0adb1ab7fa822ed253ab0ad9aa18d77257c88b29b718e")
            .unwrap();
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);

        let git_event = GitEvent {
            peer_id: "1".to_string(),
            repo_name: "2".to_string(),
            repo_target: "3".to_string(),
            repo_action: "4".to_string(),
            repo_url: "5".to_string(),
            repo_commit_id: "6".to_string(),
            repo_issue_content: "".to_string(),
        };
        let event = NostrEvent::new_git_event_with_timestamp(key_pair, 123, git_event);
        let sample_event = r#"{"content":"","created_at":123,"id":"66ed2655a12d95a62293c285571e6e27161ae2aef7e4457d67cc7ae90f921a90","kind":111,"pubkey":"385c3a6ec0b9d57a4330dbd6284989be5bd00e41c535f9ca39b6ae7c521b81cd","sig":"f97eba7bc292dcd036f8f15193cf3647bab16aceaf48dc290d940a4bb0c1a0501b3cf73242d9b76e6ddc6a3d7e2e14a5ea730e0e1b8a5328463d279c9ace324e","tags":[["peer_id","1"],["repo_name","2"],["repo_action","4"],["repo_target","3"],["repo_url","5"],["repo_commit_id","6"]]}"#;
        assert_eq!(event.as_json(), sample_event);
    }
}
