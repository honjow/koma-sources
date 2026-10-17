# Source template resource contract

Implement runtime SourceInfo and operations in src/lib.rs, then add exactly one
genuine local icon.png, icon.jpg, icon.jpeg, icon.gif, icon.webp or icon.ico. No
placeholder is provided. Missing/empty/duplicate/undecodable icons stop production
build.sh before compilation (1 MiB, 1,048,576 total pixels). The generated package
manifest.icon names that file; published indexes must reference the same bytes
and extension. SDK SourceInfo remains runtime metadata, not a file container.

Prefer suitable official originals. Do not retain extra backing plates, external
masks or shadows; authorized secondary cleanup must retain genuine identity and
provenance. Preserve the App's separate rounded display clipping. See
docs/WRITING_A_SOURCE.md before preparing a source.
