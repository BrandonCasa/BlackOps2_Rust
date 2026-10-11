import bevy_pbr::render::mesh_view_types;

fn height_primitive(height: f32) -> f32 {
    if height < 0.0 {
        return exp(max(height, -80.0));
    }
    return height + 1.0;
}

fn apply(fog: mesh_view_types::Fog, input_color: vec4<f32>, fragment: vec3<f32>, camera: vec3<f32>) -> vec4<f32> {
    let ray = fragment - camera;
    let distance = length(ray);
    let start = max(fog.be.x, 0.0);
    if distance <= 0.0001 {
        return input_color;
    }
    let extinction = 1.0 / max(fog.be.y, 0.0254);
    let density_scale = clamp(1.0 / 0.0254, extinction, extinction * 100.0);
    let height_decay = 0.69314718 / max(fog.bi.x, 0.0254);
    let direction = ray / distance;
    let height = log(extinction / density_scale) + (fog.be.z - camera.y) * height_decay;
    let delta = -ray.y * height_decay;
    let primitive = height_primitive(height);
    var density = clamp(primitive, 0.0, 1.0);
    if abs(delta) >= 0.0001 {
        density = (height_primitive(height + delta) - primitive) / delta;
    }
    let optical_depth = max(distance * density * density_scale * 0.69314718 - start * extinction, 0.0);
    let pitch = radians(fog.bi.y);
    let yaw = radians(fog.bi.z);
    let to_sun = vec3<f32>(cos(pitch) * cos(yaw), -sin(pitch), -cos(pitch) * sin(yaw));
    let outer = radians(-fog.directional_light_exponent - 1.0);
    let sun_weight = clamp((dot(direction, to_sun) - cos(outer)) / max(1.0 - cos(outer), 0.000001), 0.0, 1.0);
    let color = mix(fog.base_color, fog.directional_light_color, sun_weight);
    let opacity = (1.0 - exp2(-optical_depth)) * color.a;
    return vec4<f32>(mix(input_color.rgb, color.rgb, opacity), input_color.a);
}
