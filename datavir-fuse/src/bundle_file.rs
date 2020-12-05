use crate::inode_record::INodeRegisterable;
use crate::prelude::*;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
enum BundleFileKind {
    #[serde(rename = "F")]
    File = 1,
    #[serde(rename = "D")]
    Dir = 2,
    #[serde(rename = "L")]
    Link = 3,
}

#[derive(Debug, Serialize, Deserialize)]
struct BundleFile {
    file_uuid: Uuid,
    bundle_uuid: Uuid,
    modified: DateTime<Utc>,
    base_blob_uuid: Option<Uuid>,
    tree_hash: String,
    kind: BundleFileKind,
    unix_perm: i64,
    size: u64,
    path: String,
}

impl INodeRegisterable for BundleFile {
    fn get_uuid(&self) -> uuid::Uuid {
        self.bundle_uuid
    }
    fn get_obj_type(&self) -> ObjectType {
        ObjectType::BundleElement
    }
    fn get_name(&self) -> std::string::String {
        self.path.clone()
    }
}
