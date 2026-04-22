#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptionPlan {
    pub enabled: bool,
    pub algorithm: &'static str,
}

impl Default for EncryptionPlan {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithm: "AES-256",
        }
    }
}
