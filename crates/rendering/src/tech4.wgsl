// Project-owned implementation of the documented TECH4 mathematical contract.
// Synthetic/qualified float inputs only; source packing/globals are not inferred.
#import bevy_pbr::{mesh_functions, mesh_view_bindings::view,
    view_transformations::position_world_to_clip}

struct Tech4Globals {
    sh0: vec4<f32>,
    sh1: vec4<f32>,
    sh2: vec4<f32>,
    hdr_scale: f32,
}
struct Tech4Fog {
    color: vec4<f32>, constants: vec4<f32>, constants2: vec4<f32>,
    sun_direction: vec4<f32>, sun_color: vec4<f32>, sun_parameters: vec4<f32>,
    enabled: f32,
}
struct CapturedClip { view_projection: mat4x4<f32>, camera_source: vec3<f32>, enabled: f32, viewport_depth: vec2<f32>, }
@group(#{MATERIAL_BIND_GROUP}) @binding(12) var<uniform> captured_clip: CapturedClip;
@group(#{MATERIAL_BIND_GROUP}) @binding(11) var<uniform> fog: Tech4Fog;
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> globals: Tech4Globals;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var base_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var base_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var normal_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var normal_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var specular_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var specular_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(7) var lightmap_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(8) var lightmap_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(9) var probe_texture: texture_cube<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(10) var probe_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) lightmap_uv: vec2<f32>,
    @location(4) tangent: vec4<f32>,
    @location(5) color: vec4<f32>,
    @location(6) fog: vec4<f32>,
}
struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) camera_relative: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec3<f32>,
    @location(3) bitangent: vec3<f32>,
    @location(4) color: vec4<f32>,
    @location(5) uv: vec2<f32>,
    @location(6) lightmap_uv: vec2<f32>,
    @location(7) inverse_sh: vec3<f32>,
    @location(8) fog: vec4<f32>,
}
// Return canonical Bevy vectors to source basis for the measured SH/cube math.
fn source_vector(v: vec3<f32>) -> vec3<f32> { return vec3(v.x, -v.z, v.y); }

@vertex
fn vertex(input: Vertex) -> Varyings {
    var out: Varyings;
    let world_from_local = mesh_functions::get_world_from_local(input.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(input.position,1.0));
    let n = source_vector(mesh_functions::mesh_normal_local_to_world(input.normal,input.instance_index));
    let t = normalize(source_vector((world_from_local * vec4(input.tangent.xyz,0.0)).xyz));
    out.position = position_world_to_clip(world_position.xyz);
    out.camera_relative = source_vector(world_position.xyz-view.world_position);
    if captured_clip.enabled>0.5 {
        out.camera_relative=source_vector(world_position.xyz)-captured_clip.camera_source;
        var original_clip=captured_clip.view_projection*vec4(out.camera_relative,1.0);
        original_clip.z=captured_clip.viewport_depth.x*original_clip.w+(captured_clip.viewport_depth.y-captured_clip.viewport_depth.x)*original_clip.z;
        out.position=original_clip;
    }
    out.normal = n;
    out.tangent = t;
    out.bitangent = -sign(input.tangent.w)*cross(n,t);
    out.color=input.color;
    out.uv=input.uv;
    out.lightmap_uv=input.lightmap_uv;
    let k=dot(globals.sh2,vec4(n.z*n.x,n.y*n.z,n.x*n.y,n.x*n.x-n.y*n.y))
        +dot(globals.sh1,vec4(n,1.0))+globals.sh0.w*n.z*n.z;
    out.inverse_sh = vec3(1.0)/max(globals.sh0.xyz*k,vec3(0.1));
    out.fog=input.fog;
    if fog.enabled>0.5 {
        let p=out.camera_relative;
        let height=fog.constants.w*p.z;
        let profile=height+fog.constants.x;
        var integrated=profile+1.0;
        if profile<0.0 {integrated=exp2(min(profile,64.0)*1.442695);}
        var density=clamp(fog.constants2.x,0.0,1.0);
        if abs(height)>=0.0001 {density=(integrated-fog.constants2.x)/height;}
        let transmittance=min(exp2(density*fog.constants.y*length(p)+fog.constants.z),1.0);
        let sun=clamp(dot(fog.sun_direction.xyz,normalize(p))*fog.sun_parameters.y+fog.sun_parameters.x,0.0,1.0);
        let color=mix(fog.color,fog.sun_color,sun);
        out.fog=vec4(color.xyz,1.0-(1.0-transmittance)*color.w);
    }
    return out;
}
fn decode_radiance(v: vec4<f32>) -> vec3<f32> { return v.xyz/(v.w+0.000001); }

@fragment
fn fragment(input: Varyings) -> @location(0) vec4<f32> {
    let sampled_normal=textureSample(normal_texture,normal_sampler,input.uv).xy;
    let xy=4.015748*sampled_normal-vec2(2.015748);
    let mapped=normalize(input.normal+xy.x*input.tangent+xy.y*input.bitangent);
    let sampled_specular=textureSample(specular_texture,specular_sampler,input.uv);
    let gloss=sampled_specular.w;
    let view_direction=-normalize(input.camera_relative);
    let e=exp2(-9.28*clamp(dot(mapped,view_direction),0.0,1.0));
    let f=1.041667*gloss*min(e,0.475*gloss)+0.018229*gloss-0.015625;
    let modulation=clamp(sampled_specular.xyz*sampled_specular.xyz*(0.75+0.25*gloss-f)+vec3(f),vec3(0.0),vec3(1.0));
    let reflected=reflect(normalize(input.camera_relative),mapped);
    let probe=decode_radiance(textureSampleLevel(probe_texture,probe_sampler,reflected,4.0*(1.0-gloss)));
    let uv=input.lightmap_uv;
    let ambient=decode_radiance(textureSample(lightmap_texture,lightmap_sampler,vec2(uv.x,uv.y/3.0)));
    let directional=decode_radiance(textureSample(lightmap_texture,lightmap_sampler,vec2(uv.x,(uv.y+1.0)/3.0)));
    let direction=textureSample(lightmap_texture,lightmap_sampler,vec2(uv.x,(uv.y+2.0)/3.0)).xyz*2.0-vec3(1.0);
    let mapped_irradiance=ambient+directional*clamp(dot(direction,mapped),0.0,1.0);
    let geometric_irradiance=ambient+directional*clamp(dot(direction,input.normal),0.0,1.0);
    let base=textureSample(base_texture,base_sampler,input.uv).xyz*input.color.xyz;
    let lit=base*base*mapped_irradiance+probe*modulation*geometric_irradiance*input.inverse_sh;
    let fogged=mix(input.fog.xyz,lit,input.fog.w);
    return vec4(sqrt(fogged*globals.hdr_scale),1.0);
}
