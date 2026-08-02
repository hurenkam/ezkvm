use std::{
    os::unix::net::UnixListener,
    path::PathBuf,
    thread,
    time::Duration,
};

fn main() {
    let socket_path = ctrl_socket_path().expect("expected path=<socket_path> in --ctrl args");
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path).expect("bind fake swtpm socket");

    for stream in listener.incoming() {
        match stream {
            Ok(_) => {}
            Err(_) => break,
        }
    }

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

fn ctrl_socket_path() -> Option<PathBuf> {
    std::env::args().find_map(|arg| {
        arg.split(',')
            .find_map(|segment| segment.strip_prefix("path=").map(PathBuf::from))
    })
}
