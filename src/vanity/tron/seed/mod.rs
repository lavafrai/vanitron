#![cfg_attr(not(feature = "gpu-pbkdf2"), allow(dead_code))]

use bip39::Mnemonic;

use crate::vanity::tron::crypto::mnemonic::mnemonic_to_seed;

#[cfg(feature = "gpu-pbkdf2")]
pub mod gpu;

pub type Seed = [u8; 64];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendMode {
    Cpu,
    Gpu,
    Hybrid,
}

#[derive(Debug, Clone)]
pub struct ComputeConfig {
    pub backend: BackendMode,
    pub adapter: String,
    pub gpu_batch_size: usize,
}

impl Default for ComputeConfig {
    fn default() -> Self {
        Self {
            backend: BackendMode::Hybrid,
            adapter: "auto".to_string(),
            gpu_batch_size: 16_384,
        }
    }
}

pub trait SeedBatchDeriver {
    fn derive_batch(&mut self, mnemonics: &[Mnemonic]) -> Result<Vec<Seed>, String>;
}

pub struct CpuSeedBatchDeriver<'a> {
    passphrase: &'a str,
}

impl<'a> CpuSeedBatchDeriver<'a> {
    pub fn new(passphrase: &'a str) -> Self {
        Self { passphrase }
    }
}

impl SeedBatchDeriver for CpuSeedBatchDeriver<'_> {
    fn derive_batch(&mut self, mnemonics: &[Mnemonic]) -> Result<Vec<Seed>, String> {
        Ok(mnemonics
            .iter()
            .map(|mnemonic| mnemonic_to_seed(mnemonic, self.passphrase))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::{CpuSeedBatchDeriver, SeedBatchDeriver};
    use bip39::Mnemonic;

    #[test]
    fn cpu_batch_deriver_matches_bip39() {
        let mnemonic = Mnemonic::parse(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        )
        .unwrap();
        let expected = mnemonic.to_seed("");

        let result = CpuSeedBatchDeriver::new("")
            .derive_batch(&[mnemonic])
            .unwrap();

        assert_eq!(result, vec![expected]);
    }

    #[test]
    fn empty_passphrase_matches_known_bip39_vector() {
        let mnemonic = Mnemonic::parse(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        )
        .unwrap();
        let seed = CpuSeedBatchDeriver::new("")
            .derive_batch(&[mnemonic])
            .unwrap()
            .remove(0);
        let actual = seed
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();

        assert_eq!(
            actual,
            "5eb00bbddcf069084889a8ab9155568165f5c453ccb85e70811aaed6f6da5fc19a5ac40b389cd370d086206dec8aa6c43daea6690f20ad3d8d48b2d2ce9e38e4"
        );
    }
}
