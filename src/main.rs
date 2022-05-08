mod org;
use rodio::{OutputStream,Sink};
use std::io::Read;


fn main() {
    let args: Vec<String> = std::env::args().collect();
    // let player = org::play::Player::load(&args[1]);
    let (_stream, handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&handle).unwrap();
    let mut data: Vec<u8> = vec![];
    {
        let mut f = std::fs::File::open(&args[1]).unwrap();
        f.read_to_end(&mut data).unwrap();
    }
    let org = org::file::OrgFile::new(&data);
    sink.append(org::play::get_mixer(org));
    sink.sleep_until_end();

}
