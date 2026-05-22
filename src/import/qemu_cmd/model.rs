#[derive(Debug, Clone, PartialEq)]
pub struct QemuCmdModel {
    pub executable: String,
    pub options: Vec<QemuCmdOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QemuCmdOption {
    pub flag: String,
    pub raw_value: Option<String>,
    pub value: QemuCmdOptionValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QemuCmdOptionValue {
    None,
    Scalar(String),
    Csv(Vec<QemuCsvPart>),
    Json(serde_json::Value),
}

#[derive(Debug, Clone, PartialEq)]
pub enum QemuCsvPart {
    Bare(String),
    KeyValue { key: String, value: String },
}

impl QemuCmdModel {
    pub fn options_for_flag<'a>(
        &'a self,
        flag: &'a str,
    ) -> impl Iterator<Item = &'a QemuCmdOption> + 'a {
        self.options
            .iter()
            .filter(move |option| option.flag == flag)
    }
}
