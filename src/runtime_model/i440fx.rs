use super::BusRegistrationApi;

pub struct I440fxChipset {}
impl I440fxChipset {
    pub fn new(_api: &dyn BusRegistrationApi) -> Self {
        Self {}
    }
    pub fn qemu_args(&self) -> Vec<String> {
        todo!()
    }
}
