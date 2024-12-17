![Map of London](https://raw.githubusercontent.com/OCzarnecki/pretty-map/master/thumbnail.png)

# Rendering a pretty-ish map of London into PNG

This repository contains all the code I used in working on my map project. It is more for documentation
and reference, so not polished to be usable.

If you do want to use it, you will need to

1. Downloaded raw OSM data from https://extract.bbbike.org/
2. Adjust the config file in `config/london_full.json`, or create your own based on it and specify its
   path in `rust_rewrite/src/main.rs`.
3. CWD into `rust_rewrite` and `cargo run --release`. Then, run `python -m tile_combiner.py` to assemble
   the generated tiles into an image.

# Licence

The source code in this repository ("the source code") is released under the [GNU AGPLv3](https://www.gnu.org/licenses/agpl-3.0.en.html#license-text) and images as well as other digital artifacts created with the source code are covered by the [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/) license. If you have a use in mind which is not permitted by these licenses, get in touch!

The map is generated based on Open Street Map data, which is available under the [Open Database License](https://www.openstreetmap.org/copyright). Copyright OpenStreetMap Contributors.
