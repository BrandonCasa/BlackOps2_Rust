//! Observe the existing movie player. A clean process exit alone is not an ending.
use super::Controller;
use bevy::prelude::*;
use serde::Serialize;
use serde_json::{Value, json};
use std::{fs::File, io::{Read, Write}, path::PathBuf, time::{Duration, Instant}};

#[derive(Resource, Serialize)]
pub(crate) struct Playback {
    pub file: PathBuf,
    pub position: f64,
    pub duration: Option<f64>,
    pub speed: Option<f64>,
    pub paused: Option<bool>,
    pub captures: [Option<PathBuf>; 3],
    pub terminal: Option<Value>,
    pub failure: Option<String>,
    pub complete: bool,
    #[serde(skip)] report: PathBuf,
    #[serde(skip)] buffer: Vec<u8>,
    #[serde(skip)] attached: bool,
    #[serde(skip)] released: bool,
    #[serde(skip)] observed_file: bool,
    #[serde(skip)] desired_pause: bool,
    #[serde(skip)] pending_captures: [bool; 3],
    #[serde(skip)] last_message: Instant,
    #[serde(skip)] ping: Instant,
    #[serde(skip)] progressed: bool,
}
impl Playback {
    pub(crate) fn new(file: PathBuf, controller: &Controller) -> Self {
        Self { file, position:0., duration:None,speed:None,paused:None,captures:default(),terminal:None,failure:None,complete:false,
            report:controller.report.clone(),buffer:Vec::with_capacity(8192),attached:false,released:false,observed_file:false,
            desired_pause:false,pending_captures:[false;3],last_message:Instant::now(),ping:Instant::now(),progressed:false }
    }
    pub(crate) fn pause(&mut self, paused: bool) { self.desired_pause = paused; }
    fn send(pipe: &mut File, command: Value, id: u32) -> Result<(), String> {
        writeln!(pipe,"{}",json!({"command":command,"request_id":id})).map_err(|e|format!("Cinematic IPC write: {e}"))
    }
    pub(crate) fn poll(&mut self, pipe: &mut File) -> Result<Vec<String>,String> {
        if !self.attached {
            for (id,name) in ["path","time-pos","duration","speed","pause"].into_iter().enumerate() {
                Self::send(pipe,json!(["observe_property",id,name]),id as u32)?;
            }
            for (key,message) in [("ESC","validator-toggle-pause"),("F12","validator-abort")] {
                Self::send(pipe,json!(["keybind",key,format!("script-message {message}")]),20)?;
            }
            self.attached=true;
        }
        let mut controls=Vec::new();
        let available=available(pipe)?;
        if available>0 {
            let mut bytes=[0u8;8192];
            let count=pipe.read(&mut bytes[..available.min(8192)]).map_err(|e|format!("Cinematic IPC read: {e}"))?;
            if count==0 { return Err("Cinematic IPC closed before terminal evidence".into()); }
            self.buffer.extend_from_slice(&bytes[..count]);
            while let Some(end)=self.buffer.iter().position(|b|*b==b'\n') {
                let message:Value=serde_json::from_slice(&self.buffer[..end]).map_err(|e|format!("Cinematic IPC message: {e}"))?;
                self.buffer.drain(..=end);
                self.last_message=Instant::now();
                if message["event"]=="client-message" {
                    if let Some(control)=message["args"][0].as_str() { controls.push(control.to_owned()); }
                }
                self.observe(message);
            }
        }
        if self.terminal.is_some() { return Ok(controls); }
        if self.last_message.elapsed()>Duration::from_secs(5) { return Err("Cinematic telemetry lost".into()); }
        if self.ping.elapsed()>Duration::from_millis(500) {
            Self::send(pipe,json!(["get_property","pause"]),30)?;
            self.ping=Instant::now();
        }
        if !self.released && self.observed_file && self.duration.is_some() && self.speed==Some(1.) && self.paused==Some(true) {
            Self::send(pipe,json!(["set_property","pause",self.desired_pause]),31)?;
            self.released=true;
        } else if self.released && self.paused.is_some_and(|p|p!=self.desired_pause) {
            Self::send(pipe,json!(["set_property","pause",self.desired_pause]),31)?;
        }
        if let Some(duration)=self.duration {
            for (index,at) in [1.,duration*0.5,(duration-2.).max(1.)].into_iter().enumerate() {
                if self.position>=at && !self.pending_captures[index] {
                    let path=self.report.join(format!("ending-{}.png",["early","middle","late"][index]));
                    Self::send(pipe,json!(["screenshot-to-file",path,"window"]),100+index as u32)?;
                    self.pending_captures[index]=true;
                }
            }
        }
        if let Some(error)=&self.failure { return Err(error.clone()); }
        Ok(controls)
    }
    fn observe(&mut self, message: Value) {
        match message["event"].as_str() {
            Some("property-change") => match message["name"].as_str() {
                Some("path") => {
                    if let Some(path)=message["data"].as_str() {
                        self.observed_file=path.replace('\\',"/").eq_ignore_ascii_case(&self.file.to_string_lossy().replace('\\',"/"));
                        if !self.observed_file { self.failure=Some(format!("Unexpected ending movie: {path}")); }
                    }
                }
                Some("time-pos") => if let Some(position)=message["data"].as_f64() {
                    if position+0.05<self.position { self.failure=Some("Ending playback moved backwards".into()); }
                    self.progressed |= position>self.position+0.01;
                    self.position=self.position.max(position);
                },
                Some("duration") => self.duration=message["data"].as_f64().filter(|d|*d>0.),
                Some("speed") => {
                    self.speed=message["data"].as_f64();
                    if self.speed.is_some_and(|speed| (speed-1.).abs()>1e-6) { self.failure=Some("Ending playback speed changed".into()); }
                }
                Some("pause") => self.paused=message["data"].as_bool(),
                _=>{}
            },
            Some("seek") => self.failure=Some("Ending playback was sought or skipped".into()),
            Some("end-file") => {
                self.complete=self.failure.is_none() && self.observed_file && self.progressed && self.speed==Some(1.)
                    && message["reason"]=="eof" && message.get("file_error").is_none_or(Value::is_null)
                    && self.duration.is_some_and(|d| (d-self.position).abs()<=1.) && self.captures.iter().all(Option::is_some);
                if !self.complete { self.failure=Some("Ending stopped without complete natural playback".into()); }
                self.terminal=Some(message);
                let path=self.report.join("ending.json");
                if let Err(e)=std::fs::write(path,serde_json::to_vec_pretty(self).unwrap()) { self.failure=Some(format!("Save ending result: {e}")); self.complete=false; }
            }
            _=>{}
        }
        if let Some(id)=message["request_id"].as_u64().filter(|id|(100..103).contains(id)) {
            let index=id as usize-100;
            let path=self.report.join(format!("ending-{}.png",["early","middle","late"][index]));
            if message["error"]=="success" && path.is_file() { self.captures[index]=Some(path); }
            else { self.failure=Some(format!("Ending screenshot {} failed",index+1)); }
        }
    }
    pub(crate) fn exited(&mut self) -> Result<(),String> {
        if self.complete { Ok(()) } else { Err(self.failure.clone().unwrap_or_else(||"Movie exited before natural EOF was observed".into())) }
    }
}

#[cfg(windows)]
fn available(pipe: &File) -> Result<usize,String> {
    use std::os::windows::io::AsRawHandle;
    #[link(name="kernel32")]
    unsafe extern "system" { fn PeekNamedPipe(handle:*mut std::ffi::c_void,buffer:*mut std::ffi::c_void,size:u32,read:*mut u32,total:*mut u32,left:*mut u32)->i32; }
    let mut bytes=0;
    let ok=unsafe { PeekNamedPipe(pipe.as_raw_handle(),std::ptr::null_mut(),0,std::ptr::null_mut(),&mut bytes,std::ptr::null_mut()) };
    if ok!=0 { Ok(bytes as usize) } else { Err(format!("Cinematic IPC: {}",std::io::Error::last_os_error())) }
}
#[cfg(not(windows))]
fn available(_: &File) -> Result<usize,String> { Err("Validator movie observation requires Windows named pipes".into()) }
