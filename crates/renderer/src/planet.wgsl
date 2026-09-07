struct Camera { mvp: mat4x4<f32>, model: mat4x4<f32>, selection: vec4<f32> }
@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var terrain: texture_2d<f32>;
@group(0) @binding(2) var pixel_sampler: sampler;
struct Out { @builtin(position) position: vec4<f32>, @location(0) normal: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) selected: f32 }
@vertex fn vertex_main(@location(0) position:vec3<f32>, @location(1) normal:vec3<f32>, @location(2) uv:vec2<f32>)->Out {
    var o:Out;
    o.position=camera.mvp*vec4<f32>(position,1.0);
    o.normal=(camera.model*vec4<f32>(normal,0.0)).xyz;
    o.uv=uv;
    o.selected=select(0.0,1.0,camera.selection.w>0.0 && dot(normalize(normal),camera.selection.xyz)>0.99999);
    return o;
}
@fragment fn fragment_main(i:Out)->@location(0) vec4<f32> {
    let light=max(dot(normalize(i.normal),normalize(vec3<f32>(-0.5,0.8,1.0))),0.0);
    let tint=vec3<f32>(0.63,0.67,0.83)*0.66+vec3<f32>(1.0,0.89,0.69)*light*0.52;
    let color=textureSample(terrain,pixel_sampler,i.uv).rgb*tint;
    return vec4<f32>(mix(color,vec3<f32>(0.89,0.53,0.16),i.selected*0.55),1.0);
}
