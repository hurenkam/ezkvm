use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
    thread,
    time::Duration,
};

fn main() {
    if let Some(path) = qmp_socket_path() {
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).expect("bind fake qmp socket");
        thread::spawn(move || serve(listener));
    }

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

fn qmp_socket_path() -> Option<PathBuf> {
    let args = std::env::args().collect::<Vec<_>>();
    for (idx, arg) in args.iter().enumerate() {
        if let Some(rest) = arg.strip_prefix("unix:") {
            return Some(PathBuf::from(rest.split(',').next().unwrap_or(rest)));
        }
        if arg == "-qmp" {
            if let Some(next) = args.get(idx + 1) {
                if let Some(rest) = next.strip_prefix("unix:") {
                    return Some(PathBuf::from(rest.split(',').next().unwrap_or(rest)));
                }
            }
        }
    }
    None
}

fn serve(listener: UnixListener) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_client(stream),
            Err(_) => break,
        }
    }
}

fn handle_client(mut stream: UnixStream) {
    let _ = writeln!(stream, r#"{{"QMP":{{"version":{{"qemu":{{"major":8,"minor":0,"micro":0}},"package":"fake"}},"capabilities":[]}}}}"#);
    let reader_stream = match stream.try_clone() {
        Ok(reader) => reader,
        Err(_) => return,
    };
    let mut reader = BufReader::new(reader_stream);
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
                    continue;
                };
                let command = value.get("execute").and_then(|v| v.as_str()).unwrap_or_default();
                let _ = writeln!(stream, r#"{{"return":{{}}}}"#);
                match command {
                    "qmp_capabilities" => {}
                    "system_powerdown" => {
                        if std::env::var_os("FAKE_QEMU_IGNORE_POWERDOWN").is_none() {
                            std::process::exit(0);
                        }
                    }
                    "quit" => {
                        if std::env::var_os("FAKE_QEMU_IGNORE_QUIT").is_none() {
                            std::process::exit(0);
                        }
                    }
                    "system_reset" => {}
                    _ => {}
                }
            }
        }
    }
}
