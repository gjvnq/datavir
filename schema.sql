CREATE TABLE `bundles` (
	`bundle_uuid` CHAR(36),
	`conflicts_from` CHAR(36),
	`sync_status` INT,
	`modified` TIMESTAMP,
	`name` VARCHAR(250)
);

CREATE INDEX `bundles_sync_status_idx` ON `bundles` (`sync_status`);

CREATE TABLE `files` (
	`file_uuid` CHAR(36),
	`bundle_uuid` CHAR(36),
	`modified` TIMESTAMP,
	`base_blob_uuid` CHAR(36),
	`tree_hash` VARCHAR(200),
	`kind` CHAR(1), /* File, Directory, symbolic Link */
	`unix_perm` INTEGER,
	`size` INTEGER,
	`path` VARCHAR(250)
);

CREATE INDEX `files_bundle_uuid_idx` ON `files` (`bundle_uuid`);

CREATE TABLE `blobs` (
	`blob_uuid` CHAR(36),
	`size` INTEGER,
	`status` INTEGER
);

CREATE INDEX `blobs_blob_uuid_idx` ON `blobs` (`blob_uuid`);
CREATE INDEX `blobs_status_idx` ON `blobs` (`status`);

CREATE TABLE `blocks` (
	`blob_uuid` CHAR(36),
	`file_uuid` CHAR(36),
	`block_num` INTEGER
);

CREATE INDEX `blocks_file_uuid_idx` ON `blocks` (`file_uuid`);
CREATE INDEX `blocks_blob_uuid_idx` ON `blocks` (`blob_uuid`);

CREATE TABLE `basic_metadata` (
	`subject_uuid` CHAR(36),
	`predicate` VARCHAR(250),
	`value` TEXT
);

CREATE INDEX `basic_metadata_subject_uuid_idx` ON `basic_metadata` (`subject_uuid`);
CREATE INDEX `basic_metadata_predicate_idx` ON `basic_metadata` (`predicate`);

CREATE TABLE `inode` (
	`inode_num` INTEGER PRIMARY KEY, /* the value is actually u64 but may be preented as i64 */
	`object_uuid` CHAR(36),
	`object_type` VARCHAR(20),
	`path` VARCHAR(250)
) WITHOUT ROWID;

CREATE INDEX `inode_inode_num_idx` ON `inode` (`inode_num`);
CREATE INDEX `inode_object_uuid_idx` ON `inode` (`object_uuid`);

CREATE TABLE `app_config` (
	`key` TEXT,
	`value` TEXT
);
