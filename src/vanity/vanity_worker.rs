use crate::vanity::generated_key::KeyGenerationResult;

pub trait VanityWorker<'a> {
    fn new() -> Self where Self: Sized;
    fn add_matcher(&mut self, f: Box<dyn Fn(&str) -> bool + 'a>);
    fn clear_matchers(&mut self);
    fn generate_key(&self) -> Vec<KeyGenerationResult>;
    fn test(&self, s: &str) -> bool;
}
