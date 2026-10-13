use crate::{convert::Catalog, core::*, fastfile::{Address, Streams}};

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
    pub source_flags: [u8;8],
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

struct Cursor<'a> { bytes: &'a [u8], offset: usize }

impl<'a> Cursor<'a> {
    fn read(&mut self, size: usize) -> Result<&'a [u8]> {
        let end = self.offset.checked_add(size).ok_or("Native animation stream size overflow")?;
        let value = self.bytes.get(self.offset..end).ok_or("Native animation track exceeds its source stream")?;
        self.offset = end;
        Ok(value)
    }
    fn byte(&mut self) -> Result<u8> { Ok(self.read(1)?[0]) }
    fn short(&mut self) -> Result<u16> { Ok(u16_at(self.read(2)?,0)) }
    fn float(&mut self) -> Result<f32> {
        let value = f32::from_bits(u32_at(self.read(4)?,0));
        if !value.is_finite() { return Err("Native animation contains a non-finite translation".into()); }
        Ok(value)
    }
}

fn stream<'a>(streams: &'a Streams, root: Address, pointer: usize, bytes: usize) -> Result<Cursor<'a>> {
    let bytes = if bytes == 0 { &[] } else {
        streams.read(Address::decode(streams.u32(root.add(pointer)?)?)?,bytes)?
    };
    Ok(Cursor { bytes, offset: 0 })
}

fn keyframes(bytes: &mut Cursor<'_>, shorts: &mut Cursor<'_>, wide: bool, last: u16) -> Result<Vec<u16>> {
    let count = shorts.short()? as usize;
    if wide && count >= 64 {
        return Err("Native wide animation with >=64 keys requires the 0078f7c0 block-index consumer".into());
    }
    let frames = (0..=count).map(|_| if wide { shorts.short() } else { bytes.byte().map(u16::from) }).collect::<Result<Vec<_>>>()?;
    if frames.iter().any(|&frame| frame > last) || frames.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("Native animation keyframe indices are outside their ordered frame domain".into());
    }
    Ok(frames)
}

fn quaternion(source: &mut Cursor<'_>, lanes: usize) -> Result<[f32;4]> {
    let mut value = [0.;4];
    for lane in 4-lanes..4 { value[lane] = source.short()? as i16 as f32*f32::from_bits(0x38000100); }
    let [x,y,z,w] = value;
    Ok([x,z,-y,w])
}

pub fn decode(catalog: &Catalog<'_>, name: &str, zone: usize, root: Address) -> Result<Clip> {
    let streams = &catalog.zones[zone].1.streams;
    let frames = streams.u16(root.add(14)?)?;
    let frame_rate = f32::from_bits(streams.u32(root.add(48)?)?);
    if !frame_rate.is_finite() || frame_rate <= 0. { return Err(format!("Animation {name} has invalid native frame rate").into()); }
    if streams.u32(root.add(100)?)? != 0 {
        return Err(format!("Animation {name}: native root-delta consumer is not recovered").into());
    }
    if streams.u16(root.add(12)?)? != 0 || streams.u32(root.add(44)?)? != 0 {
        return Err(format!("Animation {name}: native random-int or external index stream requires its consumer").into());
    }
    let count = streams.u8(root.add(33)?)? as usize;
    let names = Address::decode(streams.u32(root.add(64)?)?)?;
    let mut bones = Vec::with_capacity(count);
    for index in 0..count {
        let word = streams.u16(names.add(index*2)?)? as usize;
        let address = streams.script_strings.get(word).copied().ok_or("Native animation bone name is absent from its owning zone")?;
        bones.push(Bone { name: address.map(|address| streams.text(address)).transpose()?, rotation:None, translation:None });
    }
    let mut bytes = stream(streams,root,68,streams.u16(root.add(4)?)? as usize)?;
    let mut shorts = stream(streams,root,72,streams.u16(root.add(6)?)? as usize*2)?;
    let mut floats = stream(streams,root,76,streams.u16(root.add(8)?)? as usize*4)?;
    let mut random_shorts = stream(streams,root,80,streams.u32(root.add(40)?)? as usize*2)?;
    let mut random_bytes = stream(streams,root,84,streams.u16(root.add(10)?)? as usize)?;
    let groups = streams.read(root.add(24)?,9)?;
    let mut index = 0;
    for (group,&size) in groups[..5].iter().enumerate() {
        for _ in 0..size {
            let bone = bones.get_mut(index).ok_or("Native animation rotation groups exceed bone count")?;
            let dynamic = matches!(group,1|2);
            let keyframes = if dynamic { keyframes(&mut bytes,&mut shorts,frames>=256,frames)? } else { vec![0] };
            let mut values = Vec::with_capacity(keyframes.len());
            for _ in &keyframes {
                values.push(if group == 0 { [0.,0.,0.,1.] } else {
                    quaternion(if dynamic { &mut random_shorts } else { &mut shorts },if matches!(group,1|3) {2} else {4})?
                });
            }
            bone.rotation=Some(Track {frames:keyframes,values});
            index+=1;
        }
    }
    for (group,&size) in groups[5..].iter().enumerate() {
        for _ in 0..size {
            let index = bytes.byte()? as usize;
            let bone = bones.get_mut(index).ok_or("Native animation translation targets an absent bone")?;
            if bone.translation.is_some() { return Err("Native animation has duplicate translation tracks".into()); }
            let keyframes = if group < 2 { keyframes(&mut bytes,&mut shorts,frames>=256,frames)? } else { vec![0] };
            let mut values = Vec::with_capacity(keyframes.len());
            let base = if group < 3 { [floats.float()?,floats.float()?,floats.float()?] } else { [0.;3] };
            let scale = if group < 2 { [floats.float()?,floats.float()?,floats.float()?] } else { [0.;3] };
            for _ in &keyframes {
                let mut value = base;
                if group < 2 {
                    for axis in 0..3 {
                        let packed = if group == 0 { random_bytes.byte()? as f32 } else { random_shorts.short()? as f32 };
                        value[axis]+=scale[axis]*packed;
                    }
                }
                let [x,y,z] = value;
                let value=[x*0.0254,z*0.0254,-y*0.0254];
                if value.iter().any(|value| !value.is_finite()) { return Err("Decoded native animation translation is non-finite".into()); }
                values.push(value);
            }
            bone.translation=Some(Track {frames:keyframes,values});
        }
    }
    for (label,cursor) in [("byte",bytes),("short",shorts),("float",floats),("random short",random_shorts),("random byte",random_bytes)] {
        if cursor.offset != cursor.bytes.len() { return Err(format!("Animation {name}: decoded {} of {} native {label} stream bytes",cursor.offset,cursor.bytes.len()).into()); }
    }
    let source_flags=streams.read(root.add(16)?,8)?.try_into()?;
    let notify_count=streams.u8(root.add(34)?)? as usize;
    let mut notifies=Vec::with_capacity(notify_count);
    if notify_count > 0 {
        let source=Address::decode(streams.u32(root.add(96)?)?)?;
        for index in 0..notify_count {
            let entry=source.add(index*8)?;
            let word=streams.u16(entry)? as usize;
            let address=streams.script_strings.get(word).copied().ok_or("Native animation notify name is absent from its owning zone")?;
            let time=f32::from_bits(streams.u32(entry.add(4)?)?);
            if !time.is_finite() { return Err("Native animation notify time is non-finite".into()); }
            let name=address.map(|address| streams.text(address)).transpose()?;
            if name.as_deref().is_some_and(|name| name.eq_ignore_ascii_case("jumpcut")) {
                return Err("Native animation jumpcut intervals require source-faithful stepped sampling".into());
            }
            notifies.push((name,time));
        }
    }
    Ok(Clip {name:name.into(),frame_rate,frames,source_flags,notifies,bones})
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
