# egui_spine

[Spine](http://esotericsoftware.com/) rendering support for egui!

## Api

The `Spine` struct handles all rendering and interactions with the Spine
runtime. For the moment, not all spine features are exposed, and only
`eframe` with the `wgpu` renderer was tested (`eframe` + `glow` is
currently not supported).

## Examples

You can find an example using eframe [here](https://github.com/UserIsntAvailable/egui_spine/blob/main/examples/eframe.rs);
Your mileage might vary with others egui integrations; as long as `wgpu`
is the gpu renderer, _everything should work_.
