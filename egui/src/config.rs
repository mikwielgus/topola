#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Config {}

impl Default for Config {
    fn default() -> Self {
        Self {}
    }
}
