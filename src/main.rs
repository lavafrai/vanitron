mod vanity;
mod cli;

use std::thread;
use clap::Parser;
use crate::vanity::tron::tron_worker::TronWorker;
use crate::vanity::vanity_worker::VanityWorker;
use regex::RegexBuilder;
use crate::cli::args::Args;

fn main() {;
    let args = Args::parse();

    let mut vanity_worker: Box<dyn VanityWorker> = Box::new(TronWorker::new(
        args.mnemonic_size.unwrap_or(24),
        1,
        args.passphrase.unwrap_or(String::from("")),
        args.threads.unwrap_or((num_cpus::get() - 1).max(1)),
    ));

    {
        let regex = RegexBuilder::new(&args.pattern)
            .case_insensitive(!args.case_sensitive)
            .build()
            .unwrap_or_else(|e| {
                eprintln!("Invalid regex pattern: {}", e);
                std::process::exit(1);
            });
        vanity_worker.add_matcher(regex);
    }

    let start_time = std::time::Instant::now();

    vanity_worker.start_generation();
    let mut found_wallets = false;
    let mut tries: u64 = 0;
    let mut last_time = std::time::Instant::now();
    while !found_wallets {
        thread::sleep(std::time::Duration::from_millis(1000));
        found_wallets = vanity_worker.has_found_wallets();
        let new_tries = vanity_worker.get_generated_wallets_count();
        let time_elapsed = start_time.elapsed().as_secs_f64().max(0.1f64);

        let rate_total = tries as f64 / time_elapsed;
        let rate_last = (new_tries as f64 - tries as f64) / last_time.elapsed().as_secs_f64();

        println!("Total tries: {}, Rate: {} tries/sec, Last rate: {} tries/sec",
                 new_tries, rate_total, rate_last);

        tries = new_tries;
        last_time = std::time::Instant::now();
    }

    let found_wallets_list = vanity_worker.get_found_wallets();
    let wallet = found_wallets_list.first().unwrap();

    println!("Found wallet: {}", wallet.address);
    println!("Mnemonic: {}", wallet.mnemonic);

    let tries = vanity_worker.get_generated_wallets_count();
    let elapsed = start_time.elapsed();
    println!("Time elapsed: {:.2?}", elapsed);
    println!("Total tries: {}", tries);
    println!("Speed: {:.2} tries/sec", tries as f64 / elapsed.as_secs_f64());
    println!("Done!");
}
