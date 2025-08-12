use crate::vanity::generated_key::KeyGenerationResult;
use crate::vanity::tron::crypto::address::public_key_to_address;
use crate::vanity::tron::crypto::keys::{derivate_seed_to_private, private_key_to_public};
use crate::vanity::tron::crypto::mnemonic::{generate_mnemonic, mnemonic_to_seed};
use crate::vanity::vanity_worker::VanityWorker;
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use regex::Regex;

#[derive(Clone)]
pub struct TronWorker {
    matchers: Vec<Regex>,
    found_wallets: Arc<Mutex<Vec<KeyGenerationResult>>>,
    mnemonic_size: usize,
    max_child_number: u32,
    passphrase: String,
    threads_count: usize,
    wallets_generated: Arc<AtomicU64>,
    running: Arc<AtomicBool>,
}

impl VanityWorker for TronWorker {
    fn new() -> Self {
        Self::new(24, 1, String::new(), 1)
    }

    fn add_matcher(&mut self, f: Regex) {
        self.matchers.push(f);
    }

    fn clear_matchers(&mut self) {
        self.matchers.clear();
    }

    fn has_wallets_found(&self) -> bool {
        // проверяем через мьютекс
        !self.found_wallets.lock().unwrap().is_empty()
    }

    fn get_generated_wallets_count(&self) -> u64 {
        self.wallets_generated.load(Ordering::SeqCst)
    }

    fn get_found_wallets(&self) -> Vec<KeyGenerationResult> {
        self.found_wallets.lock().unwrap().clone()
    }

    fn start_generation(&self) {
        let self_arc = Arc::new(self.clone());
        self.running.store(true, Ordering::SeqCst);
        for _ in 0..self.threads_count {
            let worker = self_arc.clone();
            thread::spawn(move || {
                while worker.has_wallets_found() {
                    let keys = worker.generate_key();
                    for key in keys {
                        if worker.test(&key.address) {
                            worker.found_wallets.lock().unwrap().push(key.clone());
                        }
                    }
                    worker.wallets_generated.fetch_add(1, Ordering::SeqCst);
                }
            });
        }
    }
    
    fn stop_generation(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    fn test(&self, s: &str) -> bool {
        for matcher in &self.matchers {
            if matcher.is_match(s) {
                return true;
            }
        }
        false
    }
}

impl TronWorker {
    pub fn new(
        mnemonic_size: usize,
        max_child_number: u32,
        passphrase: String,
        threads_count: usize,
    ) -> Self {
        Self {
            matchers: Vec::new(),
            found_wallets: Arc::new(Mutex::new(Vec::new())),
            mnemonic_size,
            max_child_number,
            passphrase,
            threads_count,
            wallets_generated: Arc::new(AtomicU64::new(0)),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    fn generate_key(&self) -> Vec<KeyGenerationResult> {
        let mnemonic = generate_mnemonic(self.mnemonic_size);
        let seed = mnemonic_to_seed(&mnemonic, &self.passphrase);
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
}
