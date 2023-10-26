use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Topic {
    r#type: TopicType,
    #[serde(skip_serializing_if = "String::is_empty", rename = "id")]
    id: String,
}

impl Topic {
    pub fn new(r#type: TopicType, id: String) -> Self {
        Self { r#type, id }
    }
}

impl FromStr for Topic {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(':');

        let r#type = parts.next().ok_or(())?.parse()?;
        let id = parts.next().ok_or(())?.to_owned();

        Ok(Self { r#type, id })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopicType {
    /// Updates to a guild, e.g. name, icon, etc...
    /// Channel list, etc...
    Guild,
    /// Messages or typing, etc... in a channel
    Channel,
}

impl FromStr for TopicType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "guild" => Ok(Self::Guild),
            "channel" => Ok(Self::Channel),
            _ => Err(()),
        }
    }
}
