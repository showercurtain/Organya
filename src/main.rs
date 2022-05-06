#[allow(dead_code)]
mod org;
use rodio::{OutputStream,Sink};


fn main() {
    let args: Vec<String> = std::env::args().collect();
    let player = org::play::Player::load(&args[1]);
    let (_stream, handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&handle).unwrap();
    sink.append(player);
    sink.sleep_until_end();
}
