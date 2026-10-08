import bevy_solari::scene::bindings::directional_lights;
import origins::ray_surface::{trace_surface, resolve_surface, surface_double_sided};
import bevy_pbr::render::utils::sample_cube_dir;
@if(ORIGINS_DDGI) import origins::gi_sampling::{sample_gi_field, gi_read, gi_oct_direction};

enable wgpu_ray_query;
// This software contains source code provided by NVIDIA Corporation.
// WGSL adaptation of RTXGI-DDGI f33e496: fixed-ray classification,
// bounded relocation, directional blending, and perceptual accumulation.
struct Constants { center_spacing: vec4<f32>, grid_generation: vec4<u32>, batch_lamps: vec4<u32>, reflection_position: vec4<f32>, lamp_grid_origin: vec4<f32>, lamp_grid_extent: vec4<u32> }
struct Lamp { position_range: vec4<f32>, radiance_radius: vec4<f32> }
@group(1) @binding(0) var<uniform> constants: Constants;
@if(ORIGINS_DDGI) @group(1) @binding(1) var previous: texture_3d<f32>;
@if(ORIGINS_DDGI) @group(1) @binding(2) var<storage, read_write> updates: array<vec4<f32>>;
@if(ORIGINS_DDGI) @group(1) @binding(6) var<storage> probes: array<vec4<i32>>;
@group(1) @binding(3) var sky_sampler: sampler;
@group(1) @binding(4) var sky: texture_cube<f32>;
@group(1) @binding(5) var<storage> lamps: array<Lamp>;
@group(1) @binding(7) var reflection_output: texture_storage_2d_array<rgba16float,write>;
@group(1) @binding(8) var<storage> lamp_grid: array<u32>;
@if(ORIGINS_DDGI) var<workgroup> radiances: array<vec3<f32>, 64>;
@if(ORIGINS_DDGI) var<workgroup> distances: array<f32, 96>;
@if(ORIGINS_DDGI) var<workgroup> directions: array<vec3<f32>, 96>;
@if(ORIGINS_DDGI) var<workgroup> probe_active: u32;
@if(ORIGINS_DDGI) var<workgroup> relocated: vec3<f32>;
@if(ORIGINS_DDGI) var<workgroup> moved: u32;

fn lamp_range(position: vec3<f32>) -> vec2<u32> {
    let cell = vec3<i32>(floor((position-constants.lamp_grid_origin.xyz)/constants.lamp_grid_origin.w));
    let extent = constants.lamp_grid_extent.xyz;
    if any(extent == vec3(0u)) || any(cell < vec3(0i)) || any(cell >= vec3<i32>(extent)) {
        return vec2(0xffffffffu,constants.batch_lamps.y);
    }
    let index = u32(cell.x)+extent.x*(u32(cell.y)+extent.y*u32(cell.z));
    return vec2(lamp_grid[index*2u],lamp_grid[index*2u+1u]);
}
fn lamp_id(range: vec2<u32>, ordinal: u32) -> u32 {
    if range.x == 0xffffffffu { return ordinal; }
    return lamp_grid[range.x+ordinal];
}

@if(ORIGINS_DDGI) fn sphere_direction(index: u32, count: u32) -> vec3<f32> {
    let y = 1.0 - 2.0 * (f32(index) + 0.5) / f32(count);
    let radius = sqrt(max(0.0, 1.0 - y * y));
    let phi = f32(index) * 2.39996323;
    return vec3(cos(phi) * radius, y, sin(phi) * radius);
}
@if(ORIGINS_DDGI) fn gi_random(seed: u32) -> f32 {
    var v = seed;
    v ^= v >> 16u; v *= 0x7feb352du;
    v ^= v >> 15u; v *= 0x846ca68bu;
    v ^= v >> 16u;
    return f32(v >> 8u) / 16777216.0;
}
@if(ORIGINS_DDGI) fn rotated_direction(direction: vec3<f32>, seed: u32) -> vec3<f32> {
    let u = gi_random(seed);
    let a = gi_random(seed ^ 0x9e3779b9u) * 6.28318531;
    let b = gi_random(seed ^ 0x85ebca6bu) * 6.28318531;
    let q = vec4(sqrt(1.0 - u) * sin(a), sqrt(1.0 - u) * cos(a), sqrt(u) * sin(b), sqrt(u) * cos(b));
    return direction + 2.0 * cross(q.xyz, cross(q.xyz, direction) + q.w * direction);
}
fn visible(origin: vec3<f32>, direction: vec3<f32>, distance: f32) -> bool {
    return trace_surface(origin, direction, 0.01, distance, 0.0).kind == RAY_QUERY_INTERSECTION_NONE;
}
@if(ORIGINS_DDGI) fn direct_irradiance(origin: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    // Incident E/pi stays linear RGB: never replace the source's color with
    // luminance or a neutral bounce tint.
    var incident = vec3(0.0);
    for (var light_index = 0u; light_index < arrayLength(&directional_lights); light_index++) {
        let light = directional_lights[light_index];
        let cosine = max(0.0, dot(normal, light.direction_to_light));
        if cosine > 0.0 && visible(origin, light.direction_to_light, 2000.0) {
            incident += light.luminance * light.inverse_pdf * (cosine / 3.14159265);
        }
    }
    let local_lamps = lamp_range(origin);
    for (var light_index = 0u; light_index < local_lamps.y; light_index++) {
        let light = lamps[lamp_id(local_lamps, light_index)];
        let delta = light.position_range.xyz - origin;
        let length_squared = dot(delta, delta);
        if length_squared >= light.position_range.w * light.position_range.w { continue; }
        let light_distance = sqrt(max(length_squared, 0.0001));
        let light_direction = delta / light_distance;
        let cosine = max(0.0, dot(normal, light_direction));
        if cosine > 0.0 && visible(origin, light_direction, max(0.01, light_distance - light.radiance_radius.w)) {
            let ratio = light_distance / light.position_range.w;
            let falloff = max(0.0, 1.0 - ratio * ratio * ratio * ratio);
            incident += light.radiance_radius.rgb * cosine * falloff * falloff
                / (3.14159265 * max(length_squared, light.radiance_radius.w * light.radiance_radius.w));
        }
    }
    return incident;
}
@if(ORIGINS_DDGI) fn diffuse_direction(normal: vec3<f32>, geometric_normal: vec3<f32>, seed: u32) -> vec3<f32> {
    let helper = select(vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0), abs(normal.y) > 0.9);
    let tangent = normalize(cross(helper, normal));
    let bitangent = cross(normal, tangent);
    let radius_squared = gi_random(seed);
    let phi = gi_random(seed ^ 0x9e3779b9u) * 6.28318531;
    let disk = sqrt(radius_squared) * vec2(cos(phi), sin(phi));
    let direction = tangent * disk.x + bitangent * disk.y + normal * sqrt(1.0 - radius_squared);
    // Fold extreme mapped-normal directions back above the actual triangle.
    // The geometric offset and this hemisphere guard retain thin-wall blocking.
    return normalize(direction - 2.0 * min(dot(direction, geometric_normal), 0.0) * geometric_normal);
}
@if(ORIGINS_DDGI) fn distance_moments(direction: vec3<f32>) -> vec2<f32> {
    var sum = vec2(0.0); var weights = 0.0;
    for (var ray = 32u; ray < 96u; ray++) {
        let cosine = max(0.0, dot(direction, directions[ray]));
        let weight = pow(cosine, 50.0);
        let d = min(abs(distances[ray]), 2.59807621);
        sum += vec2(d, d * d) * weight;
        weights += weight;
    }
    return sum / max(weights, 0.000000064);
}
@if(ORIGINS_DDGI) @compute @workgroup_size(64)
fn main(@builtin(workgroup_id) group: vec3<u32>, @builtin(local_invocation_index) lane: u32) {
    let probe = probes[group.x];
    let spacing = constants.center_spacing.w;
    let state = gi_read(previous, probe.xyz, 192u);
    let retained = all(state.xyz == vec3<f32>(probe.xyz));
    let old_offset = gi_read(previous, probe.xyz, 193u).xyz;
    let offset = select(vec3(0.0), old_offset, retained);
    let position = (vec3<f32>(probe.xyz) + 0.5 + offset) * spacing;
    let max_distance = spacing * 64.0;
    if lane < 32u {
        let direction = sphere_direction(lane, 32u);
        let ray = trace_surface(position, direction, 0.01, max_distance, 0.0625);
        var distance = 64.0;
        if ray.kind != RAY_QUERY_INTERSECTION_NONE {
            distance = ray.t / spacing;
            if !surface_double_sided(ray) && !ray.front_face { distance *= -0.2; }
        }
        distances[lane] = distance;
        directions[lane] = direction;
    }
    workgroupBarrier();
    if lane == 0u {
        var backfaces = 0u; var nearest_back = 64.0;
        var nearest_front = 64.0; var farthest_front = 0.0;
        var back_index = 0u; var front_index = 0u; var far_index = 0u;
        var nearby = false;
        for (var ray = 0u; ray < 32u; ray++) {
            let d = distances[ray];
            if d < 0.0 {
                backfaces++;
                if -d * 5.0 < nearest_back { nearest_back = -d * 5.0; back_index = ray; }
            } else {
                if d < nearest_front { nearest_front = d; front_index = ray; }
                if d > farthest_front { farthest_front = d; far_index = ray; }
                let axis = abs(directions[ray]);
                nearby = nearby || d <= 1.0 / max(axis.x, max(axis.y, axis.z));
            }
        }
        let inside = backfaces > 8u;
        var candidate = offset;
        let min_front = min(0.2 / spacing, 0.1);
        if inside {
            candidate += directions[back_index] * (nearest_back + min_front * 0.5);
        } else if nearest_front < min_front {
            if dot(directions[front_index], directions[far_index]) <= 0.0 {
                candidate += directions[far_index] * min(farthest_front, min_front);
            }
        } else {
            let margin = min(nearest_front - min_front, length(offset));
            candidate -= offset / max(length(offset), 0.000001) * margin;
        }
        relocated = offset;
        if dot(candidate, candidate) < 0.2025 { relocated = candidate; }
        moved = u32(length(relocated - offset) > 0.0001);
        probe_active = u32(!inside && moved == 0u && (nearby || (probe.w & 4) != 0));
    }
    workgroupBarrier();
    let seed = constants.grid_generation.w
        ^ (u32(probe.x) * 73856093u) ^ (u32(probe.y) * 19349663u) ^ (u32(probe.z) * 83492791u);
    let direction = rotated_direction(sphere_direction(lane, 64u), seed);
    var radiance = vec3(0.0); var distance = 64.0;
    if probe_active != 0u {
        var path_origin = position;
        var path_direction = direction;
        var throughput = vec3(1.0);
        // Four actual diffuse surface bounces carry light around corners in a
        // single update. Only the last hit reads the persistent field, adding
        // the remaining recursive tail once rather than double-counting it.
        for (var bounce = 0u; bounce < 4u; bounce++) {
            let ray = trace_surface(path_origin, path_direction, 0.01, max_distance, 0.0625);
            if ray.kind == RAY_QUERY_INTERSECTION_NONE {
                radiance += throughput * textureSampleLevel(sky, sky_sampler, path_direction * vec3(1.0, 1.0, -1.0), 0.0).rgb;
                break;
            }
            if bounce == 0u { distance = ray.t / spacing; }
            let surface = resolve_surface(ray, ray.t * 0.0625);
            let hit = surface.resolved;
            let geometry_normal = hit.geometric_world_normal;
            if !surface.double_sided && !ray.front_face {
                if bounce == 0u { distance *= -0.2; }
                break;
            }
            let origin = hit.world_position + geometry_normal * 0.02;
            radiance += throughput * hit.material.emissive;
            // Cosine-weighted continuation cancels the Lambertian cosine/pi
            // against its PDF. Retain each channel's material reflectance.
            let reflectance = clamp(hit.material.base_color, vec3(0.0), vec3(1.0))
                * (1.0 - hit.material.metallic) * (1.0 - 0.04 * surface.coating) * surface.occlusion;
            throughput *= reflectance;
            radiance += throughput * direct_irradiance(origin, hit.world_normal);
            if bounce == 3u {
                let unit_position = (origin - constants.center_spacing.xyz) / (vec3<f32>(constants.grid_generation.xyz) * spacing);
                if all(abs(unit_position) <= vec3(0.5)) {
                    radiance += throughput * sample_gi_field(previous, unit_position, hit.world_normal, spacing, constants.center_spacing.xyz).irradiance;
                }
                break;
            }
            if all(throughput == vec3(0.0)) { break; }
            path_origin = origin;
            path_direction = diffuse_direction(hit.world_normal, geometry_normal, seed ^ (lane * 0x85ebca6bu) ^ ((bounce + 1u) * 0xc2b2ae35u));
        }
    }
    radiances[lane] = radiance;
    directions[lane + 32u] = direction;
    distances[lane + 32u] = distance;
    workgroupBarrier();
    let history = retained && state.w > 0.0 && (probe.w & 1) == 0 && moved == 0u;
    for (var index = lane; index < 208u; index += 64u) {
        var result = gi_read(previous, probe.xyz, index);
        if index < 64u && probe_active != 0u {
            let texel_direction = gi_oct_direction((vec2(f32(index % 8u), f32(index / 8u)) + 0.5) / 8.0 * 2.0 - 1.0);
            var sum = vec3(0.0); var weights = 0.0;
            for (var ray = 0u; ray < 64u; ray++) {
                if distances[ray + 32u] < 0.0 { continue; }
                let weight = max(0.0, dot(texel_direction, directions[ray + 32u]));
                sum += radiances[ray] * weight; weights += weight;
            }
            let encoded = pow(max(sum / (2.0 * max(weights, 0.000000064)), vec3(0.0)), vec3(0.2));
            var hysteresis = select(0.0, 0.75, history);
            if max(result.x - encoded.x, max(result.y - encoded.y, result.z - encoded.z)) > 0.25 {
                hysteresis = max(0.0, hysteresis - 0.75);
            }
            var delta = encoded - result.rgb;
            if history && dot(delta, vec3(0.2126, 0.7152, 0.0722)) > 1.0 { delta *= 0.25; }
            var step = (1.0 - hysteresis) * delta;
            if history && max(encoded.x, max(encoded.y, encoded.z)) < max(result.x, max(result.y, result.z)) {
                step = min(max(vec3(0.0009765625), abs(step)), abs(delta)) * sign(step);
            }
            result = vec4(select(encoded, result.rgb + step, history), 1.0);
        } else if index >= 64u && index < 192u && probe_active != 0u {
            let first = (index - 64u) * 2u;
            let a = gi_oct_direction((vec2(f32(first % 16u), f32(first / 16u)) + 0.5) / 16.0 * 2.0 - 1.0);
            let second = first + 1u;
            let b = gi_oct_direction((vec2(f32(second % 16u), f32(second / 16u)) + 0.5) / 16.0 * 2.0 - 1.0);
            let moments = vec4(distance_moments(a), distance_moments(b));
            // Admit newly nearer occluders immediately. Keeping a bright-room
            // distance for ten updates leaks while doors/actors block a probe.
            let a_history = select(0.0, 0.9, history && moments.x >= result.x);
            let b_history = select(0.0, 0.9, history && moments.z >= result.z);
            result = vec4(mix(moments.xy, result.xy, a_history), mix(moments.zw, result.zw, b_history));
        } else if index == 192u { result = vec4(vec3<f32>(probe.xyz), f32(probe_active)); }
        else if index == 193u { result = vec4(relocated, f32(moved)); }
        updates[group.x * 208u + index] = result;
    }
}

fn local_irradiance(position: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    @if(ORIGINS_DDGI) let spacing = constants.center_spacing.w;
    @if(ORIGINS_DDGI) let unit = (position-constants.center_spacing.xyz)/(vec3<f32>(constants.grid_generation.xyz)*spacing);
    @if(ORIGINS_DDGI) if all(abs(unit) <= vec3(0.5)) {
        return sample_gi_field(previous,unit,normal,spacing,constants.center_spacing.xyz).irradiance;
    }
    return vec3(0.0);
}
fn reflection_brdf(color: vec3<f32>, metal: f32, reflectance: f32, roughness: f32, N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, coat: f32, coat_roughness: f32) -> vec3<f32> {
    let halfway = V+L;
    let H = halfway*inverseSqrt(max(dot(halfway,halfway),0.00000001));
    let nl = max(0.0,dot(N,L)); let nv = max(0.0001,dot(N,V));
    let nh = max(0.0,dot(N,H)); let vh = max(0.0,dot(V,H));
    // Match the native material's dielectric reflectance parameterization.
    // Forcing every surface to 4% made low-reflectance stone/fabric too bright.
    let f0 = mix(vec3(0.16*reflectance*reflectance),color,metal);
    let F = f0+(1.0-f0)*pow(1.0-vh,5.0);
    let a2 = pow(max(roughness,0.089),4.0);
    let D = a2/(3.14159265*pow(nh*nh*(a2-1.0)+1.0,2.0));
    let visibility = 0.5/max(nl*sqrt(nv*nv*(1.0-a2)+a2)+nv*sqrt(nl*nl*(1.0-a2)+a2),0.0001);
    let fd90 = 0.5+2.0*roughness*vh*vh;
    let burley = (1.0+(fd90-1.0)*pow(1.0-nl,5.0))*(1.0+(fd90-1.0)*pow(1.0-nv,5.0));
    let base = color*(1.0-metal)*(1.0-F)*(burley/3.14159265)+F*D*visibility;
    let coat_f = 0.04+0.96*pow(1.0-vh,5.0);
    let ca2 = pow(max(coat_roughness,0.089),4.0);
    let cd = ca2/(3.14159265*pow(nh*nh*(ca2-1.0)+1.0,2.0));
    let cv = 0.5/max(nl*sqrt(nv*nv*(1.0-ca2)+ca2)+nv*sqrt(nl*nl*(1.0-ca2)+ca2),0.0001);
    return (base*(1.0-coat*coat_f)+vec3(coat*coat_f*cd*cv))*nl;
}
@compute @workgroup_size(8,8,1)
fn reflection(@builtin(global_invocation_id) pixel: vec3<u32>) {
    let extent = textureDimensions(reflection_output);
    if any(pixel.xy >= extent) || pixel.z >= 6u { return; }
    let direction = sample_cube_dir((vec2<f32>(pixel.xy)+0.5)/vec2<f32>(extent),pixel.z)*vec3(1.0,1.0,-1.0);
    let ray = trace_surface(constants.reflection_position.xyz,direction,0.01,2000.0,0.0078125);
    var radiance = vec3(0.0);
    if ray.kind == RAY_QUERY_INTERSECTION_NONE {
        radiance = textureSampleLevel(sky,sky_sampler,direction*vec3(1.0,1.0,-1.0),0.0).rgb;
    } else {
        let surface = resolve_surface(ray,ray.t*0.0078125);
        let hit = surface.resolved;
        let N = faceForward(hit.world_normal,direction,hit.geometric_world_normal);
        let Ng = faceForward(hit.geometric_world_normal,direction,hit.geometric_world_normal);
        let V = -direction;
        // Normal-map detail must not move the visibility ray through its actual
        // surface and turn a bump into self-occlusion or a bright light leak.
        let origin = hit.world_position+Ng*0.02;
        let m = hit.material;
        radiance = m.emissive+m.base_color*(1.0-m.metallic)*(1.0-0.04*surface.coating)*surface.occlusion*local_irradiance(origin,N);
        for (var i = 0u; i < arrayLength(&directional_lights); i++) {
            let light = directional_lights[i];
            if dot(N,light.direction_to_light) > 0.0 && visible(origin,light.direction_to_light,2000.0) {
                radiance += light.luminance*light.inverse_pdf*reflection_brdf(m.base_color,m.metallic,m.reflectance,m.perceptual_roughness,N,V,light.direction_to_light,surface.coating,surface.coating_roughness);
            }
        }
        let local_lamps = lamp_range(origin);
        for (var i = 0u; i < local_lamps.y; i++) {
            let light = lamps[lamp_id(local_lamps,i)]; let delta = light.position_range.xyz-origin;
            let d2 = dot(delta,delta);
            if d2 >= light.position_range.w*light.position_range.w { continue; }
            let d = sqrt(max(d2,0.0001)); let L = delta/d;
            if dot(N,L) <= 0.0 || !visible(origin,L,max(0.01,d-light.radiance_radius.w)) { continue; }
            let ratio = d/light.position_range.w;
            let falloff = max(0.0,1.0-pow(ratio,4.0));
            radiance += light.radiance_radius.rgb*falloff*falloff/max(d2,light.radiance_radius.w*light.radiance_radius.w)*reflection_brdf(m.base_color,m.metallic,m.reflectance,m.perceptual_roughness,N,V,L,surface.coating,surface.coating_roughness);
        }
        // One additional glossy ray supplies scene occlusion and metal response.
        // No stochastic image reconstruction. Fall back to the geometric lobe
        // when an extreme shading normal would send this ray below the surface.
        let mapped_reflection = reflect(direction,N);
        let reflected = select(reflect(direction,Ng),mapped_reflection,dot(mapped_reflection,Ng) > 0.0);
        let bounce = trace_surface(origin,reflected,0.01,2000.0,0.0078125+m.perceptual_roughness*m.perceptual_roughness*0.25);
        var incident = vec3(0.0);
        if bounce.kind == RAY_QUERY_INTERSECTION_NONE {
            incident = textureSampleLevel(sky,sky_sampler,reflected*vec3(1.0,1.0,-1.0),m.perceptual_roughness*8.0).rgb;
        } else {
            let secondary = resolve_surface(bounce,bounce.t*(0.0078125+m.perceptual_roughness*m.perceptual_roughness*0.25));
            let sm = secondary.resolved.material;
            incident = sm.emissive+sm.base_color*(1.0-sm.metallic)*secondary.occlusion*local_irradiance(secondary.resolved.world_position,secondary.resolved.world_normal);
        }
        let f0 = mix(vec3(0.16*m.reflectance*m.reflectance),m.base_color,m.metallic);
        let grazing = pow(1.0-max(0.0,dot(N,V)),5.0);
        let F = f0+(1.0-f0)*grazing;
        let coat_F = surface.coating*(0.04+0.96*grazing);
        // A single continuation approximates both lobes; bound their combined
        // Fresnel energy instead of adding a coating on top of the full base.
        radiance += incident*(F*(1.0-coat_F)+vec3(coat_F));
    }
    textureStore(reflection_output,pixel.xy,i32(pixel.z),vec4(max(radiance,vec3(0.0)),1.0));
}
