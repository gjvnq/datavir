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

All bundles are identified by an UUID and must have at least one of the following:
  * ```rdfs:label```
  * ```dc:title```
  * ```dv:name```

The metadata is in RDF to be extensible. An example of a bundle with common metadata is listed below:

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

When mounting as a file system (e.g. via FUSE) the mount point will contain:

  1. 🌐 `DataVir.yaml`: a file with general metadata about what is mounted and what language is in use.
  1. 🌐`Bundles.txt`: a text file for creating and deleting symbolic links to the bundles.
  1. 🌐`Filters`: a directory that stores the filters.
  1. 🌐`All bundles`: a direcory containing all bundles.
  1. 🌐`Bundles`: user defined links to bundles according to the "bundles.txt" file.
  1. `.datavir.socket`: a socket for advanced communication with the filesystem.
  1. `.filters`: a symbolic link to the "Filters" directory.
  1. `.all-bundles`: a symbolic link to the "All Bundles" directory.
  1. `.user-bundles`: a symbolic link to the "Bundles" directory.
  1. `.bundles.txt`: a symbolic link to the "bundles.txt" file.
  1. `.datavir.yaml`: a symbolic link to the "DataVir.txt" file.

Paths preceded by 🌐 may be changed according to the user's language.

The "All Bundles" directory contains a single direcory per bundle following the naming convention: `{name}` if unique, `{UUID} - {name}` otherwise. The name field is obtain from, in order:

  1. The `dv:name`.
  2. The `rdfs:label`.
  3. The `dc:title`.

If no name is available, only the UUID will be used.

Note that the fields are first checked in the user's language and then in any language. So a bundle with `dv:name@en="Some bundle"` and `dc:title@pt="Algum maço"` will show as
"Algum maço" for Portuguese speaking users and as "Some bundle" for everybody else.

Notice that the bundles can be accessed by name if it begins with the corret UUID. Thus: `my bundle`, `d20e819c-b65d-44c7-8d9c-2eeb489b601f`, `d20e819c-b65d-44c7-8d9c-2eeb489b601f - my bundle` and
`d20e819c-b65d-44c7-8d9c-2eeb489b601f - some bundle` act like hardlinks even tough only a single one appear on the directory listing.

The reason for this strange behaviour and not enforcing bundle name uniqueness is to allow multiple volumes to be easily combined. (**TODO:** check if this is actually a good idea, may be it is better to unify only the filters)

The bundle direcories have a hidden file called `.datavir.bundle.rdf` which holds the metadata for the bundle.

If the bundle has a single file, it will appear as a file but still contain the the `.datavir.bundle.rdf` as if the bundle were a directory.
This a similar threatment to [HFS+ resource forks on Linux](https://static.lwn.net/2000/0817/a/lt-fork.php3).

The "Filters" directory contains a directory for each filter. Each of those filter directories contains:

  1. 🌐`filter.sparql`: a file describing the filter.
  2. `.filter.sparql`: a symbolic link to the `filter.sparql` file.
  3. A symbolic link to each bundle that matches the filter.

Finally, the "Bundles" direcory contains a user defined structure of symbolic links to the actual bundles.

Example: (`H` means hard link)

```
┌ D "~/mnt"
├─ F "DataVir.yaml"
├─ F ".datavir.socket"
├─ F "bundles.txt"
├┬ D "Filters"
│├┬ D "Has PDF"
││├─ F "filter.sparql"
││├─ L "62080c79-3f6d-4339-92b8-03c6c3bb5c89 - DataVir Design Document" -> "../../../All Bundles/62080c79-3f6d-4339-92b8-03c6c3bb5c89 - DataVir Design Document"
││└─ L "0520084f-fc6f-4815-af89-6d67da78df53 - Manual.pdf" -> "../../../All Bundles/0520084f-fc6f-4815-af89-6d67da78df53 - Manual.pdf"
│├┬ D "LaTeX documents"
││├─ F "filter.sparql"
│╵└─ L "62080c79-3f6d-4339-92b8-03c6c3bb5c89 - DataVir Design Document" -> "../../../All Bundles/62080c79-3f6d-4339-92b8-03c6c3bb5c89 - DataVir Design Document"
├┬ D "All Bundles"
│├┬ D "62080c79-3f6d-4339-92b8-03c6c3bb5c89 - DataVir Design Document"
││├─ F ".datavir.bundle.rdf"
││├─ F "Design.tex"
││├─ F "Design.pdf"
││└─ F "Makefile"
│├┬ F "0520084f-fc6f-4815-af89-6d67da78df53 - Manual.pdf"
│╵└─ F ".datavir.bundle.rdf"
├┬ D "Bundles"
│├─ H "Manual.pdf" -> "../All Bundles/0520084f-fc6f-4815-af89-6d67da78df53  - Manual.pdf"
│├┬ D "Docs"
│╵└─ H "Design" -> "../../All Bundles/62080c79-3f6d-4339-92b8-03c6c3bb5c89 - DataVir Design Document"
```

Each bundle file or directory has the following extended attributes:

  * `fs.uuid`: the bundle UUID.
  * `fs.volume-uuid`: The volume UUID that stores the bundle.
  * `fs.metadata-file`: The name of the file that stores the bundle metadata, usually it is `.metadata.datavir.rdf` but it can be changed by the user.
  * `fs.cannonical-name`: The bundle name in the style `{UUID} - {dc:title}`.
  * `fs.bundle-root`: `true`. (this helps with showing the correct icons for the user)
  * `fs.dir-like`: `true`. (because there will always be a metadata subfile)

### bundles.txt

This file is used to create and delete symbolic links to the actual bundles. It is basically a shortcut definitions file.

The syntax is pretty simple: UUID followed by a space followed by the desired path. Notice that folders will be created and deleted automatically.

```
0520084f-fc6f-4815-af89-6d67da78df53 Manual.pdf
62080c79-3f6d-4339-92b8-03c6c3bb5c89 Docs/Design
```

### DataVir.yaml

### Behaviour

  * Renaming a bundle node (within "All Bundles" and "Bundles") automatically adjusts the metadata.
  * Deleting a bundle shortcut won't delete the actual bundle.
  * It is not possible to add nodes within the "Bundles" direcory, unless the new node is actually within an existing bundle.
  * Adding a new file or directory within "All Bundles" will automatically create a new bundle with the correct metadata.
  * If invalid syntax is used in the metadata files, an error will be produced when saving or closing the file. (probably `EACCES`)
