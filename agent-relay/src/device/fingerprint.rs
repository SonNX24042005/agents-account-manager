use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProfile {
    pub machine_id: String,
    pub mac_machine_id: String,
    pub dev_device_id: String,
    pub sqm_id: String,
}

impl DeviceProfile {
    pub fn generate() -> Self {
        Self {
            machine_id: Uuid::new_v4().to_string().replace('-', ""),
            mac_machine_id: Uuid::new_v4().to_string().replace('-', ""),
            dev_device_id: Uuid::new_v4().to_string(),
            sqm_id: format!("{{{}}}", Uuid::new_v4().to_string().to_uppercase()),
        }
    }
}
