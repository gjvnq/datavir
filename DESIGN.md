# DataVir Design Document

## Main concepts

**FileGraph** — A connected graph of FileNodes with a root FileNode.

**FileNode** — A metadata holder that: 1) points to a filestream, and 2) has a clear location within a volume.

**FileStream** — A long byte sequence with some metadata but no clear location within a volume. Each file stream belongs to exactly one storage pool.

**Metadata Pool** — A folder in which the file graph and all metadata of a volume is stored.

**Mounting** — An ordered set of volumes that is mounted on some mountpoint be it the local filesystem or an HTTP interface.

**Real FileStream** — A filestream that gets its content from the users. Basically a regular file.

**Storage Pool** — A space in which the filestream contents are stored. It may be a local folder or a remote service. It may implement automatic deduplication.

**Virtual FileStream** — A filestream that is automatically generated from other filestreams (e.g. a metadata striped version of another filestream)

**Volume** — An instance of a DataVir FileSystem. All volumes have exactly one metadata pool and at least one storage pool.

## FilePath syntax

Both filenames and metadata entry names are UTF-8 strings in the NFC (Normalization Form C, i.e. composition) that:

  * Don't begin with ```!```, ```@```, ```#```, ```$```, ```%```, ```&```, or ```:```.
  * Don't include the forward slash ```/``` (U+002F SOLIDUS) anywhere.
  * Don't include two consecutives dots ```..``` (U+002E FULL STOP * 2) anywhere.
  * Don't have trailing white spaces (including the U+1680 OGHAM SPACE MARK ``` ```) either in the begning or end.

Additionally filenames are limited to 4096 bytes including the final null.

## Access APIs

### Standard DataVir

A binary protocol to be implemented over TLS.

### FUSE


### Dokan

### HTTP/WebDAV