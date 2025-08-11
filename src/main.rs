mod vanity;

use regex::{Regex, RegexBuilder};
use crate::vanity::tron::tron_worker::TronWorker;
use crate::vanity::vanity_worker::VanityWorker;

fn main() {
    let mut vanity_worker: Box<dyn VanityWorker> = Box::new(TronWorker::new(
        24,
        1,
        ""
    ));

    {
        let regex = RegexBuilder::new(r".+va$")
            .case_insensitive(true)
            .build()
            .expect("Couldn't build regex");
        vanity_worker.add_matcher(Box::new(move |s: &str| {
            regex.is_match(s)
        }));
    }

    let start_time = std::time::Instant::now();
    let mut done = false;
    let mut tries = 0;
    while !done {
        let results = vanity_worker.generate_key();
        for result in results {
            if (vanity_worker.test(result.address.as_str())) {
                println!("Found matching address: {}", result.address);
                println!("Mnemonic: {}", result.mnemonic);
                println!("Derivation Path: {}", result.derivation_path);
                done = true;
                break;
            } else {
                println!("Generated address: {}", result.address);
                tries += 1;
            }
        }
    }
    let elapsed = start_time.elapsed();
    println!("Time elapsed: {:.2?}", elapsed);
    println!("Total tries: {}", tries);
    println!("Speed: {:.2} tries/sec", tries as f64 / elapsed.as_secs_f64());
    println!("Done!");
}
