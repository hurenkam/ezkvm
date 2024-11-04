use crate::config::network::network_footer::NetworkFooter;

use crate::config::network::network_header::NetworkHeader;
use crate::config::network::network_payload::NetworkPayload;
use crate::config::QemuDevice;
use derive_getters::Getters;
use serde::Deserialize;

#[derive(Deserialize, Debug, Getters)]
pub struct NetworkItem {
    #[serde(flatten)]
    header: NetworkHeader,
    #[serde(flatten)]
    payload: Box<dyn NetworkPayload>,
    #[serde(flatten)]
    footer: NetworkFooter,
}

impl QemuDevice for NetworkItem {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        let mut netdev_args: Vec<String> = vec![];
        netdev_args.extend(self.header().get_netdev_options());
        netdev_args.extend(self.payload().get_netdev_options(index));
        netdev_args.extend(self.footer().get_netdev_options());

        let mut device_args: Vec<String> = vec![];
        device_args.extend(self.header().get_device_options());
        device_args.extend(self.payload().get_device_options(index));
        device_args.extend(self.footer().get_device_options());

        vec![
            format!("-netdev {}", netdev_args.join(",")),
            format!("-device {}", device_args.join(",")),
        ]
    }
}
