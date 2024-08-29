use serde::Deserialize;
use crate::yaml::config::Config;
use crate::yaml::{QemuArgs, SwtpmArgs};

#[derive(Debug,Deserialize)]
pub struct System {
    chipset: String,
    bios: Bios,
    cpu: Cpu,
    memory: Memory,
    tpm: Option<Tpm>
}
impl System {
    pub fn get_tpm(&self) -> Option<Tpm> {
        self.tpm.clone()
    }
}

const PVE_CONFIG_FILE: &str = "/usr/share/ezkvm/pve-q35-4.0.cfg";
const OVMF_BIOS_FILE: &str = "/usr/share/ezkvm/OVMF_CODE.secboot.4m.fd";
const BOOT_SPLASH_FILE: &str = "/usr/share/ezkvm/bootsplash.jpg";

impl SwtpmArgs for System {
    fn get_swtpm_args(&self, index: usize) -> Vec<String> {
        let mut result = vec![];
        if let Some(tpm) = &self.tpm {
            result.extend(tpm.get_swtpm_args(0));
        }
        result
    }
}

impl QemuArgs for System {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        let mut result = vec![];
        match self.chipset.as_str() {
            "q35" => {
                result.extend(vec![
                    "-machine hpet=off,type=pc-q35-8.1".to_string(),
                    "-rtc driftfix=slew,base=localtime".to_string(),
                    "-global kvm-pit.lost_tick_policy=discard".to_string(),
                    format!("-readconfig {}",PVE_CONFIG_FILE),
                    "-device qemu-xhci,p2=15,p3=15,id=xhci,bus=pci.1,addr=0x1b".to_string(),
                    "-iscsi initiator-name=iqn.1993-08.org.debian:01:39407ad058b".to_string(),
                    "-device pvscsi,id=scsihw0,bus=pci.0,addr=0x5".to_string(),
                    //"-device ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc".to_string(),
                    //"-device hda-micro,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=spice-backend0".to_string(),
                    //"-device hda-duplex,id=audiodev0-codec1,bus=audiodev0.0,cad=1,audiodev=spice-backend0".to_string(),

                    // -device ich9-intel-hda,id=sound0,bus=pcie.0,addr=0x1b \
                    // -device hda-duplex,id=sound0-codec0,bus=sound0.0,cad=0 \
                    // -global ICH9-LPC.disable_s3=1 -global ICH9-LPC.disable_s4=1

                ]);
            }
            "i440fx" => {
                todo!()
            }
            _ => { panic!("{}", format!("Unsupported chipset {}", self.chipset)); }
        }

        result.extend(self.bios.get_qemu_args(0));
        result.extend(self.cpu.get_qemu_args(0));
        result.extend(self.memory.get_qemu_args(0));

        if let Some(tpm) = &self.tpm {
            result.extend(tpm.get_qemu_args(0));
        }

        result
    }
}

impl Default for System {
    fn default() -> Self {
        Self {
            bios: Bios::default(),
            chipset: "q35".to_string(),
            cpu: Cpu::default(),
            memory: Memory::default(),
            tpm: None
        }
    }
}

#[derive(Debug,Deserialize)]
struct Bios {
    model: String,
    uuid: Option<String>,
    disk: Option<String>
}

impl QemuArgs for Bios {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        match self.model.as_str() {
            "ovmf" => {
                vec![
                    format!("-boot menu=on,strict=on,reboot-timeout=1000,splash={}", BOOT_SPLASH_FILE),
                    format!("-smbios type=1,uuid={}",self.uuid.clone().unwrap()),
                    format!("-drive if=pflash,unit=0,format=raw,readonly=on,file={}",OVMF_BIOS_FILE),
                    format!("-drive if=pflash,unit=1,id=drive-efidisk0,format=raw,file={},size=540672",self.disk.clone().unwrap()),
                ]
            },
            "seabios" => {
                vec![
                    format!("-boot menu=on,strict=on,reboot-timeout=1000,splash={}", BOOT_SPLASH_FILE),
                    format!("-smbios type=1,uuid={}",self.uuid.clone().unwrap()),
                ]
            }
            _ => {
                vec![
                    // no bios defined, but qemu will still fallback to seabios
                    format!("-boot menu=on,strict=on,reboot-timeout=1000,splash={}", BOOT_SPLASH_FILE),
                ]
            }
        }
    }
}

impl Default for Bios {
    fn default() -> Self {

        Self {
            model: "seabios".to_string(),
            uuid: None,
            disk: None
        }
    }
}

#[derive(Debug,Deserialize)]
struct Cpu {
    model: String,
    flags: String,
    sockets: u16,
    cores: u16
}
impl QemuArgs for Cpu {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        let total = self.sockets * self.cores;
        vec![
            format!("-smp {},sockets={},cores={},maxcpus={}", total,self.sockets,self.cores,total),
            format!("-cpu {},{}", self.model,self.flags),
        ]
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            model: "qemu64".to_string(),
            flags: "hv_vendor_id=ezkvm".to_string(),
            sockets: 1,
            cores: 4
        }
    }
}

#[derive(Debug,Deserialize)]
struct Memory {
    max: u64,
    balloon:bool
}
impl QemuArgs for Memory {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        vec![
            format!("-m {}", self.max),
        ]
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            max: 4096,
            balloon: false
        }
    }
}


#[derive(Debug,Deserialize,PartialEq,Clone)]
pub struct Tpm {
    model: String,
    version: Option<f32>,
    disk: Option<String>,
    socket: Option<String>
}

impl SwtpmArgs for Tpm {
    fn get_swtpm_args(&self, index: usize) -> Vec<String> {
        match self.model.as_str() {
            "passthrough" => {
                todo!()
            },
            "swtpm" => {
                if let Some(socket_path) = self.socket.clone() {
                    if let Some(tpmstate_path) = self.disk.clone() {
                        vec![
                            "socket".to_string(),
                            "--tpmstate".to_string(),
                            format!("backend-uri=file://{},mode=0600", tpmstate_path),
                            "--ctrl".to_string(),
                            format!("type=unixio,path={},mode=0600", socket_path),
                            "--tpm2".to_string()
                        ]
                    } else { vec![] }
                } else { vec![] }
            },
            _ => vec![]
        }
    }
}
impl QemuArgs for Tpm {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        match self.model.as_str() {
            "none" => vec![],
            "passthrough" => {
                todo!()
            },
            "swtpm" => vec![
                format!("-chardev socket,id=chrtpm{},path={}", index, self.socket.clone().unwrap()),
                format!("-tpmdev emulator,id=tpm{},chardev=chrtpm{}", index, index),
                format!("-device tpm-tis,tpmdev=tpm{}", index),
            ],
            _ => vec![]
        }
    }
}

impl Default for Tpm {
    fn default() -> Self {
        Self {
            model: "none".to_string(),
            version: None,
            disk: None,
            socket: None
        }
    }
}
