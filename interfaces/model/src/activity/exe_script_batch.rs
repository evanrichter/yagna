/*
 * Yagna Activity API
 *
 * It conforms with capability level 1 of the [Activity API specification](https://docs.google.com/document/d/1BXaN32ediXdBHljEApmznSfbuudTU8TmvOmHKl0gmQM).
 *
 * The version of the OpenAPI document: v1
 *
 * Generated by: https://openapi-generator.tech
 */

use crate::activity::ExeScriptCommand;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExeScriptBatch {
    #[serde(rename = "commands")]
    pub commands: Vec<ExeScriptCommand>,
}

impl ExeScriptBatch {
    pub fn new(commands: Vec<ExeScriptCommand>) -> ExeScriptBatch {
        ExeScriptBatch { commands }
    }
}