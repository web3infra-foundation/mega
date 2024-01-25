use secp256k1::hashes::sha256::Hash as Sha256Hash;
use secp256k1::hashes::Hash;
use secp256k1::rand::rngs::OsRng;
use secp256k1::rand::RngCore;
use secp256k1::XOnlyPublicKey;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::nostr::event::{EventId, NostrEvent};
use crate::nostr::kind::NostrKind;
use crate::nostr::tag::TagKind;
use crate::nostr::MessageHandleError;
use serde::ser::SerializeMap;
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientMessage {
    /// Event
    Event(Box<NostrEvent>),
    /// Req
    Req {
        /// Subscription ID
        subscription_id: SubscriptionId,
        /// Filters
        filters: Vec<Filter>,
    },
}

impl Serialize for ClientMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let json_value: Value = self.as_value();
        json_value.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ClientMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let json_value = Value::deserialize(deserializer)?;
        ClientMessage::from_value(json_value).map_err(serde::de::Error::custom)
    }
}

impl ClientMessage {
    /// Create new `EVENT` message
    pub fn new_event(event: NostrEvent) -> Self {
        Self::Event(Box::new(event))
    }

    /// Create new `REQ` message
    pub fn new_req(subscription_id: SubscriptionId, filters: Vec<Filter>) -> Self {
        Self::Req {
            subscription_id,
            filters,
        }
    }

    /// Check if is an `EVENT` message
    pub fn is_event(&self) -> bool {
        matches!(self, ClientMessage::Event(_))
    }

    /// Check if is an `REQ` message
    pub fn is_req(&self) -> bool {
        matches!(self, ClientMessage::Req { .. })
    }

    /// Serialize as [`Value`]
    pub fn as_value(&self) -> Value {
        match self {
            Self::Event(event) => json!(["EVENT", event]),
            Self::Req {
                subscription_id,
                filters,
            } => {
                let mut json = json!(["REQ", subscription_id]);
                let mut filters = json!(filters);

                if let Some(json) = json.as_array_mut() {
                    if let Some(filters) = filters.as_array_mut() {
                        json.append(filters);
                    }
                }

                json
            }
        }
    }

    /// Deserialize from [`Value`]
    ///
    /// **This method NOT verify the event signature!**
    pub fn from_value(msg: Value) -> Result<Self, MessageHandleError> {
        let v = msg
            .as_array()
            .ok_or(MessageHandleError::InvalidMessageFormat)?;

        if v.is_empty() {
            return Err(MessageHandleError::InvalidMessageFormat);
        }

        let v_len: usize = v.len();

        // Event
        // ["EVENT", <event JSON>]
        if v[0] == "EVENT" {
            if v_len != 2 {
                return Err(MessageHandleError::InvalidMessageFormat);
            }
            let event = NostrEvent::from_value(v[1].clone())?;
            return Ok(Self::new_event(event));
        }

        // Req
        // ["REQ", <subscription_id>, <filter JSON>, <filter JSON>...]
        if v[0] == "REQ" {
            if v_len == 2 {
                let subscription_id: SubscriptionId = serde_json::from_value(v[1].clone())?;
                return Ok(Self::new_req(subscription_id, Vec::new()));
            } else if v_len >= 3 {
                let subscription_id: SubscriptionId = serde_json::from_value(v[1].clone())?;
                let filters: Vec<Filter> = serde_json::from_value(Value::Array(v[2..].to_vec()))?;
                return Ok(Self::new_req(subscription_id, filters));
            } else {
                return Err(MessageHandleError::InvalidMessageFormat);
            }
        }

        Err(MessageHandleError::InvalidMessageFormat)
    }

    pub fn as_json(&self) -> String {
        json!(self).to_string()
    }
}

/// Subscription ID
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubscriptionId(String);

impl SubscriptionId {
    /// Create new [`SubscriptionId`]
    pub fn new<S>(id: S) -> Self
    where
        S: Into<String>,
    {
        Self(id.into())
    }

    /// Generate new random [`SubscriptionId`]
    pub fn generate() -> Self {
        let mut rng = OsRng;
        Self::generate_with_rng(&mut rng)
    }
    /// Generate new random [`SubscriptionId`]
    pub fn generate_with_rng<R>(rng: &mut R) -> Self
    where
        R: RngCore,
    {
        let mut os_random = [0u8; 32];
        rng.fill_bytes(&mut os_random);
        let hash = Sha256Hash::hash(&os_random).to_string();
        Self::new(&hash[..32])
    }
}

impl fmt::Display for SubscriptionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for SubscriptionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SubscriptionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let id: String = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        Ok(Self::new(id))
    }
}

/// Subscription filters
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Filter {
    /// List of [`EventId`]
    #[serde(skip_serializing_if = "HashSet::is_empty")]
    #[serde(default)]
    pub ids: HashSet<EventId>,
    /// List of [`XOnlyPublicKey`]
    #[serde(skip_serializing_if = "HashSet::is_empty")]
    #[serde(default)]
    pub authors: HashSet<XOnlyPublicKey>,
    /// List of a kind numbers
    #[serde(skip_serializing_if = "HashSet::is_empty")]
    #[serde(default)]
    pub kinds: HashSet<NostrKind>,
    /// It's a string describing a query in a human-readable form, i.e. "best nostr apps"
    ///
    /// <https://github.com/nostr-protocol/nips/blob/master/50.md>
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub search: Option<String>,
    /// An integer unix timestamp, events must be newer than this to pass
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub since: Option<u64>,
    /// An integer unix timestamp, events must be older than this to pass
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub until: Option<u64>,
    /// Maximum number of events to be returned in the initial query
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub limit: Option<usize>,
    /// Generic tag queries (NIP12)
    #[serde(
        flatten,
        serialize_with = "serialize_generic_tags",
        deserialize_with = "deserialize_generic_tags"
    )]
    #[serde(default)]
    pub generic_tags: HashMap<String, HashSet<String>>,
}

impl Filter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn author(mut self, author: XOnlyPublicKey) -> Self {
        self.authors.insert(author);
        self
    }
    pub fn kind(mut self, kind: NostrKind) -> Self {
        self.kinds.insert(kind);
        self
    }

    pub fn pubkey(self, pubkey: XOnlyPublicKey) -> Self {
        self.custom_tag(TagKind::P.to_string(), vec![pubkey.to_string()])
    }

    pub fn get_pubkey(self) -> HashSet<String> {
        if let Some(pks) = self.generic_tags.get("#p") {
            return pks.clone();
        }
        HashSet::new()
    }

    pub fn peer_id(self, peer_id: String) -> Self {
        self.custom_tag(String::from("P"), vec![peer_id])
    }

    pub fn repo_name(self, repo_name: String) -> Self {
        self.custom_tag(TagKind::RepoName.into(), vec![repo_name])
    }

    pub fn custom_tag<S>(mut self, tag: String, values: Vec<S>) -> Self
    where
        S: Into<String>,
    {
        let values: HashSet<String> = values.into_iter().map(|value| value.into()).collect();
        self.generic_tags
            .entry(tag)
            .and_modify(|list| {
                for value in values.clone().into_iter() {
                    list.insert(value);
                }
            })
            .or_insert(values);
        self
    }
}

fn serialize_generic_tags<S>(
    generic_tags: &HashMap<String, HashSet<String>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(generic_tags.len()))?;
    for (tag, values) in generic_tags.iter() {
        map.serialize_entry(&format!("#{tag}"), values)?;
    }
    map.end()
}

fn deserialize_generic_tags<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, HashSet<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct GenericTagsVisitor;

    impl<'de> Visitor<'de> for GenericTagsVisitor {
        type Value = HashMap<String, HashSet<String>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("map in which the keys are \"#X\" for some character X")
        }

        fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut generic_tags = HashMap::new();
            while let Some(key) = map.next_key::<String>()? {
                let mut chars = key.chars().clone();
                if let (Some('#'), Some(ch), None) = (chars.next(), chars.next(), chars.next()) {
                    let tag: String = String::from(ch);
                    let values = map.next_value()?;
                    generic_tags.insert(tag, values);
                } else {
                    let (_a, b) = key.split_at(1);
                    let values = map.next_value()?;
                    generic_tags.insert(b.to_string(), values);
                }
            }
            Ok(generic_tags)
        }
    }

    deserializer.deserialize_map(GenericTagsVisitor)
}

#[cfg(test)]
mod tests {
    use core::str::FromStr;

    use super::*;

    #[test]
    fn test_client_message_req() {
        let pk = XOnlyPublicKey::from_str(
            "379e863e8357163b5bce5d2688dc4f1dcc2d505222fb8d74db600f30535dfdfe",
        )
        .unwrap();
        let filters = vec![
            Filter::new().kind(NostrKind::Mega),
            Filter::new().pubkey(pk),
        ];

        let client_req = ClientMessage::new_req(SubscriptionId::new("test"), filters);
        assert_eq!(
            client_req.as_json(),
            r##"["REQ","test",{"kinds":[111]},{"#p":["379e863e8357163b5bce5d2688dc4f1dcc2d505222fb8d74db600f30535dfdfe"]}]"##
        );
    }

    #[test]
    fn test_client_message_from_value() {
        let req = json!(["REQ","8aff61479fb8e406a0c29dee13981db6",{"#P":["16Uiu2HAmJMy5xuyCnsKcGESmEar15Ca7c5abG8TadqrqfeHub2tS"]}]);

        let msg = ClientMessage::from_value(req.clone()).unwrap();

        assert_eq!(msg.as_value(), req);
    }
}
