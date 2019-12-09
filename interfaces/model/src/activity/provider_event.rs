/*
 * Yagna Activity API
 *
 * It conforms with capability level 1 of the [Activity API specification](https://docs.google.com/document/d/1BXaN32ediXdBHljEApmznSfbuudTU8TmvOmHKl0gmQM).
 *
 * The version of the OpenAPI document: v1
 *
 * Generated by: https://openapi-generator.tech
 */

use serde::{Deserialize, Serialize};

pub mod gsb {
    use crate::activity::ExeScriptBatch;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "eventType")]
    pub enum GsbProviderEvent {
        #[serde(rename = "Exec")]
        Exec {
            #[serde(rename = "activityId")]
            activity_id: String,
            #[serde(rename = "batchId")]
            batch_id: String,
            #[serde(rename = "exeScript")]
            exe_script: ExeScriptBatch,
        },
        #[serde(rename = "GetRunningCommand")]
        GetRunningCommand {
            #[serde(rename = "activityId")]
            activity_id: String,
        },
        #[serde(rename = "GetState")]
        GetState {
            #[serde(rename = "activityId")]
            activity_id: String,
        },
        #[serde(rename = "GetUsage")]
        GetUsage {
            #[serde(rename = "activityId")]
            activity_id: String,
        },
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "eventType")]
pub enum ProviderEvent {
    #[serde(rename = "CreateActivity")]
    CreateActivity {
        #[serde(rename = "activityId")]
        activity_id: String,
        #[serde(rename = "agreementId")]
        agreement_id: String,
    },
    #[serde(rename = "DestroyActivity")]
    DestroyActivity {
        #[serde(rename = "activityId")]
        activity_id: String,
        #[serde(rename = "agreementId")]
        agreement_id: String,
    },
    #[serde(rename = "GetActivityState")]
    GetActivityState {
        #[serde(rename = "activityId")]
        activity_id: String,
    },
    #[serde(rename = "GetActivityUsage")]
    GetActivityUsage {
        #[serde(rename = "activityId")]
        activity_id: String,
    },
}