use super::file::{OrgNote, OrgFile, OrgInstrument};
use rodio::source::Source;
use std::fs::File;
use std::io::Read;

const VOLUME: f32 = 0.7;

fn vec_u8_into_i8(v: Vec<u8>) -> Vec<i8> {
  // ideally we'd use Vec::into_raw_parts, but it's unstable,
  // so we have to do it manually:

  // first, make sure v's destructor doesn't free the data
  // it thinks it owns when it goes out of scope
  let mut v = std::mem::ManuallyDrop::new(v);

  // then, pick apart the existing Vec
  let p = v.as_mut_ptr();
  let len = v.len();
  let cap = v.capacity();
  
  // finally, adopt the data into a new Vec
  unsafe { Vec::from_raw_parts(p as *mut i8, len, cap) }
}

fn add_to_vec(main: &mut Vec<f32>, to_add: &Vec<f32>, at: usize) {
  let at2: usize;
  if at % 2 != 0 {
    at2 = at + 1;
  } else {
    at2 = at;
  }
  for (i,x) in to_add.iter().enumerate() {
    if main.len() == i+at2 {
      main.push(*x);
    } else {
      if main.len() < i+at2 {
        for _ in main.len()..i+at2+1 {
          main.push(0.0);
        }
      }
      main[i+at2] += x;
    }
  }
}

fn vol_calc1(volume: f32) -> f32 {
  (10.0_f32.ln() * (volume-1.0)).exp()
}

fn vol_calc(volume: u8) -> f32 {
  vol_calc1((volume as f32) / 255.0)
}

fn pan_calc(pan: u8) -> (f32,f32) {
  let pan = pan as f32 / 6.0;
  let panl = if pan > 1.0 {(20.0_f32.ln()*(1.0-pan)).exp()} else {1.0};
  let panr = if pan < 1.0 {(20.0_f32.ln()*(pan-1.0)).exp()} else {1.0};
  (panl,panr)
}

struct Instrument {
  audio: Vec<i8>,
  drum_framerate: Option<u16>,
}

pub struct Player {
  sound: Vec<f32>,
  loop_start: usize,
  loop_end: usize,
  index: usize,
  padding: u32,
}

fn add_audio(sounds: &Vec<Instrument>, drums: &Vec<Instrument>, instrument: &OrgInstrument, click: u16, note: &OrgNote, out: &mut Vec<f32>, at: usize) {
  if note.drum {
    add_to_vec(out, &drums[instrument.instrument as usize].get_audio(0.0,0.0,note.volume,note.pan), at);
  } else {
    if instrument.pi {return}
    add_to_vec(out, &sounds[instrument.instrument as usize].get_audio(440.0*((note.note as f64 - 45.0) / 12.0).exp2(),note.length as f64 * click as f64 / 1000.0,note.volume,note.pan), at);
  }
}

impl Instrument {
  fn load_waves(data: &Vec<u8>) -> (Vec<Self>,Vec<Self>) {
    let mut ptr: usize = 0;
    let mqty = data[0];
    let mut mlen: usize = 0;
    for i in 0..3 {mlen = mlen << 8; mlen += data[i+1] as usize;};
    ptr += 4;
    let mut out1: Vec<Vec<u8>> = vec![];
    for _ in 0..mqty {
      out1.push(data[ptr..ptr+mlen].to_vec());
      ptr += mlen;
    }
    let mut out2: Vec<Vec<u8>> = vec![];
    let _framerate = ((data[ptr+1] as u16)<<8) + data[ptr+2] as u16;
    ptr += 3;
    for _ in 0..data[ptr-3] as usize {
      mlen = 0;
      for j in 0..3 {mlen = mlen << 8; mlen += data[ptr+j] as usize}
      ptr += 3;
      out2.push(data[ptr..ptr+mlen].to_vec());
      ptr += mlen;
    }
    let mut out3: Vec<Self> = vec![];
    let mut out4: Vec<Self> = vec![];
    for i in out1 {
      out3.push( Self {
        audio: vec_u8_into_i8(i),
        drum_framerate: None,
      })
    }
    for i in out2 {
      out4.push( Self {
        audio: vec_u8_into_i8(i),
        drum_framerate: Some(44100),
      })
    }
    (out3,out4)
  }

  fn get_audio(&self, freq: f64, duration: f64, volume: u8, pan: u8) -> Vec<f32> {
    let mut to_play: Vec<f32> = vec![];
    let vol = vol_calc(volume);
    let (panl, panr) = pan_calc(pan);
    if let Some(fr) = self.drum_framerate {
      for frame in 0..self.audio.len()*44100/fr as usize {
        to_play.push((self.audio[frame * fr as usize / 44100] as f32 / 128.0) * vol * panr);
        to_play.push((self.audio[frame * fr as usize / 44100] as f32 / 128.0) * vol * panl);
      }
      // for frame in &self.audio {
      //   to_play.push((*frame as f32 / 128.0)*vol*panr);
      //   to_play.push((*frame as f32 / 128.0)*vol*panl);
      // }
    } else {
      let size= self.audio.len() as u64;
      for frame in 0..(44100.0 * duration) as u64 {
        to_play.push((self.audio[(((frame*size*freq as u64) as f64/44100.0) as u64 % size) as usize] as f32 / 128.0)*vol*panr);
        to_play.push((self.audio[(((frame*size*freq as u64) as f64/44100.0) as u64 % size) as usize] as f32 / 128.0)*vol*panl);
      }
    }
    to_play
  }
}

impl Player {
  pub fn load(filename: &str) -> Self {
    let mut file = File::open("orgsamp.dat").unwrap();
    let mut data: Vec<u8> = vec![];
    file.read_to_end(&mut data).unwrap();
    let (sounds, drums) = Instrument::load_waves(&data);
    let mut file = File::open(filename).unwrap();
    let mut data: Vec<u8> = vec![];
    file.read_to_end(&mut data).unwrap();
    let orgdata = OrgFile::new(&data);
    let mut sound: Vec<f32> = vec![];
    for i in orgdata.notes {
      if i.position > orgdata.properties.loop_end {continue}
      let instrument = if i.drum {&orgdata.drums[i.instrument as usize]} else {&orgdata.sounds[i.instrument as usize]};
      add_audio(&sounds, &drums, instrument, orgdata.properties.tempo, &i, &mut sound, (i.position * orgdata.properties.tempo as u32) as usize * 44100 / 500);
    };
    //let mut file = File::create("audio.raw").unwrap();
    //unsafe {file.write(std::slice::from_raw_parts(sound.as_ptr() as *const u8, sound.len() *4)).unwrap();}
    Self {
      sound,
      loop_start: orgdata.properties.loop_start as usize * orgdata.properties.tempo as usize * 44100 / 500,
      loop_end: orgdata.properties.loop_end as usize * orgdata.properties.tempo as usize * 44100 / 500,
      index: 0,
      padding: 88200,
    }
  }
}

impl Iterator for Player {
  type Item = f32;

  fn next(&mut self) -> Option<Self::Item> {
    if self.padding > 0 {
      self.padding -= 1;
      return Some(0.0);
    }
    let ind = self.index;
    if self.index == self.loop_end {
      self.index = self.loop_start;
    } else {
      self.index += 1;
    }
    let sound: f32;
    if ind >= self.sound.len() {
      sound = 0.0;
    } else {
      sound = self.sound[ind] * VOLUME;
    }
    Some(if sound > 1.0 {1.0} else if sound < -1.0 {-1.0} else {sound})
  }
}

impl Source for Player {
  fn channels(&self) -> u16 { 2 }
  fn sample_rate(&self) -> u32 { 44100 }
  fn total_duration(&self) -> Option<std::time::Duration> { None }
  fn current_frame_len(&self) -> Option<usize> { None }
}