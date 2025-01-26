use super::super::{Config, QemuDevice};
use super::network_footer::NetworkFooter;
use super::network_header::NetworkHeader;
use super::network_payload::NetworkPayload;
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
    fn pre_start(&self, config: &Config) {
        self.payload.pre_start(self, config);
    }
    fn post_start(&self, config: &Config) {
        self.payload.post_start(self, config);
    }
    fn pre_stop(&self, config: &Config) {
        self.payload.pre_stop(self, config);
    }
    fn post_stop(&self, config: &Config) {
        self.payload.post_stop(self, config);
    }
    fn pre_hibernate(&self, config: &Config) {
        self.payload.pre_hibernate(self, config);
    }
    fn post_hibernate(&self, config: &Config) {
        self.payload.post_hibernate(self, config);
    }

    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        let mut netdev_args: Vec<String> = vec![];
        netdev_args.extend(self.header().get_netdev_options());
        netdev_args.extend(self.payload().get_netdev_options(index));
        netdev_args.extend(self.footer().get_netdev_options(index));

        let mut device_args: Vec<String> = vec![];
        device_args.extend(self.header().get_device_options());
        device_args.extend(self.payload().get_device_options(index));
        device_args.extend(self.footer().get_device_options(index));

        let mut result = vec![];
        if !netdev_args.is_empty() {
            result.push(format!("-netdev {}", netdev_args.join(",")));
        }
        if !device_args.is_empty() {
            result.push(format!("-device {}", device_args.join(",")));
        }

        result
    }
}
