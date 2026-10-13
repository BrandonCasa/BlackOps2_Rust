use crate::{animation::{Bone, Clip, Track}, convert::Catalog, core::*, fastfile::{Address, Streams}};

pub struct DecodedClip {
    pub clip: Clip,
    pub has_root_delta: bool,
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
    label: &'static str,
}

impl<'a> Cursor<'a> {
    fn read(&mut self, size: usize) -> Result<&'a [u8]> {
        let end=self.offset.checked_add(size).ok_or("Native animation cursor overflow")?;
        let value=self.bytes.get(self.offset..end).ok_or_else(|| format!("Native animation {} cursor exceeds declared {} bytes at {} + {size}",self.label,self.bytes.len(),self.offset))?;
        self.offset=end;
        Ok(value)
    }
    fn byte(&mut self) -> Result<u8> { Ok(self.read(1)?[0]) }
    fn short(&mut self) -> Result<u16> { Ok(u16_at(self.read(2)?,0)) }
    fn float(&mut self) -> Result<f32> {
        let value=f32::from_bits(u32_at(self.read(4)?,0));
        if !value.is_finite() { return Err(format!("Native animation {} contains a non-finite float",self.label).into()); }
        Ok(value)
    }
    fn finish(self) -> Result<()> {
        if self.offset != self.bytes.len() { return Err(format!("Native animation {} cursor consumed {} of {} declared bytes",self.label,self.offset,self.bytes.len()).into()); }
        Ok(())
    }
}

fn stream<'a>(streams: &'a Streams, root: Address, pointer: usize, count: usize, stride: usize, label: &'static str) -> Result<Cursor<'a>> {
    let size=count.checked_mul(stride).ok_or("Native animation declared stream size overflow")?;
    let bytes=if size == 0 { &[] } else { streams.read(Address::decode(streams.u32(root.add(pointer)?)?)?,size)? };
    Ok(Cursor {bytes,offset:0,label})
}

fn script_string(streams: &Streams, word: u16) -> Result<Option<String>> {
    streams.script_strings.get(word as usize).copied().ok_or_else(|| format!("Native animation script-string index {word} is absent from its owning zone"))?
        .map(|address| streams.text(address)).transpose().map_err(Into::into)
}

fn keyframes(bytes: &mut Cursor<'_>, shorts: &mut Cursor<'_>, fine: &mut Cursor<'_>, wide: bool, last: u16) -> Result<Vec<u16>> {
    let maximum=shorts.short()? as usize;
    let count=maximum+1;
    let frames=if !wide {
        (0..count).map(|_| bytes.byte().map(u16::from)).collect::<Result<Vec<_>>>()?
    } else if maximum < 64 {
        (0..count).map(|_| shorts.short()).collect::<Result<Vec<_>>>()?
    } else {
        let blocks=maximum.div_ceil(256);
        let coarse=(0..=blocks).map(|_| shorts.short()).collect::<Result<Vec<_>>>()?;
        let frames=(0..count).map(|_| fine.short()).collect::<Result<Vec<_>>>()?;
        if coarse.iter().enumerate().any(|(index,&value)| value != frames[(index*256).min(maximum)]) {
            return Err("Native animation blocked coarse indices disagree with the fine boundary keys".into());
        }
        frames
    };
    if frames.first() != Some(&0) || frames.last() != Some(&last) || frames.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(format!("Native dynamic animation indices do not strictly span frame domain 0..{last}").into());
    }
    Ok(frames)
}

fn quaternion(source: &mut Cursor<'_>, lanes: usize) -> Result<[f32;4]> {
    let mut value=[0.;4];
    for lane in 4-lanes..4 { value[lane]=source.short()? as i16 as f32*f32::from_bits(0x38000100); }
    let [x,y,z,w]=value;
    Ok([x,z,-y,w])
}

fn decode(catalog: &Catalog<'_>, name: &str, zone: usize, root: Address) -> Result<DecodedClip> {
    let streams=&catalog.zones[zone].1.streams;
    let frames=streams.u16(root.add(14)?)?;
    let frame_rate=f32::from_bits(streams.u32(root.add(48)?)?);
    if !frame_rate.is_finite() || frame_rate <= 0. { return Err("Native animation has an invalid frame rate".into()); }
    if streams.u16(root.add(12)?)? != 0 { return Err("Native animation declares random-int data outside the recovered ordinary channel consumers".into()); }
    let count=streams.u8(root.add(33)?)? as usize;
    let mut bones=Vec::with_capacity(count);
    if count > 0 {
        let names=Address::decode(streams.u32(root.add(64)?)?)?;
        for index in 0..count {
            bones.push(Bone {name:script_string(streams,streams.u16(names.add(index*2)?)?)?,rotation:None,translation:None});
        }
    }
    let mut bytes=stream(streams,root,68,streams.u16(root.add(4)?)? as usize,1,"data-byte")?;
    let mut shorts=stream(streams,root,72,streams.u16(root.add(6)?)? as usize,2,"data-short")?;
    let mut floats=stream(streams,root,76,streams.u16(root.add(8)?)? as usize,4,"data-float")?;
    let mut random_shorts=stream(streams,root,80,streams.u32(root.add(40)?)? as usize,2,"random-short")?;
    let mut random_bytes=stream(streams,root,84,streams.u16(root.add(10)?)? as usize,1,"random-byte")?;
    let mut fine=stream(streams,root,92,streams.u32(root.add(44)?)? as usize,if frames >= 256 {2} else {1},"fine-index")?;
    let groups=streams.read(root.add(24)?,9)?;
    if groups[..5].iter().map(|&count| count as usize).sum::<usize>() != count
        || groups[5..].iter().map(|&count| count as usize).sum::<usize>() != count {
        return Err("Native animation channel-group totals disagree with the declared bone count".into());
    }
    let mut index=0;
    for (group,&count) in groups[..5].iter().enumerate() {
        for _ in 0..count {
            let dynamic=matches!(group,1|2);
            let keyframes=if dynamic {keyframes(&mut bytes,&mut shorts,&mut fine,frames>=256,frames)?} else {vec![0]};
            let mut values=Vec::with_capacity(keyframes.len());
            for _ in &keyframes {
                values.push(if group == 0 {[0.,0.,0.,1.]} else {
                    quaternion(if dynamic {&mut random_shorts} else {&mut shorts},if matches!(group,1|3) {2} else {4})?
                });
            }
            bones[index].rotation=Some(Track {frames:keyframes,values});
            index+=1;
        }
    }
    for (group,&count) in groups[5..].iter().enumerate() {
        for _ in 0..count {
            let index=bytes.byte()? as usize;
            let bone=bones.get_mut(index).ok_or("Native animation translation targets an absent bone ordinal")?;
            if bone.translation.is_some() { return Err("Native animation has duplicate translation bone ordinals".into()); }
            let keyframes=if group < 2 {keyframes(&mut bytes,&mut shorts,&mut fine,frames>=256,frames)?} else {vec![0]};
            let base=if group < 3 {[floats.float()?,floats.float()?,floats.float()?]} else {[0.;3]};
            let scale=if group < 2 {[floats.float()?,floats.float()?,floats.float()?]} else {[0.;3]};
            let mut values=Vec::with_capacity(keyframes.len());
            for _ in &keyframes {
                let mut value=base;
                if group < 2 {
                    for axis in 0..3 {
                        let packed=if group == 0 {random_bytes.byte()? as f32} else {random_shorts.short()? as f32};
                        value[axis]+=scale[axis]*packed;
                    }
                }
                let [x,y,z]=value;
                let value=[x*0.0254,z*0.0254,-y*0.0254];
                if value.iter().any(|value| !value.is_finite()) { return Err("Decoded native translation is non-finite".into()); }
                values.push(value);
            }
            bone.translation=Some(Track {frames:keyframes,values});
        }
    }
    for cursor in [bytes,shorts,floats,random_shorts,random_bytes,fine] {cursor.finish()?;}
    let source_flags=streams.read(root.add(16)?,4)?.try_into()?;
    let streamed_file_size=streams.u32(root.add(20)?)?;
    let loop_entry_time=f32::from_bits(streams.u32(root.add(60)?)?);
    if !loop_entry_time.is_finite() { return Err("Native animation loop-entry time is non-finite".into()); }
    let notify_count=streams.u8(root.add(34)?)? as usize;
    let mut notifies=Vec::with_capacity(notify_count);
    if notify_count > 0 {
        let entries=Address::decode(streams.u32(root.add(96)?)?)?;
        for index in 0..notify_count {
            let entry=entries.add(index*8)?;
            let time=f32::from_bits(streams.u32(entry.add(4)?)?);
            if !time.is_finite() { return Err("Native animation notification has non-finite time".into()); }
            notifies.push((script_string(streams,streams.u16(entry)?)?,time));
        }
    }
    Ok(DecodedClip {clip:Clip {name:name.into(),frame_rate,frames,source_flags,streamed_file_size,loop_entry_time,notifies,bones},has_root_delta:streams.u32(root.add(100)?)? != 0})
}

pub fn read(catalog: &Catalog<'_>) -> Result<Vec<DecodedClip>> {
    catalog.definitions(4).map(|(name,zone,root)| decode(catalog,name,zone,root)
        .map_err(|error| format!("Native animation {}/{name}: {error}",catalog.zones[zone].0).into())).collect()
}
