use bevy::{prelude::*,audio::{Decodable,Source,Sample,SampleRate,ChannelCount}};
use std::{sync::Arc,time::Duration};
#[derive(Clone,Copy)]
pub(super) struct Preset { early:f32,late:f32,gain:f32,feedback:f32,damping:f32 }
pub(super) fn preset(room:&str)->Preset {
    let name=match room {"tomb_robot_head"=>"tomb_bot_head","tomb_bunker_sml"=>"tomb_bunker_smallroom","tomb_bunker_med"=>"tomb_bunker_mediumroom","tomb_bunker_lrg"=>"tomb_bunker_largeroom","tomb_spiral_staircase"=>"tomb_stairs","tomb_spiral_staircase_bot"=>"tomb_stairs_big","tomb_large_chamber"=>"tomb_cave_2",other=>other};
    match name {
        "tomb_tunnel"=>Preset{early:0.840000,late:0.492188,gain:1.701607,feedback:0.874023,damping:0.309570},
        "tomb_stairs_big"=>Preset{early:0.623047,late:0.730469,gain:1.674280,feedback:0.677734,damping:0.521484},
        "tomb_stairs"=>Preset{early:0.502930,late:0.534180,gain:1.470883,feedback:0.677734,damping:0.521484},
        "tomb_overhang"=>Preset{early:0.500000,late:0.680000,gain:0.801505,feedback:0.639648,damping:0.316406},
        "tomb_outdoor"=>Preset{early:0.304688,late:0.541016,gain:0.965530,feedback:0.210000,damping:0.394531},
        "tomb_church"=>Preset{early:0.500000,late:0.680000,gain:0.487901,feedback:0.230000,damping:0.440000},
        "tomb_chamber"=>Preset{early:0.205000,late:0.300000,gain:1.024570,feedback:0.820000,damping:0.350000},
        "tomb_cave_2"=>Preset{early:0.840000,late:0.575000,gain:0.646470,feedback:0.740000,damping:0.580000},
        "tomb_cave"=>Preset{early:0.599609,late:0.575000,gain:0.883271,feedback:0.689453,damping:0.250977},
        "tomb_bunker_smallroom"=>Preset{early:0.441406,late:0.460938,gain:1.191714,feedback:0.856445,damping:0.401367},
        "tomb_bunker_mediumroom"=>Preset{early:0.603516,late:0.583008,gain:2.134504,feedback:0.426758,damping:0.389648},
        "tomb_bunker_largeroom"=>Preset{early:0.633789,late:0.595000,gain:1.701607,feedback:0.367188,damping:0.673828},
        "tomb_bot_head"=>Preset{early:0.385742,late:0.539062,gain:1.389853,feedback:0.825000,damping:0.701172},

        _=>Preset{early:0.304688,late:0.541016,gain:0.965530,feedback:0.21,damping:0.394531},
    }
}
#[derive(Asset,TypePath)]
pub(super) struct RoomClip { samples:Arc<[Sample]>,channels:ChannelCount,rate:SampleRate,preset:Preset }
impl RoomClip { pub(super) fn new(clip:&super::decoded::DecodedClip,preset:Preset)->Self{Self{samples:clip.samples.clone(),channels:clip.channels,rate:clip.rate,preset}} }
impl Decodable for RoomClip {
    type Decoder=RoomSamples;
    fn decoder(&self)->RoomSamples{let unit=self.rate.get() as f32*self.channels.get() as f32;let early=((self.preset.early*0.1).clamp(0.01,0.1)*self.rate.get()as f32)as usize*self.channels.get()as usize;let late=((self.preset.late*0.2).clamp(0.03,0.2)*self.rate.get()as f32)as usize*self.channels.get()as usize;
        RoomSamples{samples:self.samples.clone(),channels:self.channels,rate:self.rate,preset:self.preset,cursor:0,early:vec![0.;early.max(1)],late:vec![0.;late.max(1)],filter:vec![0.;self.channels.get()as usize],tail:(unit*2.)as usize}}
}
pub(super) struct RoomSamples { samples:Arc<[Sample]>,channels:ChannelCount,rate:SampleRate,preset:Preset,cursor:usize,early:Vec<f32>,late:Vec<f32>,filter:Vec<f32>,tail:usize }
impl Iterator for RoomSamples {
    type Item=Sample;
    fn next(&mut self)->Option<Sample>{if self.cursor>=self.samples.len()+self.tail{return None;}let input=self.samples.get(self.cursor).copied().unwrap_or(0.);let e=self.cursor%self.early.len();let l=self.cursor%self.late.len();let c=self.cursor%self.filter.len();let early=self.early[e];let late=self.late[l];self.filter[c]+=(late-self.filter[c])*(1.-self.preset.damping.clamp(0.,0.95));self.early[e]=input;self.late[l]=input+early*0.25+self.filter[c]*self.preset.feedback.clamp(0.,0.85);self.cursor+=1;Some((input+(early*0.2+self.filter[c]*0.25)*self.preset.gain).clamp(-1.,1.))}
}
impl Source for RoomSamples {
    fn current_span_len(&self)->Option<usize>{None}
    fn channels(&self)->ChannelCount{self.channels}
    fn sample_rate(&self)->SampleRate{self.rate}
    fn total_duration(&self)->Option<Duration>{Some(Duration::from_secs_f64((self.samples.len()+self.tail)as f64/self.channels.get()as f64/self.rate.get()as f64))}
}
