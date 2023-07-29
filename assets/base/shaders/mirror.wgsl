#import bevy_pbr::mesh_vertex_output MeshVertexOutput

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

@fragment
fn fragment(
    in: MeshVertexOutput,
) -> @location(0) vec4<f32> {
    return textureSample(texture, texture_sampler, vec2<f32>(1.0 - in.uv.x, in.uv.y));
}
