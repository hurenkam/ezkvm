use std::{thread, time::Duration};

fn main() {
    let _ = std::env::args().collect::<Vec<_>>();
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
