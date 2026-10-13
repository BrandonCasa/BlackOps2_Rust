use crate::core::Result;

pub struct Track<const N: usize> {
    pub frames: Vec<u16>,
    pub values: Vec<[f32; N]>,
}

pub struct Bone {
    pub name: Option<String>,
    pub rotation: Option<Track<4>>,
    pub translation: Option<Track<3>>,
}

pub struct Clip {
    pub name: String,
    pub frame_rate: f32,
    pub frames: u16,
    pub source_flags: [u8;4],
    pub streamed_file_size: u32,
    pub loop_entry_time: f32,
    pub notifies: Vec<(Option<String>,f32)>,
    pub bones: Vec<Bone>,
}

pub fn rotation_samples(frames: &[u16], values: &[[f32;4]], frame_rate: f32) -> Result<(Vec<[f32;4]>, &'static str)> {
    if frames.is_empty() || frames.len() != values.len() || !frame_rate.is_finite() || frame_rate <= 0.
        || frames.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("Native rotation keys have an invalid time or value domain".into());
    }
    let raw: Vec<[f64;4]>=values.iter().map(|value| value.map(f64::from)).collect();
    let mut inverse_lengths=Vec::with_capacity(raw.len());
    for value in &raw {
        let squared=value.iter().map(|value| value*value).sum::<f64>();
        if !squared.is_finite() || squared <= 0.001 {
            return Err("Native rotation requires its near-zero quaternion fallback consumer".into());
        }
        inverse_lengths.push(1./squared.sqrt());
    }
    if raw.len() == 1 {
        return Ok((vec![std::array::from_fn(|axis| (raw[0][axis]*inverse_lengths[0]) as f32)],"LINEAR"));
    }
    let mut samples=vec![[0.;4];raw.len()*3];
    for index in 0..raw.len() {
        samples[index*3+1]=std::array::from_fn(|axis| (raw[index][axis]*inverse_lengths[index]) as f32);
    }
    for index in 0..raw.len()-1 {
        let a=raw[index]; let b=raw[index+1];
        let delta: [f64;4]=std::array::from_fn(|axis| b[axis]-a[axis]);
        let squared_delta=delta.iter().map(|value| value*value).sum::<f64>();
        let minimum_at=if squared_delta == 0. {0.} else {(-a.iter().zip(delta).map(|(a,d)| a*d).sum::<f64>()/squared_delta).clamp(0.,1.)};
        let minimum=a.iter().zip(delta).map(|(a,d)| (a+minimum_at*d).powi(2)).sum::<f64>();
        if minimum <= 0.001 { return Err("Native interpolated rotation enters its near-zero quaternion fallback".into()); }
        let duration=(frames[index+1] as f32/frame_rate-frames[index] as f32/frame_rate) as f64;
        if !duration.is_finite() || duration <= 0. { return Err("Native rotation times collapse in glTF float precision".into()); }
        let s0=inverse_lengths[index]; let s1=inverse_lengths[index+1]; let slope=s1-s0;
        samples[index*3+2]=std::array::from_fn(|axis| ((slope*a[axis]+s0*delta[axis])/duration) as f32);
        samples[(index+1)*3]=std::array::from_fn(|axis| ((slope*b[axis]+s1*delta[axis])/duration) as f32);
    }
    if samples.iter().flatten().any(|value| !value.is_finite()) { return Err("Native rotation derivatives exceed glTF float precision".into()); }
    Ok((samples,"CUBICSPLINE"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalize(value: [f64;4]) -> [f64;4] {
        let length=value.iter().map(|value| value*value).sum::<f64>().sqrt();
        value.map(|value| value/length)
    }

    #[test]
    fn cubic_rotation_preserves_native_unequal_norm_component_interpolation() {
        let frames=[0,11,37];
        let values=[[0.10,0.21,-0.31,0.83],[-0.27,0.44,0.12,0.71],[0.35,-0.19,0.17,0.91]];
        let rate=29.97;
        let (samples,interpolation)=rotation_samples(&frames,&values,rate).unwrap();
        assert_eq!(interpolation,"CUBICSPLINE");
        for index in 0..frames.len()-1 {
            let duration=(frames[index+1] as f32/rate-frames[index] as f32/rate) as f64;
            for u in [0.,0.03125,0.125,0.5,0.73,0.96875,1.] {
                let h00=2.*u*u*u-3.*u*u+1.; let h10=u*u*u-2.*u*u+u;
                let h01=-2.*u*u*u+3.*u*u; let h11=u*u*u-u*u;
                let cubic=normalize(std::array::from_fn(|axis| h00*samples[index*3+1][axis] as f64
                    +h10*duration*samples[index*3+2][axis] as f64+h01*samples[(index+1)*3+1][axis] as f64
                    +h11*duration*samples[(index+1)*3][axis] as f64));
                let native=normalize(std::array::from_fn(|axis| (1.-u)*values[index][axis] as f64+u*values[index+1][axis] as f64));
                assert!(cubic.iter().zip(native).all(|(a,b)| (a-b).abs()<0.0000002),"segment {index}, fraction {u}: {cubic:?} versus {native:?}");
            }
        }
        let (constant,interpolation)=rotation_samples(&[0],&[values[0]],rate).unwrap();
        assert_eq!(interpolation,"LINEAR");
        assert_eq!(constant.len(),1);
        assert!((constant[0].iter().map(|value| value*value).sum::<f32>()-1.).abs()<0.000001);
        assert!(rotation_samples(&[0],&[[0.;4]],rate).is_err());
        assert!(rotation_samples(&[0,1],&[[0.,0.,0.,1.],[0.,0.,0.,-1.]],rate).is_err());
    }
}
