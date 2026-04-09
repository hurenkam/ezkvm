use super::QemuDevice;
use serde::Deserialize;

/// Virtio-RNG device backed by the host's random number generator.
///
/// YAML example:
/// ```yaml
/// system:
///   virtio_rng: {}           # uses /dev/urandom (default)
/// ```
/// Or with custom source:
/// ```yaml
/// system:
///   virtio_rng:
///     filename: /dev/random
/// ```
#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct VirtioRng {
    #[serde(default = "VirtioRng::filename_default")]
    filename: String,
}

impl Default for VirtioRng {
    fn default() -> Self {
        Self {
            filename: Self::filename_default(),
        }
    }
}

impl VirtioRng {
    fn filename_default() -> String {
        "/dev/urandom".to_string()
    }
}

impl QemuDevice for VirtioRng {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        vec![
            format!(
                "-object rng-random,id=rng0,filename={}",
                self.filename
            ),
            "-device virtio-rng-pci,rng=rng0".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let rng = VirtioRng::default();
        let args = rng.get_qemu_args(0);
        assert_eq!(
            args,
            vec![
                "-object rng-random,id=rng0,filename=/dev/urandom".to_string(),
                "-device virtio-rng-pci,rng=rng0".to_string(),
            ]
        );
    }

    #[test]
    fn test_custom_filename() {
        let yaml = "filename: /dev/random";
        let rng: VirtioRng = serde_yaml::from_str(yaml).unwrap();
        let args = rng.get_qemu_args(0);
        assert_eq!(
            args,
            vec![
                "-object rng-random,id=rng0,filename=/dev/random".to_string(),
                "-device virtio-rng-pci,rng=rng0".to_string(),
            ]
        );
    }

    #[test]
    fn test_from_empty_yaml() {
        let rng: VirtioRng = serde_yaml::from_str("{}").unwrap();
        assert_eq!(rng.filename, "/dev/urandom");
    }
}
