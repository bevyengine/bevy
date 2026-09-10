//! Module with trimmed down `OpenRPC` document structs.
//! It tries to follow this standard: <https://spec.open-rpc.org>
use bevy_platform::collections::HashMap;
use bevy_utils::default;
use serde::{Deserialize, Serialize};

use crate::RemoteMethods;

use super::json_schema::JsonSchemaBevyType;

/// Represents an `OpenRPC` document as defined by the `OpenRPC` specification.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenRpcDocument {
    /// The version of the `OpenRPC` specification being used.
    pub openrpc: String,
    /// Informational metadata about the document.
    pub info: InfoObject,
    /// List of RPC methods defined in the document.
    pub methods: Vec<MethodObject>,
    /// Optional list of server objects that provide the API endpoint details.
    pub servers: Option<Vec<ServerObject>>,
}

/// Contains metadata information about the `OpenRPC` document.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InfoObject {
    /// The title of the API or document.
    pub title: String,
    /// The version of the API.
    pub version: String,
    /// An optional description providing additional details about the API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// A collection of custom extension fields.
    #[serde(flatten)]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl Default for InfoObject {
    fn default() -> Self {
        Self {
            title: "Bevy Remote Protocol".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            description: None,
            extensions: Default::default(),
        }
    }
}

/// Describes a server hosting the API as specified in the `OpenRPC` document.
#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServerObject {
    /// The name of the server.
    pub name: String,
    /// The URL endpoint of the server.
    pub url: String,
    /// An optional description of the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Additional custom extension fields.
    #[serde(flatten)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Represents an RPC method in the `OpenRPC` document.
#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct MethodObject {
    #[expect(
        clippy::doc_markdown,
        reason = "In this case, we are referring to a string, so using quotes instead of backticks makes sense."
    )]
    /// The method name (e.g., "world.get_components")
    pub name: String,
    /// An optional short summary of the method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// An optional detailed description of the method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Parameters for the RPC method
    #[serde(default)]
    pub params: Vec<Parameter>,
    // /// The expected result of the method
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub result: Option<Parameter>,
    /// Additional custom extension fields.
    #[serde(flatten)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Represents an RPC method parameter in the `OpenRPC` document.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    /// Parameter name
    pub name: String,
    /// Parameter description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON schema describing the parameter
    pub schema: JsonSchemaBevyType,
    /// Additional custom extension fields.
    #[serde(flatten)]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl From<&RemoteMethods> for Vec<MethodObject> {
    fn from(value: &RemoteMethods) -> Self {
        value
            .methods()
            .iter()
            .map(|e| MethodObject {
                name: e.to_owned(),
                ..default()
            })
            .collect()
    }
}
