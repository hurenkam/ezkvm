use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, Debug, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Boolean {
    #[default]
    Yes,
    No,
}
