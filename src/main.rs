mod vanity;
mod cli;

use std::thread;
use clap::Parser;
use crate::vanity::tron::tron_worker::TronWorker;
use crate::vanity::vanity_worker::VanityWorker;
use regex::RegexBuilder;
use crate::cli::args::Args;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};


fn main() {
    let args = Args::parse();
    let threads_count = args.threads.unwrap_or_else(|| (num_cpus::get() - 1).max(1));

    let mut vanity_worker: Box<dyn VanityWorker> = Box::new(TronWorker::new(
        args.mnemonic_size.unwrap_or(24),
        1,
        args.passphrase.unwrap_or_default(),
        threads_count,
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

    println!("Starting generation with {} threads...", threads_count);
    println!("Press Ctrl+C to stop.");

    let mut progress = cli::progress::ProgressBar::new(start_time);

    let interrupted = Arc::new(AtomicBool::new(false));

    {
        let interrupted = interrupted.clone();
        ctrlc::set_handler(move || {
            interrupted.store(true, Ordering::Relaxed);
        }).unwrap();
    }

    vanity_worker.start_generation();
    let mut found_wallets = false;
    while !found_wallets && !interrupted.load(Ordering::Relaxed) {
        thread::sleep(std::time::Duration::from_millis(100));
        found_wallets = vanity_worker.has_found_wallets();
        let new_tries = vanity_worker.get_generated_wallets_count();
        progress.update(new_tries);
    }

    progress.finish();

    if interrupted.load(Ordering::Relaxed) && !found_wallets {
        vanity_worker.stop_generation();
        let warn = anstyle::Style::new().bold().fg_color(Some(anstyle::AnsiColor::Yellow.into()));
        anstream::println!("{warn}[INTERRUPTED]{warn:#} Stopped by Ctrl+C");
        return;
    }

    let found_wallets_list = vanity_worker.get_found_wallets();
    let wallet = found_wallets_list.first().unwrap();

    let tries = vanity_worker.get_generated_wallets_count();
    let elapsed = start_time.elapsed();

    cli::result::print_result(wallet, tries, elapsed);
}
