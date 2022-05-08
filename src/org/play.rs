use super::file::{OrgTrack, OrgNote, OrgFile, OrgProps};
use std::{
  fs::File,
  io::{
    Read, Seek, SeekFrom,
  }
};
use rodio::{
  source::Source,
  dynamic_mixer::{
    mixer,
    DynamicMixer
  },
};

const VOLUME: f32 = 1.0;

fn vol_calc(volume: u8) -> f32 {
  (10.0_f32.ln() * (((volume as f32) / 255.0)-1.0)).exp()
}

fn pan_calc(pan: u8) -> (f32,f32) {
  let pan = pan as f32 / 6.0;
  let panl = if pan > 1.0 {(20.0_f32.ln()*(1.0-pan)).exp()} else {1.0};
  let panr = if pan < 1.0 {(20.0_f32.ln()*(pan-1.0)).exp()} else {1.0};
  (panl, panr)
}

fn freq_calc(x: u8, f: u16) -> f32 {(440.0 * (2.0_f32.ln()*((x as f32 - 45.0)/12.0)).exp())*256.0 + (f - 1000) as f32}

struct Instrument {
  start: u64,
  len: u32,
}

struct LoadedInstrument {
  audio: Vec<i8>,
  drum: bool,
}

struct InstrumentList {
  file: File,
  sounds: Vec<Instrument>,
  drums: Vec<Instrument>,
}

#[derive(Clone)]
struct PlayerNote {
  voll: f32,
  volr: f32,
  position: u32,
  pitch: f32,
  length: u8,
}

struct TrackData {
  instrument: LoadedInstrument,
  notes: Vec<PlayerNote>,
}

struct Track {
  second_frame: Option<f32>,
  current: Option<PlayerNote>,
  track: TrackData,
  time: u32,
  time_click: u32,
  click: u32,
  loop_start: u32,
  loop_end: u32,
}

impl InstrumentList {
  fn load_waves(mut file: File) -> Self {
    let mut mqty: u8 = {
      let mut tmp = [0];
      file.read_exact(&mut tmp).unwrap();
      tmp[0]
    };
    let mut mlen: u32 = 0;
    {
      let mut tmp = [0,0,0];
      file.read_exact(&mut tmp).unwrap();
      for i in 0..3 {
        mlen = mlen << 8; mlen += tmp[i] as u32;
      };
    }
    let mut out1: Vec<Instrument> = vec![];
    for _ in 0..mqty {
      out1.push(
        Instrument {
          len: mlen,
          start: file.seek(SeekFrom::Current(0)).unwrap(),
        }
      );
      file.seek(SeekFrom::Current(mlen as i64)).unwrap();
    }
    let mut out2: Vec<Instrument> = vec![];
    mqty = {
      let mut tmp = [0];
      file.read_exact(&mut tmp).unwrap();
      tmp[0]
    };
    file.seek(SeekFrom::Current(2)).unwrap();
    for _ in 0..mqty as usize {
      let mut tmp: [u8; 3] = [0;3];
      file.read_exact(&mut tmp).unwrap();
      mlen = 0;
      for j in 0..3 {mlen = mlen << 8; mlen += tmp[j] as u32}
      out2.push(
        Instrument {
          len: mlen,
          start: file.seek(SeekFrom::Current(0)).unwrap(),
        }
      );
      file.seek(SeekFrom::Current(mlen as i64)).unwrap();
    }

    Self {
      sounds: out1,
      drums: out2,
      file
    }
  }

  fn load_instrument(&mut self, instr: u8, drum: bool) -> LoadedInstrument {
    let instrument = &(if drum {&self.drums} else {&self.sounds})[instr as usize];
    self.file.seek(SeekFrom::Start(instrument.start)).unwrap();
    let mut tmp: Vec<u8> = vec![0; instrument.len as usize];
    self.file.read_exact(&mut tmp).unwrap();
    self.file.rewind().unwrap();
    let mut v = std::mem::ManuallyDrop::new(tmp);

    let p = v.as_mut_ptr();
    let len = v.len();
    let cap = v.capacity();

    LoadedInstrument {
      audio: unsafe {Vec::from_raw_parts(p as *mut i8, len, cap)},
      drum: drum,
    }
  }
}

impl LoadedInstrument {
  fn get_frame(&self, offset: u32, note: &PlayerNote) -> Option<(f32,f32)> {
    if self.drum {
      if offset >= self.audio.len() as u32 {
        None
      } else {
        let f = self.audio[offset as usize] as f32 / 128.0;
        Some((f*note.voll,f*note.volr))
      }
    } else {
      let f = self.audio[(offset as f32 * note.pitch / 44100.0) as usize % 256] as f32 / 128.0;
      //println!("{}",note.pitch);
      //println!("{:#?}",self.audio);
      //panic!();
      Some((f*note.voll,f*note.volr))
    }
  }
}

impl PlayerNote {
  fn new(from: &OrgNote, freq: u16) -> Self {
    let vol = vol_calc(from.volume);
    let (panl,panr) = pan_calc(from.pan);
    //let (panl, panr) = (1.0,1.0);
    Self {
      length: from.length,
      pitch: freq_calc(from.note, freq),
      position: from.position,
      voll: panl * vol,
      volr: panr * vol,
    }
  }
}

impl TrackData {
  fn new(from: &OrgTrack, drum: bool, instruments: &mut InstrumentList) -> Self {
    let mut notes: Vec<PlayerNote> = vec![];
    for note in &from.notes {
      notes.push(PlayerNote::new(&note, from.pitch))
    }
    Self {
      instrument: instruments.load_instrument(from.instrument, drum),
      notes,
    }
  }
}

impl Track {
  fn new(from: &OrgTrack, props: &OrgProps, drum: bool, instruments: &mut InstrumentList) -> Self {
    let current: Option<PlayerNote>;
    let track = TrackData::new(from,drum,instruments);
    if from.notes.len() > 0 && from.notes[0].position == 0 {
      current = Some(track.notes[0].clone());
    } else {
      current = None;
    }
    Self {
      track,
      time: 0,
      time_click: 0,
      click: props.click as u32 * 441 / 10,
      loop_start: props.loop_start,
      loop_end: props.loop_end,
      current,
      second_frame: None,
    }
  }
}

impl Iterator for Track {
  type Item = f32;

  fn next(&mut self) -> Option<Self::Item> {
    Some(if let Some(f) = self.second_frame {
      self.second_frame = None;
      f * VOLUME
    } else {
      let tmp: f32;
      if let Some(note) = &self.current {
        if let Some(frame) = self.track.instrument.get_frame(
            ((self.time_click + note.length as u32 - note.position) * self.click as u32) + self.time as u32, note) {
          tmp = frame.1;
          self.second_frame = Some(frame.0);
        } else {
          self.current = None;
          tmp = 0.0;
          self.second_frame = Some(0.0);
        }
      } else {
        tmp = 0.0;
        self.second_frame = Some(0.0);
      }

      self.time += 1;
      if self.time == self.click {
        self.time = 0;
        self.time_click += 1;
        if self.time_click == self.loop_end {
          self.time_click = self.loop_start;
          self.current = None;
        } else {
          if !self.track.instrument.drum {
            if let Some(current) = &self.current {
              if (current.position + current.length as u32) <= self.time_click {
                self.current = None;
              }
            }
          }
        }
        for note in &self.track.notes {
          if note.position == self.time_click {
            self.current = Some(note.clone());
            break;
          }
        }
      }

      tmp * VOLUME
    })
  }
}

impl Source for Track {
  fn current_frame_len(&self) -> Option<usize> {None}
  fn sample_rate(&self) -> u32 {44100}
  fn channels(&self) -> u16 {2}
  fn total_duration(&self) -> Option<std::time::Duration> {None}
}

pub fn get_mixer(file: OrgFile) -> DynamicMixer<f32> {
  let (inp, out) = mixer(2,44100);

  let mut instruments = InstrumentList::load_waves(File::open("orgsamp.dat").unwrap());

  for track in &file.sounds {
    inp.add(Track::new(track,&file.properties,false,&mut instruments));
  }

  for track in &file.drums {
    inp.add(Track::new(track,&file.properties,true,&mut instruments));
  }

  out
}