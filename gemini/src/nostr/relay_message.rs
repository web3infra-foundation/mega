use crate::nostr::client_message::SubscriptionId;
use crate::nostr::event::{EventId, NostrEvent};
use crate::nostr::MessageHandleError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value};

/// Messages sent by relays, received by clients
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayMessage {
    /// `["EVENT", <subscription_id>, <event JSON>]` (NIP01)
    Event {
        /// Subscription ID
        subscription_id: SubscriptionId,
        /// Event
        event: NostrEvent,
    },
    /// `["OK", <event_id>, <true|false>, <message>]` (NIP01)
    Ok {
        /// Event ID
        event_id: EventId,
        /// Status
        status: bool,
        /// Message
        message: String,
    },
    /// `["EOSE", <subscription_id>]` (NIP01)
    EndOfStoredEvents(SubscriptionId),
    /// ["NOTICE", \<message\>] (NIP01)
    Notice {
        /// Message
        message: String,
    },
}

impl Serialize for RelayMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let json_value: Value = self.as_value();
        json_value.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RelayMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let json_value = Value::deserialize(deserializer)?;
        RelayMessage::from_value(json_value).map_err(serde::de::Error::custom)
    }
}

impl RelayMessage {
    /// Create new `EVENT` message
    pub fn new_event(subscription_id: SubscriptionId, event: NostrEvent) -> Self {
        Self::Event {
            subscription_id,
            event,
        }
    }

    pub fn new_event_box(subscription_id: SubscriptionId, event: NostrEvent) -> Self {
        Self::Event {
            subscription_id,
            event,
        }
    }

    /// Create new `NOTICE` message
    pub fn new_notice<S>(message: S) -> Self
    where
        S: Into<String>,
    {
        Self::Notice {
            message: message.into(),
        }
    }

    /// Create new `EOSE` message
    pub fn new_eose(subscription_id: SubscriptionId) -> Self {
        Self::EndOfStoredEvents(subscription_id)
    }

    /// Create new `OK` message
    pub fn new_ok<S>(event_id: EventId, status: bool, message: S) -> Self
    where
        S: Into<String>,
    {
        Self::Ok {
            event_id,
            status,
            message: message.into(),
        }
    }

    fn as_value(&self) -> Value {
        match self {
            Self::Event {
                event,
                subscription_id,
            } => json!(["EVENT", subscription_id, event]),
            Self::Notice { message } => json!(["NOTICE", message]),
            Self::EndOfStoredEvents(subscription_id) => {
                json!(["EOSE", subscription_id])
            }
            Self::Ok {
                event_id,
                status,
                message,
            } => json!(["OK", event_id, status, message]),
        }
    }

    pub fn from_value(msg: Value) -> Result<Self, MessageHandleError> {
        let v = msg
            .as_array()
            .ok_or(MessageHandleError::InvalidMessageFormat)?;

        if v.is_empty() {
            return Err(MessageHandleError::InvalidMessageFormat);
        }

        let v_len: usize = v.len();

        // Notice
        // Relay response format: ["NOTICE", <message>]
        if v[0] == "NOTICE" {
            if v_len != 2 {
                return Err(MessageHandleError::InvalidMessageFormat);
            }
            return Ok(Self::Notice {
                message: serde_json::from_value(v[1].clone())?,
            });
        }

        // Event
        // Relay response format: ["EVENT", <subscription id>, <event JSON>]
        if v[0] == "EVENT" {
            if v_len != 3 {
                return Err(MessageHandleError::InvalidMessageFormat);
            }

            return Ok(Self::Event {
                subscription_id: serde_json::from_value(v[1].clone())?,
                event: NostrEvent::from_value(v[2].clone())?,
            });
        }

        // EOSE (NIP-15)
        // Relay response format: ["EOSE", <subscription_id>]
        if v[0] == "EOSE" {
            if v_len != 2 {
                return Err(MessageHandleError::InvalidMessageFormat);
            }
            let subscription_id: String = serde_json::from_value(v[1].clone())?;
            return Ok(Self::EndOfStoredEvents(SubscriptionId::new(
                subscription_id,
            )));
        }

        // OK (NIP-20)
        // Relay response format: ["OK", <event_id>, <true|false>, <message>]
        if v[0] == "OK" {
            if v_len != 4 {
                return Err(MessageHandleError::InvalidMessageFormat);
            }

            return Ok(Self::Ok {
                event_id: serde_json::from_value(v[1].clone())?,
                status: serde_json::from_value(v[2].clone())?,
                message: serde_json::from_value(v[3].clone())?,
            });
        }

        Err(MessageHandleError::InvalidMessageFormat)
    }

    pub fn from_json<T>(json: T) -> Result<Self, MessageHandleError>
    where
        T: AsRef<[u8]>,
    {
        let msg: &[u8] = json.as_ref();

        if msg.is_empty() {
            return Err(MessageHandleError::EmptyMsg);
        }

        let value: Value = serde_json::from_slice(msg)?;
        Self::from_value(value)
    }

    pub fn as_json(&self) -> String {
        json!(self).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nostr::kind::NostrKind;
    use crate::nostr::tag::{Tag, TagKind};
    use secp256k1::{Keypair, Secp256k1, SecretKey};
    use std::str::FromStr;

    #[test]
    fn test_handle_valid_notice() {
        let valid_notice_msg = r#"["NOTICE","Invalid event format!"]"#;
        let handled_valid_notice_msg =
            RelayMessage::new_notice(String::from("Invalid event format!"));

        assert_eq!(
            RelayMessage::from_json(valid_notice_msg).unwrap(),
            handled_valid_notice_msg
        );
    }

    #[test]
    fn test_handle_valid_event() {
        let valid_event_msg = r#"["EVENT", "random_string", {"content":"123","created_at":1111,"id":"bc3cc3a1499ea0c618df6dc5a300e8ad05f4d3a9a96e40e8a237cec11cc78667","kind":111,"pubkey":"385c3a6ec0b9d57a4330dbd6284989be5bd00e41c535f9ca39b6ae7c521b81cd","sig":"1285a0697eb44bf41f7a1bb063646ac47de8f6664335b791fe4982b1c9cdae1f56d0f1dd0564aeb053fd3cab16c4a0508cef70ac0bec9d79db3f2a2c28426f20","tags":[["p"]]}]"#;

        let secp = Secp256k1::new();
        let secret_key =
            SecretKey::from_str("6b911fd37cdf5c81d4c0adb1ab7fa822ed253ab0ad9aa18d77257c88b29b718e")
                .unwrap();
        let key_pair = Keypair::from_secret_key(&secp, &secret_key);
        let tag = Tag::Generic(TagKind::P, Vec::new());
        let tags: Vec<Tag> = vec![tag];
        let content = "123".to_string();
        let handled_event =
            NostrEvent::new_with_timestamp(key_pair, 1111, NostrKind::Mega, tags, content);

        assert_eq!(
            RelayMessage::from_json(valid_event_msg).unwrap(),
            RelayMessage::new_event(SubscriptionId::new("random_string"), handled_event)
        );
    }
}
