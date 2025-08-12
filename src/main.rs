mod vanity;

use std::thread;
use crate::vanity::tron::tron_worker::TronWorker;
use crate::vanity::vanity_worker::VanityWorker;
use regex::RegexBuilder;

fn main() {
    let mut vanity_worker: Box<dyn VanityWorker> = Box::new(TronWorker::new(
        24,
        1,
        String::from(""),
        11
    ));

    {
        let regex = RegexBuilder::new(r".+ava$")
            .case_insensitive(true)
            .build()
            .expect("Couldn't build regex");
        vanity_worker.add_matcher(regex);
    }

    let start_time = std::time::Instant::now();

    vanity_worker.start_generation();
    let mut i = 0;
    let mut found_wallets = false;
    while !found_wallets {
        
    }

    let tries = vanity_worker.get_generated_wallets_count();
    let elapsed = start_time.elapsed();
    println!("Time elapsed: {:.2?}", elapsed);
    println!("Total tries: {}", tries);
    println!("Speed: {:.2} tries/sec", tries as f64 / elapsed.as_secs_f64());
    println!("Done!");
}
