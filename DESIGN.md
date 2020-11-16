# DataVir Design Document

## Core concepts:

### Summary

**Bundle**: a data storage unit that combine content and metadata. They can be either a single file or a directory containing multiple files or directories.

**Filter**: a logical expression that filters bundles based on metadata (mostly tags). They act a bit like virtual folders. Filters can be "saved" in volumes or in the user's preferences.

**Volume**: a collection of bundles, a bit like a partition on a disk.

**Volume Set**: a set of volumes. This is mostly a "syntatic sugar" to help with with usability.

**User**: a human or non-human account that is used for authentication and storage of permissions.

### Bundle

A bundle is an association between metadata and "actual" content (i.e. files and dirs).

All bundles MUST have at least two metadata fields populated: UUID and name (also called title).

The metadata is in RDF so it extensible. An example of a bundle with common metadata is listed below:

```turtle
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix dc: <http://purl.org/dc/terms> .
@prefix dv: <http://github.com/gjvnq/datavir> .
 
<urn:uuid:62080c79-3f6d-4339-92b8-03c6c3bb5c89>
  dc:title "DataVir Design Document";
  dc:abstract "Described the design behind the DataVir system.";
  dc:created "2020-11";
  dc:modified "2020-11-15";
  dc:format "LaTeX";
  dv:tags "incomplete", "needs revision";
  .
```

## Mounting as a file system

When mounting as a file system (e.g. via FUSE) the following structure is used:

  * The root directory contains a "Filters" directory and one directory for each multi-file bundle and one file per single-file bundle.
  * The "Filters" directory contains a directory for each filter and they contain symbolic links to the bundles.

Example:

```
┌ D "Dir where the FS is mounted"
├┬ D "Filters"
│├┬ D "Has PDF"
││├─ L "62080c79-3f6d-4339-92b8-03c6c3bb5c89 - DataVir Design Document" -> "../../../62080c79-3f6d-4339-92b8-03c6c3bb5c89 - DataVir Design Document"
││└─ L "0520084f-fc6f-4815-af89-6d67da78df53 - Manual.pdf" -> "../../../0520084f-fc6f-4815-af89-6d67da78df53 - Manual.pdf"
│├┬ D "LaTeX documents"
│╵└─ L "62080c79-3f6d-4339-92b8-03c6c3bb5c89 - DataVir Design Document" -> "../../../62080c79-3f6d-4339-92b8-03c6c3bb5c89 - DataVir Design Document"
├┬ D "62080c79-3f6d-4339-92b8-03c6c3bb5c89 - DataVir Design Document"
│├─ F ".metadata.datavir.rdf"
│├─ F "Design.tex"
│├─ F "Design.pdf"
│└─ F "Makefile"
├┬ F "0520084f-fc6f-4815-af89-6d67da78df53 - Manual.pdf"
╵└─ F ".metadata.datavir.rdf"
```

The bundle file or directory has its name decided by:

  1. The `dv:fsName` if it is available and is unique in the volume set in use.
  2. The `dc:title` and the UUID in the following style: `{UUID} - {dc:title}`.

Each bundle file of directory also has the following extended attributes:

  * `fs:uuid`: the bundle UUID.
  * `fs:volume-uuid`: The volume UUID that stores the bundle.
  * `fs:metadata-file`: The name of the file that stores the bundle metadata, usually it is `.metadata.datavir.rdf` but it can be changed by the user.
  * `fs:cannonical-name`: The bundle name in the style `{UUID} - {dc:title}`.
  * `fs:ads`: `true` if the bundle is single-file (aka if it is a file with "subfiles").

Notice that single file bundles have a "hidden subfile". This is an approximation of the Macintosh's resource forks (or NTFS's alternate data stream) concept to the UNIX ways.
