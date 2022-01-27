// This file is mostly for ideas and drafts

pub const MAX_STRING_SIZE : u64 = 4096; // in bytes

#[derive(Debug)]
pub struct StreamGeneration {
    id: Uuid,
    generator: String,
    used_streams: HashMap<Uuid, DateTime<Utc>>
}

#[derive(Debug)]
pub struct SimpleStreamGeneration {
    id: Uuid,
    generator: String,
    used_streams: HashMap<Uuid, DateTime<Utc>>
}

#[derive(Debug)]
pub enum StreamGenerationSegment {
    Data(Vec<u8>),
    StreamLink(Uuid, DateTime<Utc>, u64, u64)
}

#[derive(Debug)]
pub struct FileStream {
    id: Uuid,
    pool: Uuid,
    created: DateTime<Utc>,
    modified: DateTime<Utc>,
    size: u64, // for virtual filestreams, this is just an estimate
    generator: Option<Uuid>, // if none, then it is a real file
    flag_immutable: bool,
    compression: Option<CompressionAlg>
}

#[derive(Debug)]
pub enum CompressionAlg {
    Gzip,
    Xz,
}

impl FileStream {
    #[allow(dead_code)]
    fn is_real(&self) -> bool {
        self.generator.is_none()
    }

    #[allow(dead_code)]
    fn is_mutable(&self) -> bool {
        !self.flag_immutable && self.is_real()
    }
}

#[derive(Debug)]
pub enum FileContent {
    Nothing,
    ShortData(Vec<u8>), // only up to 2k
    Stream(Uuid, bool), // bool is for copy on write
    SymLink(String),
    // No idea how I'm going to implement the things below
    NamedPipe(Uuid),
    CharDevice(Uuid),
    BlockDevice(Uuid),
    Socket(Uuid)
}

#[derive(Debug)]
pub struct BasicFileNode {
    id: Uuid,
    content: FileContent,
    uperm: UnixPermission,
    created: DateTime<Utc>,
    modified: DateTime<Utc>,
}

#[derive(Debug)]
pub struct RichFileNode {
    id: Uuid,
    content: FileContent,
    uperm: UnixPermission,
    metadata: HashMap<String, FileContent>,
    created: DateTime<Utc>,
    modified: DateTime<Utc>,
}

#[derive(Debug)]
pub struct FileHandle {
    path: Vec<Uuid>,
    // The
    name: String,
    node: FileNode,
}

#[derive(Debug)]
pub struct UnixPermission {
    mask: u16,
    uid: u16,
    gid: u16,
    uname: String,
    gname: String
}

#[derive(Debug)]
pub struct MetadataEntry {
    entry_name: String,
    content: MetadataValue,
    created: DateTime<Utc>,
    modified: DateTime<Utc>,
}

#[derive(Debug)]
pub enum MetadataValue {
    Text(String),
    List(Vec<MetadataValue>),
    Set(HashSet<MetadataValue>),
    Map(HashMap<String, MetadataValue>),
    Binary(Vec<u8>),
    MagicValue(Uuid),
    Stream(Uuid),
    StreamSegment(Uuid, u64, u64)
}

#[derive(Debug)]
pub enum OSAccount {
    PosixAccount(i32, String)
}
