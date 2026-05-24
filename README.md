## Text in Nested Dictionaries and Lists - with Important Comments

+ [**MANIFESTO.md**](MANIFESTO.md) = an informal English description of the format.
+ [**ORIGINS.md**](ORIGINS.md) = the story of the spark that led to this project.

Significant progress has been made on the core Rust library, but there is one more change
to the format spec that I want to make before publishing: allow newlines within the
dictionary keys. To tide you over until that's ready, here's a little sneak peek...

# https://comments-are-important.github.io/tindalwic/

The UX is still a bit rough around the edges. Wish you could just paste JSON, TOML or YAML
and it could auto-detect the format, but for now you must select the format before pasting.

The "Neutered" mode is so named because it does damage by dropping all comments. Yes, that
is directly contrary to the tenets, but this mode is appropriate for converting from typical
JSON, TOML or YAML data.

The other modes preserve all comments as data within the JSON, TOML, or YAML: "Compact"
elides the nulls, while "Verbose" includes everything - excessive for these three formats,
but appropriate for the serde binary/columnar formats that eschew field names.

Suggested demo (okay for TOML, good for JSON, better for YAML):
 1. Make sure "Neutered" is active, copy a file you know, select the format,
    and paste it under the format selector...
    + the Tindalwic should look vaguely familiar, but also a little odd.
 2. Switch to "Compact" mode, Tindalwic stays same, but now...
    + there's a bunch of "meta" stuff in your file - same overall shape, but exploded.
 3. Switch to "Verbose" mode. Tindalwic still unchanged, but...
    + even more "meta" with lots of null values, showing where comments could go.
 4. Edit to replace some of those nulls with string values...
    + watch comments appear in the Tindalwic.
 5. Switch back to "Neutered", comments remain in the Tindalwic, but...
    + your original file is back.
