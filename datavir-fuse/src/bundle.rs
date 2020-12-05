use crate::inode_record::INodeRegisterable;
use crate::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
enum BundleSyncStatus {
    Unavailable = 0,
    Downloading = 1,
    Uploading = 2,
    // Downloading + Uploading
    Syncing = 3,
    Ready = 4,
}

#[derive(Debug, Serialize, Deserialize)]
struct Bundle {
    bundle_uuid: Uuid,
    conflicts_from: Option<Uuid>,
    sync_status: BundleSyncStatus,
    name: String,
}

impl INodeRegisterable for Bundle {
    fn get_uuid(&self) -> uuid::Uuid {
        self.bundle_uuid
    }
    fn get_obj_type(&self) -> ObjectType {
        ObjectType::BundleRoot
    }
    fn get_name(&self) -> std::string::String {
        "".to_string()
    }
}
