mod display;
mod general;
mod gpu;
mod host;
mod network;
mod spice;
mod storage;
mod system;
mod types;

use crate::config::display::Display;
use crate::config::gpu::Gpu;
use derive_getters::Getters;
use log::{debug, info};
use serde::{Deserialize, Deserializer};
use std::any::{Any, TypeId};
use std::ops::Deref;

use crate::config::network::Network;
use crate::config::storage::Storage;
#[mockall_double::double]
use crate::osal::Osal;
use crate::osal::OsalError;
pub use display::Gtk;
pub use general::General;
pub use host::Host;
pub use spice::Spice;
pub use system::System;
pub use types::Pci;
pub use types::QemuDevice;
pub use types::Usb;

#[macro_export]
macro_rules! optional_value_getter {
    ($id: ident($lit: literal): $ty: ty) => {
        pub fn $id(&self) -> String {
            match &self.$id {
                None => "".to_string(),
                Some(option) => {
                    format!(",{}={}", $lit, option.clone())
                }
            }
        }
    };
}

#[macro_export]
macro_rules! required_value_getter {
    ($id: ident($lit: literal): $ty: ty = $default: expr) => {
        paste! {
            pub fn $id(&self) -> String {
                format!(",{}={}", $lit, self.$id)
            }
            pub fn [<$id _default>]() -> $ty {
                $default
            }
        }
    };
}

#[derive(Deserialize, Debug, Default, Getters)]
pub struct Config {
    #[serde(default, deserialize_with = "default_when_missing")]
    general: General,
    #[serde(default, deserialize_with = "default_when_missing")]
    system: System,
    #[serde(default, deserialize_with = "default_when_missing")]
    display: Box<dyn Display>,
    #[serde(default, deserialize_with = "default_when_missing")]
    gpu: Box<dyn Gpu>,
    #[serde(default, deserialize_with = "default_when_missing")]
    spice: Option<Spice>,
    #[serde(default, deserialize_with = "default_when_missing")]
    host: Option<Host>,
    #[serde(default, deserialize_with = "default_when_missing")]
    storage: Vec<Box<dyn Storage>>,
    #[serde(default, deserialize_with = "default_when_missing")]
    network: Vec<Box<dyn Network>>,
    #[serde(default, deserialize_with = "default_when_missing")]
    extras: Vec<String>,
}

impl Config {
    pub(crate) fn allocate_resources(&self) -> Result<Vec<String>, OsalError> {
        Ok(vec![])
    }

    fn has_gtk_display_configured(&self) -> bool {
        self.get_gtk_display().is_some()
    }

    fn get_gtk_display(&self) -> Option<Gtk> {
        let display = self.display();

        // make sure display contains an &Box<Gtk> instance
        if (*display).type_id() == TypeId::of::<Box<dyn Display>>()
            && (*display.deref()).type_id() == TypeId::of::<Gtk>()
        {
            // since we now know display is an &Box<Gtk>, we can do a cast to it
            let t = unsafe { &*(display as *const dyn Any as *const Box<Gtk>) };

            // and return a clone wrapped in an Option<>
            Some(t.deref().clone())
        } else {
            // display is not an &Box<Gtk>, so return None
            None
        }
    }

    pub fn get_escalated_uid_and_gid(&self) -> (u32, u32) {
        // if gtk display is enabled, qemu (and swtpm) can not be run with escalated privileges
        // so in this case return default uid/gid instead
        if self.has_gtk_display_configured() {
            debug!(
                "get_escalated_uid_and_gid(): Gtk display configured, dropping to default uid/gid"
            );
            return self.get_default_uid_and_gid();
        }

        // otherwise return escalated privileges if available
        Osal::get_euid_and_egid()
    }

    pub fn get_default_uid_and_gid(&self) -> (u32, u32) {
        // return actual uid/gid instead of euid/egid
        Osal::get_uid_and_gid()
    }
}

impl QemuDevice for Config {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![];
        result.extend(self.general.get_qemu_args(0));
        result.extend(self.system.get_qemu_args(0));
        result.extend(self.display.get_qemu_args(0));
        result.extend(self.gpu.get_qemu_args(0));

        match self.spice() {
            None => {}
            Some(spice) => {
                result.extend(spice.get_qemu_args(0));
            }
        }

        match self.host() {
            None => {}
            Some(host) => {
                result.extend(host.get_qemu_args(0));
            }
        }

        for (i, disk) in self.storage.iter().enumerate() {
            result.extend(disk.get_qemu_args(i));
        }

        for (i, network) in self.network.iter().enumerate() {
            result.extend(network.get_qemu_args(i));
        }

        let mut args = "qemu-system-x86_64".to_string();
        for arg in result {
            info!("{}", arg);
            args = format!("{} {}", args, arg).to_string();
        }

        args.split_whitespace().map(str::to_string).collect()
    }

    fn pre_start(&self, config: &Config) {
        self.system.pre_start(config);
    }

    fn post_start(&self, config: &Config) {
        self.system.post_start(config);
        self.display.post_start(config);
    }
}

pub fn default_when_missing<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    let option = Option::deserialize(deserializer)?;
    Ok(option.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_empty_config() {
        let config: Config = serde_yaml::from_str(
            r#"
              "#,
        )
        .unwrap();

        let tmp = config.get_qemu_args(0);
        let actual: Vec<&str> = tmp.iter().map(std::ops::Deref::deref).collect();
        let expected: Vec<&str> = vec![
            "qemu-system-x86_64",
            "-accel",
            "kvm",
            "-nodefaults",
            "-monitor",
            "unix:/var/ezkvm/anonymous.monitor,server,nowait",
            "-chardev",
            "socket,id=qmp,path=/var/ezkvm/anonymous.qmp,server=on,wait=off",
            "-mon",
            "chardev=qmp,mode=control",
            "-chardev",
            "socket,id=qmp-event,path=/var/run/qmeventd.sock,reconnect=5",
            "-mon",
            "chardev=qmp-event,mode=control",
            "-machine",
            "hpet=off,type=pc-q35-8.1",
            "-rtc",
            "driftfix=slew,base=localtime",
            "-global",
            "kvm-pit.lost_tick_policy=discard",
            "-readconfig",
            "/usr/share/ezkvm/pve-q35-4.0.cfg",
            "-device",
            "qemu-xhci,p2=15,p3=15,id=xhci,bus=pci.1,addr=0x1b",
            "-iscsi",
            "initiator-name=iqn.1993-08.org.debian:01:39407ad058b",
            "-device",
            "pvscsi,id=scsihw0,bus=pci.0,addr=0x5",
            "-boot",
            "menu=on,strict=on,reboot-timeout=1000,splash=/usr/share/ezkvm/bootsplash.jpg",
            "-smbios",
            "type=1,uuid=",
            "-m",
            "16384",
            "-smp",
            "4,sockets=1,cores=4,maxcpus=4",
            "-cpu",
            "qemu64,+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce",
        ];
        assert_eq!(actual, expected);
    }

    const DEFAULT_WINDOWS_CONFIG: &str = r#"
        general:
            name: wakiza

        system:
            bios: { type: "ovmf", uuid: "04d064c3-66a1-4aa7-9589-f8b3ecf91cd7", file: "/dev/vm1/vm-108-efidisk" }
            cpu: { model: "qemu64", sockets: 1, cores: 8, flags: "+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce" }
            memory: { max: 16384, balloon: false }
            tpm: { type: "swtpm", version: 2.0, disk: "/dev/vm1/vm-108-tpmstate", socket: "/var/ezkvm/wakiza-tpm.socket" }

        spice:
            port: 5903
            addr: 0.0.0.0

        gpu:
            type: "passthrough"
            pci:
            - { vm_id: "0.0", host_id: "0000:03:00.0", multi_function: true }
            - { vm_id: "0.1", host_id: "0000:03:00.1" }

        display:
            type: "looking_glass"
            device: { path: /dev/kvmfr0, size: 128M }
            input: { grab_keyboard: true, escape_key: KEY_F12 }
            window: { size: 1707x1067, full_screen: true }

        host:
            usb:
            - { vm_port: "1", host_bus: "1", host_port: "2.2" }

        storage:
        - { type: "scsi-hd", file: "/dev/vm1/vm-108-boot", discard: "on", boot_index: 0 }
        - { type: "scsi-hd", file: "/dev/vm1/vm-108-tmp", discard: "on" }

        network:
        - { type: "bridge", bridge: "vmbr0", driver: "virtio-net-pci", mac: "BC:24:11:3A:21:B7" }
    "#;

    #[test]
    fn test_windows_defaults() {
        let config: Config = serde_yaml::from_str(DEFAULT_WINDOWS_CONFIG).unwrap();

        let tmp = config.get_qemu_args(0);
        let actual: Vec<&str> = tmp.iter().map(std::ops::Deref::deref).collect();
        let expected: Vec<&str> = vec![
            "qemu-system-x86_64",
            "-accel", "kvm", "-nodefaults",
            "-monitor", "unix:/var/ezkvm/wakiza.monitor,server,nowait",
            "-chardev", "socket,id=qmp,path=/var/ezkvm/wakiza.qmp,server=on,wait=off",
            "-mon", "chardev=qmp,mode=control",
            "-chardev", "socket,id=qmp-event,path=/var/run/qmeventd.sock,reconnect=5",
            "-mon", "chardev=qmp-event,mode=control",
            "-machine", "hpet=off,type=pc-q35-8.1",
            "-rtc", "driftfix=slew,base=localtime",
            "-global", "kvm-pit.lost_tick_policy=discard",
            "-readconfig", "/usr/share/ezkvm/pve-q35-4.0.cfg",
            "-device", "qemu-xhci,p2=15,p3=15,id=xhci,bus=pci.1,addr=0x1b",
            "-iscsi", "initiator-name=iqn.1993-08.org.debian:01:39407ad058b",
            "-device", "pvscsi,id=scsihw0,bus=pci.0,addr=0x5",
            "-boot", "menu=on,strict=on,reboot-timeout=1000,splash=/usr/share/ezkvm/bootsplash.jpg",
            "-smbios", "type=1,uuid=04d064c3-66a1-4aa7-9589-f8b3ecf91cd7",
            "-drive", "if=pflash,unit=0,format=raw,readonly=on,file=/usr/share/ezkvm/OVMF_CODE.secboot.4m.fd",
            "-drive", "if=pflash,unit=1,id=drive-efidisk0,format=raw,file=/dev/vm1/vm-108-efidisk,size=540672",
            "-m", "16384",
            "-smp", "8,sockets=1,cores=8,maxcpus=8",
            "-cpu", "qemu64,+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce",
            "-chardev", "socket,id=chrtpm0,path=/var/ezkvm/wakiza-tpm.socket",
            "-tpmdev", "emulator,id=tpm0,chardev=chrtpm0",
            "-device", "tpm-tis,tpmdev=tpm0",
            "-vga", "none", "-nographic",
            "-device", "virtio-mouse",
            "-device", "virtio-keyboard",
            "-device", "ivshmem-plain,memdev=ivshmem0,bus=pcie.0",
            "-object", "memory-backend-file,id=ivshmem0,share=on,mem-path=/dev/kvmfr0,size=128M",
            "-device", "vfio-pci,host=0000:03:00.0,id=hostpci0.0,bus=ich9-pcie-port-1,addr=0x0.0,multifunction=on",
            "-device", "vfio-pci,host=0000:03:00.1,id=hostpci0.1,bus=ich9-pcie-port-1,addr=0x0.1",
            "-spice", "port=5903,addr=0.0.0.0,disable-ticketing=on",
            "-device", "virtio-serial-pci",
            "-chardev", "spicevmc,id=vdagent,name=vdagent",
            "-device", "virtserialport,chardev=vdagent,name=com.redhat.spice.0",
            "-audiodev", "spice,id=spice-backend0",
            "-device", "ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc",
            "-device", "hda-duplex,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=spice-backend0",
            "-device", "usb-host,bus=xhci.0,port=1,hostbus=1,hostport=2.2,id=usb0",
            "-drive", "file=/dev/vm1/vm-108-boot,if=none,aio=io_uring,id=drive-scsi0,discard=on,format=raw,cache=none,detect-zeroes=unmap",
            "-device", "scsi-hd,scsi-id=0,drive=drive-scsi0,id=scsi0,bus=scsihw0.0,rotation_rate=1,bootindex=0",
            "-drive", "file=/dev/vm1/vm-108-tmp,if=none,aio=io_uring,id=drive-scsi1,discard=on,format=raw,cache=none,detect-zeroes=unmap",
            "-device", "scsi-hd,scsi-id=1,drive=drive-scsi1,id=scsi1,bus=scsihw0.0,rotation_rate=1",
            "-netdev", "id=hostnet0,type=bridge,br=vmbr0",
            "-device", "id=net0,driver=virtio-net-pci,netdev=hostnet0,mac=BC:24:11:3A:21:B7,bus=pci.1,addr=0x0"
        ];

        assert_eq!(actual, expected);
    }

    const DEFAULT_UBUNTU_CONFIG: &str = r#"
        general:
            name: gyndine

        system:
            bios: { type: "ovmf", uuid: "c0e240a5-859a-4378-a2d9-95088f531142", file: "/dev/vm1/vm-950-disk-0" }

        gpu:
            type: "virtio-vga-gl"
            memory: 256

        display:
            type: "gtk"
            gl: true

        storage:
        - { type: "scsi-hd", file: "/dev/vm1/vm-950-disk-1", boot_index: 1 }
        - { type: "ide-cd", file: "ubuntu.iso" }

        network:
        - { type: "bridge", mac: "BC:24:11:FF:76:89" }
    "#;

    #[test]
    fn test_ubuntu_defaults() {
        let config: Config = serde_yaml::from_str(DEFAULT_UBUNTU_CONFIG).unwrap();
        let tmp = config.get_qemu_args(0);
        let actual: Vec<&str> = tmp.iter().map(std::ops::Deref::deref).collect();
        let expected: Vec<&str> = vec![
            "qemu-system-x86_64",
            "-accel", "kvm",
            "-nodefaults",
            "-monitor", "unix:/var/ezkvm/gyndine.monitor,server,nowait",
            "-chardev", "socket,id=qmp,path=/var/ezkvm/gyndine.qmp,server=on,wait=off",
            "-mon", "chardev=qmp,mode=control",
            "-chardev", "socket,id=qmp-event,path=/var/run/qmeventd.sock,reconnect=5",
            "-mon", "chardev=qmp-event,mode=control",
            "-machine", "hpet=off,type=pc-q35-8.1",
            "-rtc", "driftfix=slew,base=localtime",
            "-global", "kvm-pit.lost_tick_policy=discard",
            "-readconfig", "/usr/share/ezkvm/pve-q35-4.0.cfg",
            "-device", "qemu-xhci,p2=15,p3=15,id=xhci,bus=pci.1,addr=0x1b",
            "-iscsi", "initiator-name=iqn.1993-08.org.debian:01:39407ad058b",
            "-device", "pvscsi,id=scsihw0,bus=pci.0,addr=0x5",
            "-boot", "menu=on,strict=on,reboot-timeout=1000,splash=/usr/share/ezkvm/bootsplash.jpg",
            "-smbios", "type=1,uuid=c0e240a5-859a-4378-a2d9-95088f531142",
            "-drive", "if=pflash,unit=0,format=raw,readonly=on,file=/usr/share/ezkvm/OVMF_CODE.secboot.4m.fd",
            "-drive", "if=pflash,unit=1,id=drive-efidisk0,format=raw,file=/dev/vm1/vm-950-disk-0,size=540672",
            "-m", "16384",
            "-smp", "4,sockets=1,cores=4,maxcpus=4",
            "-cpu", "qemu64,+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce",
            "-display", "gtk,gl=on",
            "-audiodev", "pipewire,id=audiodev0",
            "-device", "usb-tablet",
            "-device", "ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc",
            "-device", "hda-duplex,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=audiodev0",
            "-device", "virtio-vga-gl,id=vga,bus=pcie.0,addr=0x2",
            "-drive", "file=/dev/vm1/vm-950-disk-1,if=none,aio=io_uring,id=drive-scsi0,format=raw,cache=none,detect-zeroes=unmap",
            "-device", "scsi-hd,scsi-id=0,drive=drive-scsi0,id=scsi0,bus=scsihw0.0,rotation_rate=1,bootindex=1",
            "-drive", "file=ubuntu.iso,if=none,aio=io_uring,id=drive-ide1,media=cdrom",
            "-device", "ide-cd,bus=ide.1,drive=drive-ide1,id=ide1,unit=0",
            "-netdev", "id=hostnet0,type=bridge,br=vmbr0",
            "-device", "id=net0,driver=virtio-net-pci,netdev=hostnet0,mac=BC:24:11:FF:76:89,bus=pci.1,addr=0x0"
        ];

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_has_gtk_display_configured() {
        let config: Config = serde_yaml::from_str(DEFAULT_UBUNTU_CONFIG).unwrap();
        assert!(config.has_gtk_display_configured())
    }

    #[test]
    fn test_has_no_gtk_display_configured() {
        let config: Config = serde_yaml::from_str(DEFAULT_WINDOWS_CONFIG).unwrap();
        assert_eq!(config.has_gtk_display_configured(), false)
    }

    #[test]
    #[serial]
    fn test_get_escalated_uid_and_gid_returns_defaults_when_gtk_display_configured() {
        let config: Config = serde_yaml::from_str(DEFAULT_UBUNTU_CONFIG).unwrap();

        let get_euid_and_egid_context = Osal::get_euid_and_egid_context();
        get_euid_and_egid_context.expect().returning(|| (0, 0));

        let get_uid_and_gid_context = Osal::get_uid_and_gid_context();
        get_uid_and_gid_context.expect().returning(|| (1000, 1000));

        let actual = config.get_escalated_uid_and_gid();
        let expected: (u32, u32) = (1000, 1000);
        assert_eq!(actual, expected)
    }

    #[test]
    #[serial]
    fn test_get_escalated_uid_and_gid_returns_escalated_when_gtk_display_not_configured() {
        let config: Config = serde_yaml::from_str(DEFAULT_WINDOWS_CONFIG).unwrap();

        let get_euid_and_egid_context = Osal::get_euid_and_egid_context();
        get_euid_and_egid_context.expect().returning(|| (0, 0));

        let get_uid_and_gid_context = Osal::get_uid_and_gid_context();
        get_uid_and_gid_context.expect().returning(|| (1000, 1000));

        let actual = config.get_escalated_uid_and_gid();
        let expected: (u32, u32) = (0, 0);
        assert_eq!(actual, expected)
    }
}
