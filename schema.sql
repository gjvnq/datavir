CREATE TABLE `bundles` (
	`bundle_uuid` CHAR(36),
	`conflicts_from` CHAR(36)
	`sync_status` INT,
	`name` VARCHAR(250),
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
	`path` MEDIUMTEXT
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