fn sample_tony_mc_mapface_lut(stimulus: vec3<f32>) -> vec3<f32> {
    let softclip = $SOFTCLIP;
    let pivot = 1.0 - softclip;
    let encoded = sqrt(max(stimulus, vec3(0.0)));
    let compressed = vec3(1.0) - softclip * exp((vec3(pivot) - encoded) / softclip);
    let clipped = select(encoded, compressed, encoded > vec3(pivot));
    let uv = saturate(clipped) * (31.0 / 32.0) + vec3(0.5 / 32.0);
    let display = sample_current_lut(uv);
    return select(display / 12.92, pow((display + vec3(0.055)) / 1.055, vec3(2.4)), display > vec3(0.04045));
}
