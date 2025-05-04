use audio::decode;

fn main() {
    let result = decode("audio.mp3");
    match result {
        Ok(v) => println!("v: {:#?}", v),
        Err(e) => eprintln!("e: {}", e),
    }
}
