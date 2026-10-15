// Original weather cloud families: rain particlecloud_ew972295 and snow
// particlecloud_j32z635q. Source VS SHA256 170d88c0cb878eed1129695ac578c97ad2a8647b41aebc62c1edfe69972abe15
// /509b95c1ebde73b3681750e93b6ca19275cd5ab6b43a22161f5b1919655f7f60;
// shared PS db338a2a9dc8b087c0cfc6c9caed96798b8144f2083a7f8c624c4e251fd9148b.
import package::rendering::native_fx::NativeFxFrame;

struct NativeCloudObject {
    world_rows: array<vec4<f32>, 4>,
    outdoor_rows: array<vec4<f32>, 3>,
    view_right: vec4<f32>,
    view_up: vec4<f32>,
    cloud_matrix: vec4<f32>,
    velocity_world: vec4<f32>,
    color: vec4<f32>,
}

struct NativeCloudVaryings {
    clip_position: vec4<f32>,
    uv: vec2<f32>,
    outdoor: vec3<f32>,
}

fn cloud_corner(packed: vec4<f32>) -> vec2<f32> {
    // Original SNORM W shorts 0,8192,16384,24576 encode the source four corners.
    let corners = array<vec2<f32>, 4>(vec2<f32>(0.0,0.0), vec2<f32>(0.0,1.0),
        vec2<f32>(1.0,0.0), vec2<f32>(1.0,1.0));
    return corners[i32(packed.w * 4.0)];
}

fn cloud_world(position: vec4<f32>, object: NativeCloudObject) -> vec4<f32> {
    return vec4<f32>(dot(position, object.world_rows[0]), dot(position, object.world_rows[1]),
        dot(position, object.world_rows[2]), dot(position, object.world_rows[3]));
}

fn cloud_output(world: vec4<f32>, input: vec4<f32>, uv: vec2<f32>,
    frame: NativeFxFrame, object: NativeCloudObject) -> NativeCloudVaryings {
    var output: NativeCloudVaryings;
    output.clip_position = vec4<f32>(dot(world, frame.scene.view_projection_rows[0]),
        dot(world, frame.scene.view_projection_rows[1]), dot(world, frame.scene.view_projection_rows[2]),
        dot(world, frame.scene.view_projection_rows[3]));
    output.uv = uv;
    // Source lookup uses the cloud center, before billboard extrusion.
    output.outdoor = vec3<f32>(dot(input, object.outdoor_rows[0]),
        dot(input, object.outdoor_rows[1]), dot(input, object.outdoor_rows[2]));
    return output;
}

fn native_cloud_rain_vertex(packed: vec4<f32>, frame: NativeFxFrame,
    object: NativeCloudObject) -> NativeCloudVaryings {
    let uv = cloud_corner(packed);
    let corner = uv - vec2<f32>(0.5);
    let input = vec4<f32>(packed.xyz, 1.0);
    var world = cloud_world(input, object);
    // Preserve the native instruction swizzles. This source program constructs
    // its rain-width direction from eye-relative native XY and velocityWorld.
    let inverse_radius = inverseSqrt(dot(world.xy, world.xy));
    let side = vec3<f32>(-world.y * inverse_radius, 0.0, -world.x * inverse_radius);
    let velocity = object.velocity_world.xyz;
    let width_direction = fma(velocity.yzx, side.yzx, -(side * velocity.zxy));
    let width = corner.x * object.cloud_matrix.x * width_direction;
    let length = corner.y * object.cloud_matrix.y * velocity;
    world = vec4<f32>(world.xyz + width + length, world.w);
    return cloud_output(world, input, uv, frame, object);
}

fn native_cloud_snow_vertex(packed: vec4<f32>, frame: NativeFxFrame,
    object: NativeCloudObject) -> NativeCloudVaryings {
    let uv = cloud_corner(packed);
    let corner = uv - vec2<f32>(0.5);
    let input = vec4<f32>(packed.xyz, 1.0);
    var world = cloud_world(input, object);
    // VS102ffd67 offsets native view XY, then uses the original projection.
    // Mapping those same axes back to native world avoids a second camera ABI.
    let view_offset = corner.x * object.cloud_matrix.xy + corner.y * object.cloud_matrix.zw;
    world = vec4<f32>(world.xyz + object.view_right.xyz * view_offset.x
        + object.view_up.xyz * view_offset.y, world.w);
    return cloud_output(world, input, uv, frame, object);
}

fn native_cloud_fragment(input: NativeCloudVaryings, object: NativeCloudObject,
    color_map: texture_2d<f32>, color_sampler: sampler,
    outdoor_map: texture_2d<f32>, outdoor_sampler: sampler) -> vec4<f32> {
    let height = textureSample(outdoor_map, outdoor_sampler, input.outdoor.xy).w;
    let source_color = textureSample(color_map, color_sampler, input.uv) * object.color;
    return vec4<f32>(source_color.xyz * 0.5, select(source_color.w, 0.0, input.outdoor.z < height));
}
