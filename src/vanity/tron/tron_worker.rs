use crate::vanity::generated_key::KeyGenerationResult;
use crate::vanity::tron::crypto::address::public_key_to_address;
use crate::vanity::tron::crypto::keys::{derivate_seed_to_private, private_key_to_public};
use crate::vanity::tron::crypto::mnemonic::{generate_mnemonic, mnemonic_to_seed};
use crate::vanity::vanity_worker::VanityWorker;

pub struct TronWorker<'a> {
    matchers: Vec<Box<dyn Fn(&str) -> bool + 'a>>,
    mnemonic_size: usize,
    max_child_number: u32,
    passphrase: &'a str,
}

impl<'a> VanityWorker<'a> for TronWorker<'a> {
    fn new() -> Self {
        Self::new(
            24,
            1,
            ""
        )
    }

    fn add_matcher(&mut self, f: Box<dyn Fn(&str) -> bool + 'a>) {
        self.matchers.push(f);
    }

    fn clear_matchers(&mut self) {
        self.matchers.clear();
    }
    
    fn generate_key(&self) -> Vec<KeyGenerationResult> {
        let mnemonic = generate_mnemonic(self.mnemonic_size);
        let seed = mnemonic_to_seed(&mnemonic, "");
        let mut result = Vec::new();

        for child_number in 0..self.max_child_number {
            let derivation_path = format!("m/44'/195'/0'/0/{}", child_number);
            let private_key = derivate_seed_to_private(&seed, child_number);
            let public_key = private_key_to_public(&private_key);
            let address = public_key_to_address(&public_key);
            let key_result = KeyGenerationResult {
                mnemonic: mnemonic.to_string(),
                derivation_path,
                address
            };
            result.push(key_result);
        }

        result
    }

    fn test(&self, s: &str) -> bool {
        for matcher in &self.matchers {
            if matcher(s) {
                return true;
            }
        }
        false
    }
}

impl<'a> TronWorker<'a> {

    pub fn new(mnemonic_size: usize, max_child_number: u32, passphrase: &'a str) -> Self {
        Self {
            matchers: Vec::new(),
            mnemonic_size,
            max_child_number,
            passphrase
        }
    }
}
