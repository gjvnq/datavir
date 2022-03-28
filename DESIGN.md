# DataVir Design Document

## Main Parts

  * Datavir Full Node: a program that implements all the functionality behind the datavir file system.
  * Datavir Dumb Node: a program that implements only the core features necessary for file syncing and usually has no way to decrypt the file contents.
  * Datavir Client: a program that connects to a datavir full node for using the files. This can also be an adapter for FUSE or other systems.

The communication between these parts is for now done via COSE (CBOR Object Signing and Encryption) transported via HTTP or HTTPS. In the future this may change to lightweight transport protocols like CoAP (Constrained Application Protocol).

## Main Ideas

  * A bundle is a small collection of file that should be treated as a single unit. This is mainly useful for things like sidecar files.
  * File and directories are the samething.
  * Some paths may point to virtual files that are produced on demand. Usually for things like compression, decompression and metadata removal.
  * A volume can be either real or pseudo. In the latter case it is possible to mix volumes just like a Linux filetree with mounts and mixing volumes in a single folder just like unionfs.