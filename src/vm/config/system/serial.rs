use super::QemuDevice;
use serde::Deserialize;

#[derive(Deserialize, PartialEq, Debug, Clone)]
#[serde(tag = "type")]
pub enum Serial {
    #[serde(rename = "socket")]
    Socket { path: String },
}

impl QemuDevice for Serial {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        match self {
            Serial::Socket { path } => vec![
                format!(
                    "-chardev socket,id=charserial0,path={},server=on,wait=off",
                    path
                ),
                "-device isa-serial,chardev=charserial0".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_serial_qemu_args() {
        let serial = Serial::Socket {
            path: "/var/run/qemu-server/301.serial0".to_string(),
        };
        let expected = vec![
            "-chardev socket,id=charserial0,path=/var/run/qemu-server/301.serial0,server=on,wait=off"
                .to_string(),
            "-device isa-serial,chardev=charserial0".to_string(),
        ];
        assert_eq!(serial.get_qemu_args(0), expected);
    }

    #[test]
    fn test_socket_serial_from_yaml() {
        let yaml = r#"
            type: socket
            path: /var/run/qemu-server/301.serial0
        "#;
        let serial: Serial = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            serial,
            Serial::Socket {
                path: "/var/run/qemu-server/301.serial0".to_string()
            }
        );
    }
}
