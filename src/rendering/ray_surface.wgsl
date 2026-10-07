enable wgpu_ray_query;
#define_import_path origins::ray_surface
#import bevy_solari::scene_bindings::{trace_ray, geometry_ids, transforms, load_vertices, transform_positions, materials, material_ids, ResolvedRayHitFull, ResolvedMaterial}
#import origins::surface_effect::{Effect, effect_uv, effect_gradient, effect_color}
struct Surface { response: vec4<f32>, modes: vec4<u32>, layers: vec4<u32>, reveal: vec4<f32>, wetness: vec4<f32>, water: vec4<f32>, velocity: vec4<f32>, effect: Effect }
struct Vertex { color: vec4<f32>, uv: vec4<f32>, uv3: vec4<f32>, rotation1: vec4<f32>, rotation2: vec4<f32> }
struct Material { surface: Surface, color: vec4<f32>, emissive: vec4<f32>, response: vec4<f32>, alpha: vec4<f32>, textures0: vec4<u32>, textures1: vec4<u32>, textures2: vec4<u32>, textures3: vec4<u32> }
@group(2) @binding(0) var<storage> vertices: array<Vertex>;
@group(2) @binding(1) var<storage> surfaces: array<Material>;
@group(2) @binding(2) var maps: binding_array<texture_2d<f32>>;
@group(2) @binding(3) var map_samplers: binding_array<sampler>;
@group(2) @binding(4) var<storage> tails: array<vec4<f32>>;
// Word zero is a native frame stamp copied by reset_feedback. Subsequent words
// hold full-chain minima across all shared probe/reflection ray hits.
@group(2) @binding(5) var<storage,read_write> mip_feedback: array<atomic<u32>>;
@group(2) @binding(6) var<storage, read> effect_clock: vec4<f32>;

fn channel(m: Material, index: u32) -> u32 {
    if index < 4u { return m.textures0[index]; }
    if index < 8u { return m.textures1[index - 4u]; }
    if index < 12u { return m.textures2[index - 8u]; }
    return m.textures3[index - 12u];
}
fn sampled(m: Material, index: u32, uv: vec2<f32>, footprint: f32, gradients: mat2x2<f32>, fallback: vec4<f32>) -> vec4<f32> {
    let id = channel(m, index);
    if id == 0xffffffffu { return fallback; }
    let tail = tails[id];
    let extent = select(vec2<f32>(textureDimensions(maps[id])), tail.xy, tail.x > 0.0);
    let dx = gradients[0] * footprint;
    let dy = gradients[1] * footprint;
    let tx = dx * extent;
    let ty = dy * extent;
    // Work in texels on each axis. max(width,height)*a scalar UV stretch
    // overblurred rectangular maps and discarded the narrow footprint that
    // anisotropic filtering can resolve. Preserve that ellipse for sampling.
    let xx = dot(tx,tx); let yy = dot(ty,ty); let xy = dot(tx,ty);
    let discriminant = sqrt(max(0.0,(xx-yy)*(xx-yy)+4.0*xy*xy));
    let minor = sqrt(max(0.0,0.5*(xx+yy-discriminant)));
    let major = max(length(tx),length(ty));
    let sampled_texels = max(minor,major/max(tail.z,1.0));
    let mip = select(max(0.0,log2(max(1.0,sampled_texels))),0.0,tail.w != 0.0);
    atomicMin(&mip_feedback[id+1u],u32(floor(min(mip,31.0))));
    if tail.w != 0.0 { return textureSampleLevel(maps[id],map_samplers[id],uv,0.0); }
    // Explicit gradients enable native anisotropy in compute shaders. They
    // remain normalized UV derivatives, so the actual live view dimensions
    // automatically account for a streamed tail or a newly promoted view.
    return textureSampleGrad(maps[id],map_samplers[id],uv,dx,dy);
}
fn slope(sampled_normal: vec4<f32>, strength: f32) -> vec2<f32> {
    let xy = sampled_normal.rg * 2.0 - 1.0;
    return xy / max(sqrt(max(1.0 - dot(xy, xy), 0.0001)), 0.01) * strength;
}
fn normal_map(m: Material, index: u32, uv: vec2<f32>, footprint: f32, density: mat2x2<f32>, enabled: bool) -> vec2<f32> {
    if !enabled { return vec2(0.0); }
    return slope(sampled(m,index,uv,footprint,density,vec4(0.5,0.5,1.0,1.0)),m.surface.response.x);
}
fn response_map(m: Material, index: u32, uv: vec2<f32>, footprint: f32, density: mat2x2<f32>, enabled: bool, fallback: vec4<f32>) -> vec4<f32> {
    if !enabled { return fallback; }
    return sampled(m,index,uv,footprint,density,fallback);
}
fn reveal(control: f32, alpha: f32, parameters: vec2<f32>) -> f32 {
    let c = clamp(control * 0.998 + 0.001, 0.001, 0.999);
    let low = clamp(1.0-c-parameters.x*pow(c,parameters.y),0.0,1.0);
    let high = clamp(1.0-c+parameters.x*pow(1.0-c,parameters.y),0.0,1.0);
    return clamp((alpha-low)/max(high-low,0.001),0.0,1.0);
}
fn orient(value: vec2<f32>, rotation: vec4<f32>) -> vec2<f32> {
    let original = value * vec2(1.0,-1.0);
    return vec2(dot(original,rotation.xy),dot(original,rotation.zw))*vec2(1.0,-1.0);
}
fn layer_color(base: vec3<f32>, overlay: vec4<f32>, weight: f32, operation: u32) -> vec3<f32> {
    if operation == 2u { return base*mix(vec3(1.0),overlay.rgb,weight); }
    if operation == 3u { return base+overlay.rgb*weight; }
    return mix(base,overlay.rgb,weight);
}
fn projected_cone(world: array<vec3<f32>,3>, direction: vec3<f32>) -> array<vec3<f32>,2> {
    let forward = normalize(direction);
    let reference = select(vec3(0.0,1.0,0.0),vec3(1.0,0.0,0.0),abs(forward.y) > 0.99);
    let x = normalize(cross(reference,forward));
    let y = cross(forward,x);
    let normal = normalize(cross(world[1]-world[0],world[2]-world[0]));
    let cosine = dot(normal,forward);
    let denominator = select(-1.0,1.0,cosine >= 0.0)*max(abs(cosine),0.01);
    // Project a circular cone cross-section along the ray onto the hit plane.
    // The grazing footprint grows along one axis; AF preserves the other.
    return array(x-forward*(dot(normal,x)/denominator),y-forward*(dot(normal,y)/denominator));
}
fn density(a: vec2<f32>, b: vec2<f32>, c: vec2<f32>, world: array<vec3<f32>,3>, cone: array<vec3<f32>,2>) -> mat2x2<f32> {
    let e1 = world[1]-world[0]; let e2 = world[2]-world[0];
    let g00 = dot(e1,e1); let g01 = dot(e1,e2); let g11 = dot(e2,e2);
    let determinant = g00*g11-g01*g01;
    if determinant <= max(g00*g11*0.000001,0.000000000001) {
        let fallback = max(length(b-a)/max(length(e1),0.0001),length(c-a)/max(length(e2),0.0001));
        return mat2x2(vec2(fallback,0.0),vec2(0.0,fallback));
    }
    // Complete world-to-UV Jacobian, retaining skew and both projected cone
    // axes until each texture's actual extent is known.
    let d1 = b-a; let d2 = c-a;
    let u = (e1*(d1.x*g11-d2.x*g01)+e2*(d2.x*g00-d1.x*g01))/determinant;
    let v = (e1*(d1.y*g11-d2.y*g01)+e2*(d2.y*g00-d1.y*g01))/determinant;
    return mat2x2(vec2(dot(u,cone[0]),dot(v,cone[0])),vec2(dot(u,cone[1]),dot(v,cone[1])));
}
fn normalize_or(value: vec3<f32>, fallback: vec3<f32>) -> vec3<f32> {
    let squared_length = dot(value,value);
    if squared_length > 0.000000000001 { return value*inverseSqrt(squared_length); }
    return fallback;
}
struct Hit { resolved: ResolvedRayHitFull, occlusion: f32, coating: f32, coating_roughness: f32, opacity: f32, double_sided: bool }
// Naga-oil imports native functions before WGSL predeclared ray types. Carry
// their scalar fields across modules rather than storing an imported builtin
// RayIntersection into a second module's predeclared type.
struct SurfaceRay { kind: u32, t: f32, instance_index: u32, primitive_index: u32, barycentrics: vec2<f32>, front_face: bool, direction: vec3<f32> }
fn surface_ray(origin: vec3<f32>, direction: vec3<f32>, near: f32, far: f32) -> SurfaceRay {
    let ray = trace_ray(origin,direction,near,far,RAY_FLAG_NONE);
    return SurfaceRay(ray.kind,ray.t,ray.instance_index,ray.primitive_index,ray.barycentrics,ray.front_face,direction);
}
fn surface_double_sided(ray: SurfaceRay) -> bool {
    return surfaces[u32(-materials[material_ids[ray.instance_index]].reflectance - 1.0)].alpha.w != 0.0;
}
fn resolve_surface(ray: SurfaceRay, footprint: f32) -> Hit {
    let m = surfaces[u32(-materials[material_ids[ray.instance_index]].reflectance - 1.0)];
    let p = m.surface;
    let v = load_vertices(geometry_ids[ray.instance_index], ray.primitive_index);
    let world = transform_positions(transforms[ray.instance_index], v);
    let cone = projected_cone(world,ray.direction);
    let b = vec3(1.0-ray.barycentrics.x-ray.barycentrics.y, ray.barycentrics);
    let a0 = vertices[u32(abs(v[0].tangent.w))-1u];
    let a1 = vertices[u32(abs(v[1].tangent.w))-1u];
    let a2 = vertices[u32(abs(v[2].tangent.w))-1u];
    let color = a0.color*b.x+a1.color*b.y+a2.color*b.z;
    let layer_uv = a0.uv*b.x+a1.uv*b.y+a2.uv*b.z;
    let uv3 = (a0.uv3*b.x+a1.uv3*b.y+a2.uv3*b.z).xy;
    let rotation1 = a0.rotation1*b.x+a1.rotation1*b.y+a2.rotation1*b.z;
    let rotation2 = a0.rotation2*b.x+a1.rotation2*b.y+a2.rotation2*b.z;
    let uv0 = v[0].uv*b.x+v[1].uv*b.y+v[2].uv*b.z;
    let density0 = density(v[0].uv,v[1].uv,v[2].uv,world,cone);
    var uv1 = uv0; var uv2 = uv0;
    var density1 = density0; var density2 = density0;
    var weights = vec3(1.0,0.0,0.0);
    var albedo = sampled(m,7u,uv0,footprint,density0,vec4(1.0))*m.color;
    let tile_macro = (p.modes.x & 1u) != 0u;
    if tile_macro {
        let mask = sampled(m,6u,uv0,footprint,density0,vec4(1.0,0.0,0.0,1.0)).rg;
        weights = select(vec3(1.0,0.0,0.0),vec3(mask.g,mask.r,0.0)/max(mask.r+mask.g,0.0001),mask.r+mask.g > 0.0001);
        uv1 *= p.response.w; density1 *= p.response.w;
        albedo = vec4(sampled(m,7u,uv1,footprint,density1,vec4(1.0)).rgb*weights.x+sampled(m,8u,uv1,footprint,density1,vec4(1.0)).rgb*weights.y,1.0);
    }
    var n0 = normal_map(m,0u,uv0,footprint,density0,(p.modes.x & 2u) != 0u);
    var n1 = normal_map(m,1u,uv1,footprint,density1,(p.modes.x & 4u) != 0u);
    var n2 = vec2(0.0);
    var s0 = response_map(m,3u,uv0,footprint,density0,p.modes.y != 0u,vec4(1.0,0.8,0.0,0.0));
    var s1 = response_map(m,4u,uv1,footprint,density1,(p.modes.z & 1u) != 0u,s0); var s2 = s0;
    if p.layers.x > 1u && m.alpha.y != 0.0 && (p.modes.x & 32u) == 0u {
        uv1 = layer_uv.xy; uv2 = layer_uv.zw;
        density1 = density(a0.uv.xy,a1.uv.xy,a2.uv.xy,world,cone);
        density2 = density(a0.uv.zw,a1.uv.zw,a2.uv.zw,world,cone);
        let c0 = sampled(m,7u,uv0,footprint,density0,vec4(1.0));
        let c1 = sampled(m,8u,uv1,footprint,density1,vec4(1.0));
        let c2 = sampled(m,9u,uv2,footprint,density2,vec4(1.0));
        let c3 = sampled(m,10u,uv3,footprint,density(a0.uv3.xy,a1.uv3.xy,a2.uv3.xy,world,cone),vec4(1.0));
        let w1 = reveal(color.g,c1.a,p.reveal.xy)*clamp(c1.a,0.0,1.0);
        let w2 = select(0.0,reveal(color.b,c2.a,p.reveal.zw)*clamp(c2.a,0.0,1.0),p.layers.x > 2u);
        var rgb = layer_color(c0.rgb,c1,w1,p.layers.y);
        if p.layers.x > 2u { rgb = layer_color(rgb,c2,w2,p.layers.z); }
        if p.layers.x > 3u { rgb = layer_color(rgb,c3,c3.a,p.layers.w); }
        albedo = vec4(rgb,1.0);
        let r1 = select(0.0,w1,p.layers.y == 1u); let r2 = select(0.0,w2,p.layers.z == 1u);
        weights = vec3((1.0-r1)*(1.0-r2),r1*(1.0-r2),r2);
        n1 = n0;
        if (p.modes.x & 4u) != 0u { n1 = orient(normal_map(m,1u,uv1,footprint,density1,true),rotation1); }
        n2 = n0;
        if (p.modes.x & 8u) != 0u { n2 = orient(normal_map(m,2u,uv2,footprint,density2,true),rotation2); }
        s1 = response_map(m,4u,uv1,footprint,density1,(p.modes.z & 1u) != 0u,s0);
        s2 = response_map(m,5u,uv2,footprint,density2,(p.modes.z & 2u) != 0u,s0);
    }
    var normal = n0*weights.x+n1*weights.y+n2*weights.z;
    var response = s0*weights.x+s1*weights.y+s2*weights.z;
    if tile_macro {
        let core = normal_map(m,2u,uv1,footprint,density1,(p.modes.x & 8u) != 0u);
        let coating = normal_map(m,1u,uv1,footprint,density1,(p.modes.x & 4u) != 0u);
        normal = n0+core*weights.x+coating*weights.y;
        s0 = response_map(m,3u,uv1,footprint,density1,p.modes.y != 0u,vec4(1.0,0.8,0.0,0.0));
        s1 = response_map(m,4u,uv1,footprint,density1,(p.modes.z & 1u) != 0u,s0);
        response = s0*weights.y+s1*weights.x;
    }
    var coating_roughness = p.response.y;
    if m.alpha.y == 0.0 {
        albedo *= color;
        let mr = sampled(m,3u,uv0,footprint,density0,vec4(1.0));
        response.r = sampled(m,13u,uv0,footprint,density0,vec4(1.0)).r;
        response.g = m.response.x*mr.g;
        response.b = m.response.y*mr.b;
        response.a = m.response.w*sampled(m,12u,uv0,footprint,density0,vec4(1.0)).r;
        coating_roughness *= sampled(m,14u,uv0,footprint,density0,vec4(1.0)).g;
        if m.alpha.z != 0.0 { normal.y = -normal.y; }
    }
    let transform = transforms[ray.instance_index];
    let linear = mat3x3(transform[0].xyz,transform[1].xyz,transform[2].xyz);
    // Cofactors are the inverse transpose up to a common determinant factor.
    let cofactors = mat3x3(cross(linear[1],linear[2]),cross(linear[2],linear[0]),cross(linear[0],linear[1]));
    let determinant_sign = sign(dot(linear[0],cross(linear[1],linear[2])));
    let face_normal = normalize_or(cross(world[1]-world[0],world[2]-world[0]),vec3(0.0,1.0,0.0));
    let normal_matrix = cofactors*determinant_sign;
    // Match Bevy/MikkTSpace: normalize each transformed vertex, then interpolate
    // without fragment normalization or Gram-Schmidt before applying the map.
    var N = normalize_or(normal_matrix*v[0].normal,face_normal)*b.x
        + normalize_or(normal_matrix*v[1].normal,face_normal)*b.y
        + normalize_or(normal_matrix*v[2].normal,face_normal)*b.z;
    let T = normalize_or(linear*v[0].tangent.xyz,vec3(1.0,0.0,0.0))*b.x
        + normalize_or(linear*v[1].tangent.xyz,vec3(1.0,0.0,0.0))*b.y
        + normalize_or(linear*v[2].tangent.xyz,vec3(1.0,0.0,0.0))*b.z;
    let double_sided = m.alpha.w != 0.0;
    var geometric_normal = face_normal;
    if double_sided && !ray.front_face { N = -N; geometric_normal = -geometric_normal; }
    let handedness = sign(v[0].tangent.w)*determinant_sign;
    let B = cross(N,T)*handedness;
    let position = world[0]*b.x+world[1]*b.y+world[2]*b.z;
    let roughness = clamp(response.g,0.089,1.0);
    var emissive = m.emissive.rgb*sampled(m,11u,uv0,footprint,density0,vec4(1.0)).rgb;
    if (p.modes.x & 32u) != 0u {
        let facing = max(dot(-ray.direction,normalize_or(N,geometric_normal)),0.0);
        let parallax = -vec2(dot(-ray.direction,T),dot(-ray.direction,B))/max(facing,0.001);
        var samples: array<vec4<f32>,3>;
        var coverage = vec3(0.0);
        for (var layer = 0u; layer < 3u; layer++) {
            let uv = effect_uv(uv0,layer,p.effect,effect_clock.x,parallax);
            let gradients = effect_gradient(density0,layer,p.effect,effect_clock.x);
            samples[layer] = sampled(m,7u+layer,uv,footprint,gradients,vec4(1.0));
            coverage[layer] = sampled(m,7u+p.effect.coverage_layers[layer],uv,footprint,gradients,vec4(1.0)).a;
        }
        albedo = effect_color(samples,coverage,p.effect,facing)*color;
        albedo.a = 1.0; // Only opaque effect surfaces enter native opaque BLAS.
    }
    var reflectance = m.response.z;
    var metallic = clamp(response.b,0.0,1.0);
    if (p.modes.x & 64u) != 0u {
        emissive += albedo.rgb;
        albedo = vec4(vec3(0.0),albedo.a);
        reflectance = 0.0;
        metallic = 0.0;
        response.a = 0.0;
    }
    let resolved = ResolvedRayHitFull(position,position,normalize_or(T*normal.x+B*normal.y+N,geometric_normal),geometric_normal,
        vec4(T,handedness),uv0,length(cross(world[1]-world[0],world[2]-world[0]))*0.5,
        geometry_ids[ray.instance_index].triangle_count,
        ResolvedMaterial(albedo.rgb,emissive,reflectance,roughness,roughness*roughness,metallic));
    return Hit(resolved,clamp(response.r,0.0,1.0),clamp(response.a,0.0,1.0),clamp(coating_roughness,0.089,1.0),albedo.a,double_sided);
}
fn opacity(ray: SurfaceRay, m: Material, footprint: f32) -> f32 {
    if m.alpha.y != 0.0 && ((m.surface.modes.x & 1u) != 0u || m.surface.layers.x > 1u) {
        return 1.0; // Both authored paths replace base alpha exactly as raster does.
    }
    let v = load_vertices(geometry_ids[ray.instance_index],ray.primitive_index);
    let world = transform_positions(transforms[ray.instance_index],v);
    let cone = projected_cone(world,ray.direction);
    let b = vec3(1.0-ray.barycentrics.x-ray.barycentrics.y,ray.barycentrics);
    let uv = v[0].uv*b.x+v[1].uv*b.y+v[2].uv*b.z;
    var alpha = sampled(m,7u,uv,footprint,density(v[0].uv,v[1].uv,v[2].uv,world,cone),vec4(1.0)).a*m.color.a;
    if m.alpha.y == 0.0 {
        alpha *= vertices[u32(abs(v[0].tangent.w))-1u].color.a*b.x
            +vertices[u32(abs(v[1].tangent.w))-1u].color.a*b.y
            +vertices[u32(abs(v[2].tangent.w))-1u].color.a*b.z;
    }
    return alpha;
}
fn trace_surface(origin: vec3<f32>, direction: vec3<f32>, near: f32, far: f32, cone: f32) -> SurfaceRay {
    var minimum = near;
    // Native opaque BLAS stays engine-owned. Reject masked cutouts by advancing
    // the ray; the bound prevents pathological overlapping foliage hangs.
    var ray = surface_ray(origin,direction,minimum,far);
    for (var rejection = 0u; rejection < 32u; rejection++) {
        if ray.kind == RAY_QUERY_INTERSECTION_NONE { return ray; }
        let m = surfaces[u32(-materials[material_ids[ray.instance_index]].reflectance - 1.0)];
        if m.alpha.x < 0.0 || opacity(ray,m,ray.t*cone) >= m.alpha.x { return ray; }
        minimum = ray.t+0.001;
        if minimum >= far { break; }
        ray = surface_ray(origin,direction,minimum,far);
    }
    // Exhausting the finite retry budget cannot turn a rejected hole into an
    // opaque occluder. Return a miss consistently for visibility and lighting.
    ray.kind = RAY_QUERY_INTERSECTION_NONE;
    return ray;
}
