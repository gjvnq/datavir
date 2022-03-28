PRAGMA encoding = 'UTF-8';
PRAGMA foreign_keys = true;
PRAGMA defer_foreign_keys = true;

BEGIN;

CREATE TABLE `app_config` (
	`key` NOT NULL,
	`value`
);

INSERT INTO `app_config` (`key`, `value`) VALUES ('schema_version', 1);
INSERT INTO `app_config` (`key`, `value`) VALUES ('root_uuid', 'd0e79d76-720c-4c8f-8986-02bb52e2d4e0');

CREATE TABLE `filenode` (
	`inode_num` INTEGER PRIMARY KEY,
	`node_uuid` NOT NULL UNIQUE,
	`parent_uuid` NOT NULL,
	`filename` NOT NULL,
	`contents` NULL,
	`super_hidden` DEFAULT 0,
	`changed_at` NOT NULL,
	`created_at` NOT NULL
);

INSERT INTO `filenode` (`inode_num`, `node_uuid`, `parent_uuid`, `filename`, `contents`, `super_hidden`, `changed_at`, `created_at`) VALUES (1, 'd0e79d76-720c-4c8f-8986-02bb52e2d4e0', 'd0e79d76-720c-4c8f-8986-02bb52e2d4e0', '', NULL, 0, strftime('%s', 'now'), strftime('%s', 'now'));

CREATE TABLE `crtd` (
	`crtd_uuid` NOT NULL UNIQUE,
	`operation` NOT NULL, # e.g. move, delete, write, ...
	`data` NOT NULL, # the JSON data of the CRTD
	`requested_at` NOT NULL
);

-- What if I use OrbitDB for everything?

COMMIT;