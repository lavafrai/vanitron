use crate::vanity::generated_key::KeyGenerationResult;
use regex::Regex;

pub trait VanityWorker: Send + Sync + 'static {
    fn new() -> Self where Self: Sized;
    fn add_matcher(&mut self, f: Regex);
    fn clear_matchers(&mut self);
    fn has_wallets_found(&self) -> bool;
    fn get_generated_wallets_count(&self) -> u64;
    fn get_found_wallets(&self) -> Vec<KeyGenerationResult>;
    fn start_generation(&self);
    fn stop_generation(&self);
    fn test(&self, _s: &str) -> bool;
}
