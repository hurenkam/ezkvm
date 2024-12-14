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
use log::debug;
use serde::{Deserialize, Deserializer};
use std::any::{Any, TypeId};
use std::ops::Deref;

use crate::config::network::NetworkItem;
use crate::config::storage::StorageItem;
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
        #[allow(dead_code)]
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
            #[allow(dead_code)]
            pub fn $id(&self) -> String {
                format!(",{}={}", $lit, self.$id)
            }
            #[allow(dead_code)]
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
    #[serde(default)]
    storage: Vec<StorageItem>,
    #[serde(default)]
    network: Vec<NetworkItem>,
    #[serde(default, deserialize_with = "default_when_missing")]
    extras: Vec<String>,
}

impl Config {
    pub fn with_general(self, general: General) -> Config {
        Config { general, ..self }
    }

    pub fn with_system(self, system: System) -> Config {
        Config { system, ..self }
    }

    pub fn with_display(self, display: Box<dyn Display>) -> Config {
        Config { display, ..self }
    }

    pub fn with_gpu(self, gpu: Box<dyn Gpu>) -> Config {
        Config { gpu, ..self }
    }

    pub fn with_spice(self, spice: Option<Spice>) -> Config {
        Config { spice, ..self }
    }

    pub fn with_host(self, host: Option<Host>) -> Config {
        Config { host, ..self }
    }

    pub fn with_storage(self, storage: Vec<StorageItem>) -> Config {
        Config { storage, ..self }
    }

    pub fn with_network(self, network: Vec<NetworkItem>) -> Config {
        Config { network, ..self }
    }

    pub fn with_extras(self, extras: Vec<String>) -> Config {
        Config { extras, ..self }
    }

    pub fn read<S>(name: S) -> Option<Self>
    where
        S: AsRef<str>,
    {
        let name = format!("{}.yaml", name.as_ref());
        let candidates = Osal::find_files(name, vec![".", "~/.ezkvm", "/etc/ezkvm"]);

        if let Some(candidate) = candidates.get(0) {
            if let Ok(config) = Osal::read_yaml_file(candidate.clone()) {
                return Some(config);
            }
        }

        None
    }
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

        result
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
    use crate::config::display::NoDisplay;
    use crate::config::gpu::NoGpu;
    use serial_test::serial;
    use std::path::PathBuf;

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
            "-accel kvm",
            "-nodefaults",
            "-monitor unix:/var/ezkvm/anonymous.monitor,server,nowait",
            "-chardev socket,id=qmp,path=/var/ezkvm/anonymous.qmp,server=on,wait=off",
            "-mon chardev=qmp,mode=control",
            "-chardev socket,id=qmp-event,path=/var/run/qmeventd.sock,reconnect=5",
            "-mon chardev=qmp-event,mode=control",
            "-machine hpet=off,type=pc-q35-8.1",
            "-rtc driftfix=slew,base=localtime",
            "-global kvm-pit.lost_tick_policy=discard",
            "-readconfig /usr/share/ezkvm/pve-q35-4.0.cfg",
            "-device qemu-xhci,p2=15,p3=15,id=xhci,bus=pci.1,addr=0x1b",
            "-iscsi initiator-name=iqn.1993-08.org.debian:01:39407ad058b",
            "-device pvscsi,id=scsihw0,bus=pci.0,addr=0x5",
            "-boot menu=on,strict=on,reboot-timeout=1000,splash=/usr/share/ezkvm/bootsplash.jpg",
            "-smbios type=1,uuid=",
            "-m 16384",
            "-smp 4,sockets=1,cores=4,maxcpus=4",
            "-cpu qemu64,+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce",
        ];

        assert_argument_lists_are_equal(actual, expected);
    }

    const WINDOWS_GAMING_CONFIG: &str = r#"
        general:
            name: windows_gaming_config

        system:
            bios: { type: "ovmf", uuid: "04d064c3-66a1-4aa7-9589-f8b3ecf91cd7", file: "/dev/vm1/vm-108-efidisk" }
            cpu: { model: "qemu64", sockets: 1, cores: 8, flags: "+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce" }
            memory: { max: 16384, balloon: false }
            tpm: { type: "swtpm", version: 2.0, disk: "/dev/vm1/vm-108-tpmstate", socket: "/var/ezkvm/windows_gaming_config-tpm.socket" }

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
    fn test_windows_gaming_config() {
        let config: Config = serde_yaml::from_str(WINDOWS_GAMING_CONFIG).unwrap();

        let tmp = config.get_qemu_args(0);
        let actual: Vec<&str> = tmp.iter().map(std::ops::Deref::deref).collect();
        let expected: Vec<&str> = vec![
            "-accel kvm",
            "-nodefaults",
            "-monitor unix:/var/ezkvm/windows_gaming_config.monitor,server,nowait",
            "-chardev socket,id=qmp,path=/var/ezkvm/windows_gaming_config.qmp,server=on,wait=off",
            "-mon chardev=qmp,mode=control",
            "-chardev socket,id=qmp-event,path=/var/run/qmeventd.sock,reconnect=5",
            "-mon chardev=qmp-event,mode=control",
            "-machine hpet=off,type=pc-q35-8.1",
            "-rtc driftfix=slew,base=localtime",
            "-global kvm-pit.lost_tick_policy=discard",
            "-readconfig /usr/share/ezkvm/pve-q35-4.0.cfg",
            "-device qemu-xhci,p2=15,p3=15,id=xhci,bus=pci.1,addr=0x1b",
            "-iscsi initiator-name=iqn.1993-08.org.debian:01:39407ad058b",
            "-device pvscsi,id=scsihw0,bus=pci.0,addr=0x5",
            "-boot menu=on,strict=on,reboot-timeout=1000",
            "-smbios type=1,uuid=04d064c3-66a1-4aa7-9589-f8b3ecf91cd7",
            "-drive if=pflash,unit=0,format=raw,readonly=on,file=/usr/share/ezkvm/OVMF_CODE_4M.secboot.fd",
            "-drive if=pflash,unit=1,id=drive-efidisk0,format=raw,file=/dev/vm1/vm-108-efidisk,size=540672",
            "-m 16384",
            "-smp 8,sockets=1,cores=8,maxcpus=8",
            "-cpu qemu64,+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce",
            "-chardev socket,id=chrtpm0,path=/var/ezkvm/windows_gaming_config-tpm.socket",
            "-tpmdev emulator,id=tpm0,chardev=chrtpm0",
            "-device tpm-tis,tpmdev=tpm0",
            "-vga none", "-nographic",
            "-device virtio-mouse",
            "-device virtio-keyboard",
            "-device ivshmem-plain,memdev=ivshmem0,bus=pcie.0",
            "-object memory-backend-file,id=ivshmem0,share=on,mem-path=/dev/kvmfr0,size=128M",
            "-device vfio-pci,host=0000:03:00.0,id=hostpci0.0,bus=ich9-pcie-port-1,addr=0x0.0,multifunction=on",
            "-device vfio-pci,host=0000:03:00.1,id=hostpci0.1,bus=ich9-pcie-port-1,addr=0x0.1",
            "-spice port=5903,addr=0.0.0.0,disable-ticketing=on",
            "-device virtio-serial-pci",
            "-chardev spicevmc,id=vdagent,name=vdagent",
            "-device virtserialport,chardev=vdagent,name=com.redhat.spice.0",
            "-audiodev spice,id=spice-backend0",
            "-device ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc",
            "-device hda-duplex,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=spice-backend0",
            "-device usb-host,bus=xhci.0,port=1,hostbus=1,hostport=2.2,id=usb0",
            "-drive file=/dev/vm1/vm-108-boot,if=none,aio=io_uring,id=drive-scsi0,discard=on,format=raw,cache=none,detect-zeroes=unmap",
            "-device scsi-hd,scsi-id=0,drive=drive-scsi0,id=scsi0,bus=scsihw0.0,rotation_rate=1,bootindex=0",
            "-drive file=/dev/vm1/vm-108-tmp,if=none,aio=io_uring,id=drive-scsi1,discard=on,format=raw,cache=none,detect-zeroes=unmap",
            "-device scsi-hd,scsi-id=1,drive=drive-scsi1,id=scsi1,bus=scsihw0.0,rotation_rate=1",
            "-netdev type=bridge,br=vmbr0,id=netdev0",
            "-device virtio-net-pci,id=net0,bus=pci.1,addr=0x0,netdev=netdev0,mac=BC:24:11:3A:21:B7"
        ];

        assert_argument_lists_are_equal(actual, expected);
    }

    const WINDOWS_DESKTOP_CONFIG: &str = r#"
        general:
            name: windows_desktop_config

        system:
          chipset: { type: "q35", version: "8.1" }
          bios: { type: "ovmf", uuid: "181f1a56-e0e2-42d1-a916-bc16dd415a59", file: "/dev/vm1/vm-111-efidisk" }
          cpu: { model: "qemu64", sockets: 1, cores: 8, flags: "+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce" }
          memory: { max: 16384, balloon: false }
          tpm: { type: "swtpm", version: 2.0, disk: "/dev/vm1/vm-111-tpmstate", socket: "/var/ezkvm/windows_desktop_config-tpm.socket" }

        spice:
          path: /var/ezkvm/windows_desktop_config-spice.socket
          gl: true
          render_node: /dev/dri/renderD128

        gpu:
          type: "virtio-vga-gl"

        display:
          type: "remote-viewer"

        storage:
          - { type: "scsi-hd", file: "/dev/vm1/vm-111-boot", discard: "on", boot_index: 0 }

        network:
          - { type: "bridge", bridge: "vmbr0", driver: "virtio-net-pci", mac: "BC:24:11:3A:21:7B" }
    "#;

    #[test]
    fn test_windows_desktop_config() {
        let config: Config = serde_yaml::from_str(WINDOWS_DESKTOP_CONFIG).unwrap();

        let tmp = config.get_qemu_args(0);
        let actual: Vec<&str> = tmp.iter().map(std::ops::Deref::deref).collect();
        let expected: Vec<&str> = vec![
            "-accel kvm",
            "-nodefaults",
            "-monitor unix:/var/ezkvm/windows_desktop_config.monitor,server,nowait",
            "-chardev socket,id=qmp,path=/var/ezkvm/windows_desktop_config.qmp,server=on,wait=off", 
            "-mon chardev=qmp,mode=control", 
            "-chardev socket,id=qmp-event,path=/var/run/qmeventd.sock,reconnect=5", 
            "-mon chardev=qmp-event,mode=control", 
            "-machine hpet=off,type=pc-q35-8.1", 
            "-rtc driftfix=slew,base=localtime", 
            "-global kvm-pit.lost_tick_policy=discard", 
            "-readconfig /usr/share/ezkvm/pve-q35-4.0.cfg", 
            "-device qemu-xhci,p2=15,p3=15,id=xhci,bus=pci.1,addr=0x1b", 
            "-iscsi initiator-name=iqn.1993-08.org.debian:01:39407ad058b", 
            "-device pvscsi,id=scsihw0,bus=pci.0,addr=0x5", 
            "-boot menu=on,strict=on,reboot-timeout=1000",
            "-smbios type=1,uuid=181f1a56-e0e2-42d1-a916-bc16dd415a59", 
            "-drive if=pflash,unit=0,format=raw,readonly=on,file=/usr/share/ezkvm/OVMF_CODE_4M.secboot.fd",
            "-drive if=pflash,unit=1,id=drive-efidisk0,format=raw,file=/dev/vm1/vm-111-efidisk,size=540672", 
            "-m 16384", 
            "-smp 8,sockets=1,cores=8,maxcpus=8", 
            "-cpu qemu64,+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce", 
            "-chardev socket,id=chrtpm0,path=/var/ezkvm/windows_desktop_config-tpm.socket", 
            "-tpmdev emulator,id=tpm0,chardev=chrtpm0", 
            "-device tpm-tis,tpmdev=tpm0", 
            "-device virtio-vga-gl,id=vga,bus=pcie.0,addr=0x2", 
            "-spice unix=on,addr=/var/ezkvm/windows_desktop_config-spice.socket,disable-ticketing=on", 
            "-device virtio-serial-pci", 
            "-chardev spicevmc,id=vdagent,name=vdagent", 
            "-device virtserialport,chardev=vdagent,name=com.redhat.spice.0", 
            "-audiodev spice,id=spice-backend0", 
            "-device ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc", 
            "-device hda-duplex,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=spice-backend0", 
            "-drive file=/dev/vm1/vm-111-boot,if=none,aio=io_uring,id=drive-scsi0,discard=on,format=raw,cache=none,detect-zeroes=unmap", 
            "-device scsi-hd,scsi-id=0,drive=drive-scsi0,id=scsi0,bus=scsihw0.0,rotation_rate=1,bootindex=0", 
            "-netdev type=bridge,br=vmbr0,id=netdev0", 
            "-device virtio-net-pci,id=net0,bus=pci.1,addr=0x0,netdev=netdev0,mac=BC:24:11:3A:21:7B"
        ];

        assert_argument_lists_are_equal(actual, expected);
    }

    const DEFAULT_UBUNTU_CONFIG: &str = r#"
        general:
            name: ubuntu_desktop

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
            "-accel kvm",
            "-nodefaults",
            "-monitor unix:/var/ezkvm/ubuntu_desktop.monitor,server,nowait",
            "-chardev socket,id=qmp,path=/var/ezkvm/ubuntu_desktop.qmp,server=on,wait=off",
            "-mon chardev=qmp,mode=control",
            "-chardev socket,id=qmp-event,path=/var/run/qmeventd.sock,reconnect=5",
            "-mon chardev=qmp-event,mode=control",
            "-machine hpet=off,type=pc-q35-8.1",
            "-rtc driftfix=slew,base=localtime",
            "-global kvm-pit.lost_tick_policy=discard",
            "-readconfig /usr/share/ezkvm/pve-q35-4.0.cfg",
            "-device qemu-xhci,p2=15,p3=15,id=xhci,bus=pci.1,addr=0x1b",
            "-iscsi initiator-name=iqn.1993-08.org.debian:01:39407ad058b",
            "-device pvscsi,id=scsihw0,bus=pci.0,addr=0x5",
            "-boot menu=on,strict=on,reboot-timeout=1000",
            "-smbios type=1,uuid=c0e240a5-859a-4378-a2d9-95088f531142",
            "-drive if=pflash,unit=0,format=raw,readonly=on,file=/usr/share/ezkvm/OVMF_CODE_4M.secboot.fd",
            "-drive if=pflash,unit=1,id=drive-efidisk0,format=raw,file=/dev/vm1/vm-950-disk-0,size=540672",
            "-m 16384",
            "-smp 4,sockets=1,cores=4,maxcpus=4",
            "-cpu qemu64,+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce",
            "-display gtk,gl=on",
            "-audiodev pipewire,id=audiodev0",
            "-device usb-tablet",
            "-device ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc",
            "-device hda-duplex,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=audiodev0",
            "-device virtio-vga-gl,id=vga,bus=pcie.0,addr=0x2",
            "-drive file=/dev/vm1/vm-950-disk-1,if=none,aio=io_uring,id=drive-scsi0,format=raw,cache=none,detect-zeroes=unmap",
            "-device scsi-hd,scsi-id=0,drive=drive-scsi0,id=scsi0,bus=scsihw0.0,rotation_rate=1,bootindex=1",
            "-drive file=ubuntu.iso,if=none,aio=io_uring,id=drive-ide1,media=cdrom",
            "-device ide-cd,bus=ide.1,drive=drive-ide1,id=ide1,unit=0",
            "-netdev type=bridge,br=vmbr0,id=netdev0",
            "-device virtio-net-pci,id=net0,bus=pci.1,addr=0x0,netdev=netdev0,mac=BC:24:11:FF:76:89"
        ];

        assert_argument_lists_are_equal(actual, expected);
    }

    const DEFAULT_MACOS_CONFIG: &str = r#"
        general:
            name: mygeeto

        system:
            bios: { type: "ovmf", uuid: "05cba597-8096-47c6-9cd7-1481dbb3b95a", file: "/dev/vm1/vm-402-efidisk" }
            cpu: { model: "Haswell-noTSX", sockets: 1, cores: 8, flags: "vendor=GenuineIntel,+invtsc,+hypervisor,+pcid,+invpcid,+erms,kvm=on,vmware-cpuid-freq=on" }
            memory: { max: 16384, balloon: false }
            applesmc: { osk: "<my_osk_key>" }

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
    fn test_macos_defaults() {
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
            "-boot", "menu=on,strict=on,reboot-timeout=1000",
            "-smbios", "type=1,uuid=04d064c3-66a1-4aa7-9589-f8b3ecf91cd7",
            "-drive", "if=pflash,unit=0,format=raw,readonly=on,file=/usr/share/ezkvm/OVMF_CODE_4M.secboot.fd",
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
            "-netdev", "type=bridge,br=vmbr0,id=netdev0",
            "-device", "virtio-net-pci,id=net0,bus=pci.1,addr=0x0,netdev=netdev0,mac=BC:24:11:3A:21:B7"
        ];
    }

    #[test]
    fn test_has_gtk_display_configured() {
        let config: Config = serde_yaml::from_str(DEFAULT_UBUNTU_CONFIG).unwrap();
        assert!(config.has_gtk_display_configured())
    }

    #[test]
    fn test_has_no_gtk_display_configured() {
        let config: Config = serde_yaml::from_str(WINDOWS_GAMING_CONFIG).unwrap();
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
        let config: Config = serde_yaml::from_str(WINDOWS_GAMING_CONFIG).unwrap();

        let get_euid_and_egid_context = Osal::get_euid_and_egid_context();
        get_euid_and_egid_context.expect().returning(|| (0, 0));

        let get_uid_and_gid_context = Osal::get_uid_and_gid_context();

        get_uid_and_gid_context.expect().returning(|| (1000, 1000));

        let actual = config.get_escalated_uid_and_gid();
        let expected: (u32, u32) = (0, 0);
        assert_eq!(actual, expected)
    }

    #[test]
    #[serial]
    fn test_config_read_file_not_found() {
        let find_files = Osal::find_files_context();
        find_files.expect().returning(|p: String, l: Vec<&str>| {
            assert_eq!(p, "wakiza.yaml");
            assert_eq!(l, vec![".", "~/.ezkvm", "/etc/ezkvm"]);
            vec![]
        });
        let result = Config::read("wakiza");
        assert!(result.is_none());
    }

    #[test]
    #[serial]
    fn test_config_read_file_success() {
        let find_files = Osal::find_files_context();
        find_files.expect().returning(|p: String, l: Vec<&str>| {
            assert_eq!(p, "wakiza.yaml");
            assert_eq!(l, vec![".", "~/.ezkvm", "/etc/ezkvm"]);
            vec![PathBuf::from("/etc/ezkvm/wakiza.yaml")]
        });

        let read_yaml_file = Osal::read_yaml_file_context();
        read_yaml_file.expect().returning(|n: PathBuf| {
            assert_eq!(n, PathBuf::from("/etc/ezkvm/wakiza.yaml"));
            Ok(Config {
                general: Default::default(),
                system: Default::default(),
                display: Box::new(NoDisplay {}),
                gpu: Box::new(NoGpu {}),
                spice: None,
                host: None,
                storage: vec![],
                network: vec![],
                extras: vec![],
            })
        });

        let _config = Config::read("wakiza").unwrap();
    }
}

/// helper function to compare argument lists independent of order
#[allow(unused)]
pub fn assert_argument_lists_are_equal(mut actual: Vec<&str>, mut expected: Vec<&str>) {
    assert_eq!(actual.len(), expected.len());
    actual.sort();
    expected.sort();
    let mut count = 0;
    while count < actual.len() {
        assert_eq!(actual[count], expected[count]);
        count += 1;
    }
}

/// helper function to compare argument options independent of order
#[allow(unused)]
pub fn assert_arguments_are_equal(actual: &str, expected: &str) {
    // arguments take the form '-<argument> [option,...]'
    // so split them further
    let actual_split: Vec<String> = actual.split_whitespace().map(str::to_string).collect();
    let expected_split: Vec<String> = expected.split_whitespace().map(str::to_string).collect();
    assert_eq!(actual_split.len(), expected_split.len());

    // there should be at most one space, so length after splitting must be 1 or 2
    assert!(actual_split.len() == 1 || actual_split.len() == 2);

    // the argument itself must match
    let actual_left = actual_split.get(0).unwrap().clone();
    let expected_left = expected_split.get(0).unwrap().clone();
    assert_eq!(actual_left, expected_left);

    // if there are options, then split them
    if actual_split.len() == 2 {
        let mut actual_split: Vec<String> = actual_split
            .get(1)
            .unwrap()
            .split(",")
            .map(str::to_string)
            .collect();
        let mut expected_split: Vec<String> = expected_split
            .get(1)
            .unwrap()
            .split(",")
            .map(str::to_string)
            .collect();

        // for the first option in the arguments, order may still be relevant, so compare those first, and the rest after sorting
        assert_eq!(actual_split.get(0).unwrap(), expected_split.get(0).unwrap());
        actual_split.sort();
        expected_split.sort();
        assert_eq!(actual_split.len(), expected_split.len());
    }
}
