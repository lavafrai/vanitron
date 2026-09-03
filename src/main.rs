mod cli;
mod vanity;

use clap::Parser;
use regex::RegexBuilder;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;

use crate::cli::args::{Args, BackendArg};
use crate::vanity::tron::seed::{BackendMode, ComputeConfig};
use crate::vanity::tron::tron_worker::TronWorker;
use crate::vanity::vanity_worker::VanityWorker;

fn main() {
    let args = Args::parse();

    if args.list_adapters {
        print_adapters();
        return;
    }

    if args.backend == BackendArg::Cpu && !args.adapter.eq_ignore_ascii_case("auto") {
        eprintln!("--adapter cannot be used with --backend cpu");
        std::process::exit(2);
    }

    let has_passphrase = args
        .passphrase
        .as_deref()
        .is_some_and(|passphrase| !passphrase.is_empty());
    if args.backend != BackendArg::Cpu
        && !has_passphrase
        && !args.adapter.eq_ignore_ascii_case("auto")
    {
        validate_adapter_or_exit(&args.adapter);
    }

    let threads_count = args
        .threads
        .unwrap_or_else(|| num_cpus::get().saturating_sub(1).max(1));
    let passphrase = args.passphrase.unwrap_or_default();
    let compute = ComputeConfig {
        backend: match args.backend {
            BackendArg::Cpu => BackendMode::Cpu,
            BackendArg::Gpu => BackendMode::Gpu,
            BackendArg::Hybrid => BackendMode::Hybrid,
        },
        adapter: args.adapter,
        gpu_batch_size: args.gpu_batch_size,
    };

    let mut vanity_worker: Box<dyn VanityWorker> = Box::new(TronWorker::new(
        args.mnemonic_size.unwrap_or(24),
        1,
        passphrase,
        threads_count,
        compute,
    ));

    {
        let regex = RegexBuilder::new(args.pattern.as_deref().expect("pattern is required"))
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
        })
        .unwrap();
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
        let warn = anstyle::Style::new()
            .bold()
            .fg_color(Some(anstyle::AnsiColor::Yellow.into()));
        anstream::println!("{warn}[INTERRUPTED]{warn:#} Stopped by Ctrl+C");
        return;
    }

    let found_wallets_list = vanity_worker.get_found_wallets();
    let wallet = found_wallets_list.first().unwrap();

    let tries = vanity_worker.get_generated_wallets_count();
    let elapsed = start_time.elapsed();

    cli::result::print_result(wallet, tries, elapsed);
}

#[cfg(feature = "gpu-pbkdf2")]
fn print_adapters() {
    match vanity::tron::seed::gpu::enumerate_adapters() {
        Ok(adapters) if adapters.is_empty() => {
            println!("No GPU adapters detected. CPU fallback will be used.");
        }
        Ok(adapters) => {
            for adapter in adapters {
                println!("{}", adapter.display_line());
            }
        }
        Err(error) => {
            eprintln!("Could not enumerate GPU adapters: {error}");
            println!("CPU fallback will be used.");
        }
    }
}

#[cfg(not(feature = "gpu-pbkdf2"))]
fn print_adapters() {
    println!("GPU support is not compiled into this binary. CPU fallback will be used.");
}

#[cfg(feature = "gpu-pbkdf2")]
fn validate_adapter_or_exit(selector: &str) {
    if let Err(error) = vanity::tron::seed::gpu::validate_manual_selector(selector) {
        eprintln!("{error}");
        eprintln!("Run --list-adapters to see available GPU adapters.");
        std::process::exit(2);
    }
}

#[cfg(not(feature = "gpu-pbkdf2"))]
fn validate_adapter_or_exit(_selector: &str) {
    eprintln!("A manual --adapter was requested, but GPU support is not compiled in.");
    std::process::exit(2);
}
