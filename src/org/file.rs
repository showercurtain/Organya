// Organya file specs in structs
// https://gist.github.com/fdeitylink/7fc9ddcc54b33971e5f505c8da2cfd28
use std::str;

#[allow(dead_code)]
pub struct OrgProps {
  version: String,
  pub click: u16,
  steps_per_bar: u8,
  beats_per_step: u8,
  pub loop_start: u32,
  pub loop_end: u32,
}

#[derive(Clone)]
pub struct OrgTrack {
  pub pitch: u16,
  pub instrument: u8,
  pub pi: bool,
  pub num_notes: u16,
  pub notes: Vec<OrgNote>,
}

#[derive(Clone)]
pub struct OrgNote {
  pub position: u32,
  pub note: u8,
  pub length: u8,
  pub volume: u8,
  pub pan: u8,
  pub complete: bool,
}

impl Default for OrgNote {
  fn default() -> Self {
    Self {
      position: 0,
      note: 0,
      length: 0,
      volume: 0,
      pan: 0,
      complete: false,
    }
  }
}

pub struct OrgFile {
  // File Properties
  pub properties: OrgProps,
  // Instruments
  pub sounds: [OrgTrack; 8],
  pub drums: [OrgTrack; 8],
}

impl OrgNote {
  fn new(data: &Vec<u8>, instrument: &mut OrgTrack, start: usize) -> usize {
    let mut ptr = start;
    let out = &mut instrument.notes;
    for i in 0..instrument.num_notes {
      out.push(Self::default());
      out[i as usize].position = u32::from_le_bytes(data[ptr..ptr+4].try_into().unwrap());
      ptr += 4;
    }
    let mut tmp: u8 = 0;
    for i in 0..instrument.num_notes {
      if data[ptr] == 255 {
        out[i as usize].note = tmp
      }
      else {
        out[i as usize].note = data[ptr];
        tmp = data[ptr];
      }
      ptr += 1;
    }
    tmp = 0;
    for i in 0..instrument.num_notes {
      if data[ptr] == 255 {
        out[i as usize].length = tmp
      }
      else {
        out[i as usize].length = data[ptr];
        tmp = data[ptr];
      }
      ptr += 1;
    }
    tmp = 0;
    for i in 0..instrument.num_notes {
      if data[ptr] == 255 {
        out[i as usize].volume = tmp
      }
      else {
        out[i as usize].volume = data[ptr];
        tmp = data[ptr];
      }
      ptr += 1;
    }
    tmp = 0;
    for i in 0..instrument.num_notes {
      if data[ptr] == 255 {
        out[i as usize].pan = tmp
      }
      else {
        out[i as usize].pan = data[ptr];
        tmp = data[ptr];
      }
      out[i as usize].complete = true;
      ptr += 1;
    }

    for i in 0..out.len() {
      if i == 0 {continue}
      if out[i].position < out[i-1].position + out[i-1].length as u32 {
        out[i].length = ((out[i-1].position + out[i-1].length as u32) - out[i].position) as u8;
        out[i-1].length = (out[i].position - out[i-1].position) as u8;
      }
    }

    ptr
  }
}

impl OrgProps {
  fn new(data: &Vec<u8>,start: usize) -> (Self,usize) {
    (
      Self {
        version: String::from(str::from_utf8(&data[0..6]).unwrap()), // Why would this ever not work?
        click: u16::from_le_bytes([data[start+6],data[start+7]]),
        steps_per_bar: data[start+8],
        beats_per_step: data[start+9],
        loop_start: u32::from_le_bytes(data[start+10..start+14].try_into().unwrap()),
        loop_end: u32::from_le_bytes(data[start+14..start+18].try_into().unwrap()),
      },
      start+18
    )
  }
}

impl OrgTrack {
  fn new(data: &Vec<u8>, start: usize) -> (Self,usize) {
    (
      Self {
        pitch: u16::from_le_bytes(data[start..start+2].try_into().unwrap()),
        instrument: data[start+2],
        pi: data[start+3] != 0,
        notes: vec![],
        num_notes: u16::from_le_bytes(data[start+4..start+6].try_into().unwrap())
      },
      start+6
    )
  }
}

impl Default for OrgTrack {
  fn default() -> Self {
    Self {
      pitch: 0,
      instrument: 0,
      pi: false,
      notes: vec![],
      num_notes: 0,
    }
  }
}

impl OrgFile {
  pub fn new(org_data: &Vec<u8>) -> Self {
    // Properties
    let mut ptr: usize = 0;
    let properties: OrgProps;
    (properties,ptr) = OrgProps::new(org_data,ptr);
    // Instruments
    let mut sounds: [OrgTrack; 8] = [
      OrgTrack::default(),OrgTrack::default(),OrgTrack::default(),OrgTrack::default(),
      OrgTrack::default(),OrgTrack::default(),OrgTrack::default(),OrgTrack::default(),
    ];
    let mut tmp: OrgTrack;
    for i in 0..8 {
      (tmp,ptr) = OrgTrack::new(org_data,ptr);
      sounds[i] = tmp;
    }
    let mut drums: [OrgTrack; 8] = [
      OrgTrack::default(),OrgTrack::default(),OrgTrack::default(),OrgTrack::default(),
      OrgTrack::default(),OrgTrack::default(),OrgTrack::default(),OrgTrack::default(),
    ];
    let mut tmp: OrgTrack;
    for i in 0..8 {
      (tmp,ptr) = OrgTrack::new(org_data,ptr);
      drums[i] = tmp;
    }
    for sound in sounds.iter_mut() {
      ptr = OrgNote::new(org_data,sound, ptr);
    }
    for drum in drums.iter_mut() {
      ptr = OrgNote::new(org_data,drum, ptr);
    }
    Self {
      properties,
      sounds,
      drums
    }
  }
}