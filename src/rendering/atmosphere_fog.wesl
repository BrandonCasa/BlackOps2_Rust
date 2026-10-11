import bevy_pbr::render::mesh_view_types;

fn apply(fog: mesh_view_types::Fog, input_color: vec4<f32>, fragment: vec3<f32>, camera: vec3<f32>) -> vec4<f32> {
    let ray = fragment - camera;
    let distance = length(ray);
    let start = max(fog.be.x, 0.0);
    let segment = max(distance - start, 0.0);
    if segment <= 0.0 {
        return input_color;
    }
    let half_distance = max(fog.be.y, 0.0001);
    let direction = ray / max(distance, 0.0001);
    var density = 1.0;
    if fog.bi.x >= 0.0254 {
        let height_start = camera.y + direction.y * min(start, distance);
        let height_change = direction.y * segment;
        let exponent = clamp((fog.be.z - height_start) / fog.bi.x, -80.0, 80.0);
        let delta = clamp(-height_change / fog.bi.x, -80.0, 80.0);
        var integral = 1.0;
        if abs(delta) >= 0.0001 {
            integral = (exp2(delta) - 1.0) / (delta * 0.69314718);
        }
        density = exp2(exponent) * integral;
    }
    let opacity = min(1.0 - exp2(-segment * density / half_distance), fog.base_color.a);
    let pitch = radians(fog.bi.y);
    let yaw = radians(fog.bi.z);
    let to_sun = vec3<f32>(cos(pitch) * cos(yaw), -sin(pitch), -cos(pitch) * sin(yaw));
    let outer = radians(-fog.directional_light_exponent - 1.0);
    let sun_weight = smoothstep(min(cos(outer), 0.999999), 1.0, dot(direction, to_sun)) * fog.directional_light_color.a;
    let color = mix(fog.base_color.rgb, fog.directional_light_color.rgb, sun_weight);
    return vec4<f32>(mix(input_color.rgb, color, opacity), input_color.a);
}
