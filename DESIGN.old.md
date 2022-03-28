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

**Generator** — A “script-like” thing that automatically populates file streams and file nodes. Mainly used for filtering by metadata and for transparernt decompression.

**Bundle** — A small collection of files that behaves as a "single thing". For example: a 3D model with sidecar files for texturing.

## FilePath syntax

Both filenames and metadata entry names are UTF-8 strings in the NFC (Normalization Form C, i.e. composition) that:

  * Don't start nor end with ```!```, ```@```, ```#```, ```$```, ```%```, ```&```, ```:``` or ```~```.
  * Don't start with ```._```.
  * Don't include the forward slash ```/``` (U+002F SOLIDUS) anywhere.
  * Don't start two consecutives dots ```..``` (U+002E FULL STOP * 2) anywhere.
  * Don't have trailing white spaces (including the U+1680 OGHAM SPACE MARK ``` ```) either in the start or end.

Additionally filenames are limited to 4096 bytes including the final NULL.

Filenames begning with ```@``` usually refer to a generator about the directory itself. So ```folder/@folder.tar.gz``` is a compressed archive of ```folder``` in the ```.tar.gz``` format.

Filenames begining with ```:``` are maped to extended attributes. So ```file.pdf/:source``` maps to the extended attribute ```user.source```.

## Extended Attributes

Filenode metadata and filestream metadata map to extended attributes begining in ```user.``` and ```stream.``` respectively.

## Sync

There are two variants of peers: full peers and dumb peers. The former can access encrypted data while the latter cannot for they lack the encryption keys. Both variants can sync with any peer.

## Storage Pools



## Access APIs

### Standard DataVir

A binary protocol to be implemented over TLS.

### FUSE

### Dokan

### HTTP/WebDAV