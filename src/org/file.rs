// Organya file specs in structs
// https://gist.github.com/fdeitylink/7fc9ddcc54b33971e5f505c8da2cfd28
use std::str;

pub struct OrgProps {
  version: String,
  pub tempo: u16,
  steps_per_bar: u8,
  beats_per_step: u8,
  pub loop_start: u32,
  pub loop_end: u32,
}

#[derive(Clone, Copy)]
pub struct OrgInstrument {
  pub pitch: u16,
  pub instrument: u8,
  pub pi: bool,
  pub num_notes: u16,
}

pub struct OrgNote {
  pub position: u32,
  pub note: u8,
  pub length: u8,
  pub volume: u8,
  pub pan: u8,
  pub instrument: u8,
  pub drum: bool,
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
      instrument: 0,
      drum: false,
      complete: false,
    }
  }
}

pub struct OrgFile {
  // File Properties
  pub properties: OrgProps,
  // Instruments
  pub sounds: [OrgInstrument; 8],
  pub drums: [OrgInstrument; 8],
  // The notes
  pub notes: Vec<OrgNote>,
}

impl OrgNote {
  fn new(data: &Vec<u8>, numnotes: usize, instrument: u8, drum: bool, start: usize) -> (Vec<Self>,usize) {
    let mut ptr = start;
    let mut out: Vec<Self> = vec![];
    for i in 0..numnotes {
      out.push(Self::default());
      out[i].instrument = instrument;
      out[i].drum = drum;
      out[i].position = u32::from_le_bytes(data[ptr..ptr+4].try_into().unwrap());
      ptr += 4;
    }
    let mut tmp: u8 = 0;
    for i in 0..numnotes {
      if data[ptr] == 255 {
        out[i].note = tmp
      }
      else {
        out[i].note = data[ptr];
        tmp = data[ptr];
      }
      ptr += 1;
    }
    tmp = 0;
    for i in 0..numnotes {
      if data[ptr] == 255 {out[i].length = tmp}
      else {
        out[i].length = data[ptr];
        tmp = data[ptr];
      }
      ptr += 1;
    }
    tmp = 0;
    for i in 0..numnotes {
      if data[ptr] == 255 {out[i].volume = tmp}
      else {
        out[i].volume = data[ptr];
        tmp = data[ptr];
      }
      ptr += 1;
    }
    tmp = 0;
    for i in 0..numnotes {
      if data[ptr] == 255 {out[i].pan = tmp}
      else {
        out[i].pan = data[ptr];
        tmp = data[ptr];
      }
      out[i].complete = true;
      ptr += 1;
    }

    for i in 0..out.len() {
      if i == 0 {continue}
      if out[i].position < out[i-1].position + out[i-1].length as u32 {
        out[i].length = ((out[i-1].position + out[i-1].length as u32) - out[i].position) as u8;
        out[i-1].length = (out[i].position - out[i-1].position) as u8;
      }
    }

    (out,ptr)
  }
}

impl OrgProps {
  fn new(data: &Vec<u8>,start: usize) -> (Self,usize) {
    (
      Self {
        version: String::from(str::from_utf8(&data[0..6]).unwrap()), // Why would this ever not work?
        tempo: u16::from_le_bytes([data[start+6],data[start+7]]),
        steps_per_bar: data[start+8],
        beats_per_step: data[start+9],
        loop_start: u32::from_le_bytes(data[start+10..start+14].try_into().unwrap()),
        loop_end: u32::from_le_bytes(data[start+14..start+18].try_into().unwrap()),
      },
      start+18
    )
  }
}

impl OrgInstrument {
  fn new(data: &Vec<u8>, start: usize) -> (Self,usize) {
    (
      Self {
        pitch: u16::from_le_bytes(data[start..start+2].try_into().unwrap()),
        instrument: data[start+2],
        pi: data[start+3] != 0,
        num_notes: u16::from_le_bytes(data[start+4..start+6].try_into().unwrap())
      },
      start+6
    )
  }
}

impl Default for OrgInstrument {
  fn default() -> Self {
    Self {
      pitch: 0,
      instrument: 0,
      pi: false,
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
    let mut sounds: [OrgInstrument; 8] = [OrgInstrument::default(); 8];
    let mut tmp: OrgInstrument;
    for i in 0..8 {
      (tmp,ptr) = OrgInstrument::new(org_data,ptr);
      sounds[i] = tmp;
    }
    let mut drums: [OrgInstrument; 8] = [OrgInstrument::default(); 8];
    let mut tmp: OrgInstrument;
    for i in 0..8 {
      (tmp,ptr) = OrgInstrument::new(org_data,ptr);
      drums[i] = tmp;
    }
    let mut notes: Vec<OrgNote> = vec![];
    let mut tmp: Vec<OrgNote>;
    for (i,sound) in sounds.iter().enumerate() {
      notes.append(
        {
          (tmp,ptr) = OrgNote::new(org_data,sound.num_notes as usize,i as u8,false,ptr);
          &mut tmp
        }
      )
    }
    for (i,drum) in drums.iter().enumerate() {
      notes.append(
        {
          (tmp,ptr) = OrgNote::new(org_data,drum.num_notes as usize,i as u8,true,ptr);
          &mut tmp
        }
      )
    }
    Self {
      properties,
      sounds,
      drums,
      notes
    }
  }
}