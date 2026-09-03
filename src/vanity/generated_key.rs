#[derive(Debug, Clone)]
pub struct KeyGenerationResult {
    pub mnemonic: String,
    pub derivation_path: String,
    pub address: String,
}
