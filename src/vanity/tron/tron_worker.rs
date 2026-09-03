use crate::vanity::generated_key::KeyGenerationResult;
use crate::vanity::tron::crypto::address::public_key_to_address;
use crate::vanity::tron::crypto::keys::{
    derivate_seed_to_private, private_key_to_public, tron_derivation_path,
};
use crate::vanity::tron::crypto::mnemonic::{generate_mnemonic, mnemonic_to_seed};
use crate::vanity::tron::seed::{BackendMode, ComputeConfig, Seed};
#[cfg(feature = "gpu-pbkdf2")]
use crate::vanity::tron::seed::{SeedBatchDeriver, gpu::GpuCoordinator};
use crate::vanity::vanity_worker::VanityWorker;
use bip39::Mnemonic;
#[cfg(feature = "gpu-pbkdf2")]
use rayon::prelude::*;
use regex::Regex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use zeroize::Zeroize;

#[cfg(feature = "gpu-pbkdf2")]
fn hybrid_thread_split(thread_budget: usize) -> (usize, usize) {
    if thread_budget <= 1 {
        return (1, 1);
    }
    let gpu_post_threads = (thread_budget / 4).max(1);
    (thread_budget - gpu_post_threads, gpu_post_threads)
}

#[cfg(feature = "gpu-pbkdf2")]
struct GpuPostTask {
    batch_id: u64,
    mnemonics: Arc<Vec<Mnemonic>>,
    seeds: Arc<Vec<Seed>>,
    handle: thread::JoinHandle<Result<(), String>>,
}

#[cfg(feature = "gpu-pbkdf2")]
struct GpuPostFailure {
    batch_id: u64,
    mnemonics: Arc<Vec<Mnemonic>>,
    seeds: Arc<Vec<Seed>>,
    error: String,
}

#[derive(Clone)]
pub struct TronWorker {
    matchers: Vec<Regex>,
    found_wallets: Arc<Mutex<Vec<KeyGenerationResult>>>,
    mnemonic_size: usize,
    max_child_number: u32,
    passphrase: String,
    threads_count: usize,
    compute: ComputeConfig,
    wallets_generated: Arc<AtomicU64>,
    running: Arc<AtomicBool>,
}

impl VanityWorker for TronWorker {
    fn new() -> Self {
        Self::new(24, 1, String::new(), 1, ComputeConfig::default())
    }

    fn add_matcher(&mut self, f: Regex) {
        self.matchers.push(f);
    }

    fn clear_matchers(&mut self) {
        self.matchers.clear();
    }

    fn has_wallets_found(&self) -> bool {
        !self.lock_found_wallets().is_empty()
    }

    fn get_generated_wallets_count(&self) -> u64 {
        self.wallets_generated.load(Ordering::Relaxed)
    }

    fn has_found_wallets(&self) -> bool {
        !self.lock_found_wallets().is_empty()
    }

    fn get_found_wallets(&self) -> Vec<KeyGenerationResult> {
        self.lock_found_wallets().clone()
    }

    fn start_generation(&self) {
        let worker = Arc::new(self.clone());
        self.running.store(true, Ordering::Release);

        if self.compute.backend == BackendMode::Cpu || !self.passphrase.is_empty() {
            if !self.passphrase.is_empty() && self.compute.backend != BackendMode::Cpu {
                eprintln!(
                    "GPU backends currently support only an empty BIP-39 passphrase; using CPU."
                );
            }
            worker.start_cpu_workers();
            return;
        }

        #[cfg(feature = "gpu-pbkdf2")]
        match self.compute.backend {
            BackendMode::Cpu => unreachable!("CPU backend returned above"),
            BackendMode::Gpu => {
                let post_threads = self.threads_count;
                thread::spawn(move || {
                    worker.run_gpu_generation_guarded(post_threads, post_threads, false)
                });
            }
            BackendMode::Hybrid => worker.start_hybrid_generation(),
        }

        #[cfg(not(feature = "gpu-pbkdf2"))]
        {
            eprintln!("GPU support is not compiled into this binary; using CPU.");
            worker.start_cpu_workers();
        }
    }

    fn stop_generation(&self) {
        self.running.store(false, Ordering::Release);
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
        compute: ComputeConfig,
    ) -> Self {
        Self {
            matchers: Vec::new(),
            found_wallets: Arc::new(Mutex::new(Vec::new())),
            mnemonic_size,
            max_child_number,
            passphrase,
            threads_count,
            compute,
            wallets_generated: Arc::new(AtomicU64::new(0)),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    fn lock_found_wallets(&self) -> MutexGuard<'_, Vec<KeyGenerationResult>> {
        self.found_wallets
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn generate_matching_key(&self) -> Option<KeyGenerationResult> {
        let mut mnemonic = generate_mnemonic(self.mnemonic_size);
        let mut seed = mnemonic_to_seed(&mnemonic, &self.passphrase);
        let result = self
            .matching_key_from_seed(&mnemonic, &seed, false)
            .unwrap_or(None);

        seed.zeroize();
        mnemonic.zeroize();
        result
    }

    fn matching_key_from_seed(
        &self,
        mnemonic: &Mnemonic,
        seed: &Seed,
        verify_with_cpu: bool,
    ) -> Result<Option<KeyGenerationResult>, String> {
        for child_number in 0..self.max_child_number {
            let Some(address) = Self::address_from_seed(seed, child_number) else {
                continue;
            };
            if !self.test(&address) {
                continue;
            }

            if verify_with_cpu {
                return self
                    .verify_gpu_match(mnemonic, child_number, &address)
                    .map(Some);
            }
            return Ok(Some(Self::key_result(mnemonic, child_number, address)));
        }
        Ok(None)
    }

    fn address_from_seed(seed: &Seed, child_number: u32) -> Option<String> {
        let mut private_key = derivate_seed_to_private(seed, child_number)?;
        let public_key = private_key_to_public(&private_key);
        private_key.zeroize();
        let public_key = public_key?;
        Some(public_key_to_address(&public_key))
    }

    fn key_result(mnemonic: &Mnemonic, child_number: u32, address: String) -> KeyGenerationResult {
        KeyGenerationResult {
            mnemonic: mnemonic.to_string(),
            derivation_path: tron_derivation_path(child_number).into_owned(),
            address,
        }
    }

    fn verify_gpu_match(
        &self,
        mnemonic: &Mnemonic,
        child_number: u32,
        gpu_address: &str,
    ) -> Result<KeyGenerationResult, String> {
        let mut reference_seed = mnemonic_to_seed(mnemonic, &self.passphrase);
        let verified_address = Self::address_from_seed(&reference_seed, child_number);
        reference_seed.zeroize();

        let Some(verified_address) = verified_address else {
            return Err(format!(
                "reference CPU derivation failed for {}",
                tron_derivation_path(child_number)
            ));
        };
        if verified_address != gpu_address {
            return Err(format!(
                "GPU PBKDF2 verification mismatch for {}",
                tron_derivation_path(child_number)
            ));
        }
        Ok(Self::key_result(mnemonic, child_number, verified_address))
    }

    fn start_cpu_workers(self: &Arc<Self>) {
        println!("Compute backend: CPU ({} threads)", self.threads_count);
        self.spawn_cpu_workers(self.threads_count);
    }

    fn spawn_cpu_workers(self: &Arc<Self>, count: usize) {
        for _ in 0..count {
            let worker = Arc::clone(self);
            thread::spawn(move || {
                while worker.running.load(Ordering::Acquire) {
                    let key = worker.generate_matching_key();
                    worker.wallets_generated.fetch_add(1, Ordering::Relaxed);
                    if let Some(key) = key {
                        worker.lock_found_wallets().push(key);
                        worker.running.store(false, Ordering::Release);
                    }
                }
            });
        }
    }

    #[cfg(feature = "gpu-pbkdf2")]
    fn start_hybrid_generation(self: &Arc<Self>) {
        let (cpu_brute_threads, gpu_post_threads) = hybrid_thread_split(self.threads_count);
        println!(
            "Compute backend: hybrid ({cpu_brute_threads} independent CPU threads, {gpu_post_threads} GPU post-processing threads)"
        );
        self.spawn_cpu_workers(cpu_brute_threads);
        let gpu_worker = Arc::clone(self);
        thread::spawn(move || {
            gpu_worker.run_gpu_generation_guarded(gpu_post_threads, gpu_post_threads, true)
        });
    }

    #[cfg(feature = "gpu-pbkdf2")]
    fn run_gpu_generation_guarded(
        self: Arc<Self>,
        post_threads: usize,
        fallback_threads: usize,
        hybrid: bool,
    ) {
        let fallback_worker = Arc::clone(&self);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.run_gpu_generation(post_threads, fallback_threads, hybrid);
        }));
        if result.is_err() && fallback_worker.running.load(Ordering::Acquire) {
            eprintln!("GPU backend panicked unexpectedly; disabling it.");
            fallback_worker.activate_gpu_cpu_fallback(fallback_threads, hybrid);
        }
    }

    #[cfg(feature = "gpu-pbkdf2")]
    fn run_gpu_generation(
        self: Arc<Self>,
        post_threads: usize,
        fallback_threads: usize,
        hybrid: bool,
    ) {
        let mut coordinator =
            match GpuCoordinator::initialize(&self.compute.adapter, self.compute.gpu_batch_size) {
                Ok(coordinator) => coordinator,
                Err(error) => {
                    eprintln!("GPU initialization failed ({error}).");
                    self.activate_gpu_cpu_fallback(fallback_threads, hybrid);
                    return;
                }
            };

        let pool = match rayon::ThreadPoolBuilder::new()
            .num_threads(post_threads)
            .thread_name(|index| format!("vanitron-post-{index}"))
            .build()
        {
            Ok(pool) => Arc::new(pool),
            Err(error) => {
                eprintln!("Could not create CPU post-processing pool ({error}).");
                self.activate_gpu_cpu_fallback(fallback_threads, hybrid);
                return;
            }
        };

        if hybrid {
            println!(
                "GPU lane: {} ({}, PBKDF2 probe {:.0}/s)",
                coordinator.adapter_name(),
                coordinator.adapter_backend(),
                coordinator.measured_rate(),
            );
        } else {
            println!(
                "Compute backend: GPU {} ({}, PBKDF2 probe {:.0}/s), CPU post-processing: {post_threads} threads",
                coordinator.adapter_name(),
                coordinator.adapter_backend(),
                coordinator.measured_rate(),
            );
        }

        let mut batch_id = 0u64;
        let mut pending_post = None;
        while self.running.load(Ordering::Acquire) {
            let mut mnemonics: Vec<_> = (0..self.compute.gpu_batch_size)
                .take_while(|_| self.running.load(Ordering::Acquire))
                .map(|_| generate_mnemonic(self.mnemonic_size))
                .collect();
            if mnemonics.is_empty() {
                break;
            }

            let gpu_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                coordinator.derive_batch(&mnemonics)
            }))
            .unwrap_or_else(|_| Err("GPU runtime panicked".to_string()));

            match gpu_result {
                Ok(mut seeds) => {
                    // Double-buffer the lane: while the CPU checks batch N, the GPU derives
                    // batch N+1. Joining here bounds memory use to one batch per stage.
                    if let Some(task) = pending_post.take()
                        && let Err(failure) = Self::join_gpu_post_task(task)
                    {
                        self.replay_gpu_post_failure(failure, &pool);
                        seeds.zeroize();
                        if self.running.load(Ordering::Acquire)
                            && let Err(error) =
                                self.process_mnemonic_batch_on_cpu(batch_id, &pool, &mnemonics)
                        {
                            eprintln!("CPU replay of the current GPU batch failed ({error}).");
                        }
                        mnemonics.zeroize();
                        self.activate_gpu_cpu_fallback(fallback_threads, hybrid);
                        return;
                    }
                    if !self.running.load(Ordering::Acquire) {
                        seeds.zeroize();
                        mnemonics.zeroize();
                        break;
                    }
                    pending_post = Some(self.spawn_gpu_post_task(
                        Arc::clone(&pool),
                        batch_id,
                        mnemonics,
                        seeds,
                    ));
                }
                Err(primary_error) => {
                    if let Some(task) = pending_post.take()
                        && let Err(failure) = Self::join_gpu_post_task(task)
                    {
                        self.replay_gpu_post_failure(failure, &pool);
                    }
                    eprintln!(
                        "GPU failed while processing batch {batch_id} ({primary_error}); replaying it on CPU and disabling GPU."
                    );
                    if self.running.load(Ordering::Acquire)
                        && let Err(error) =
                            self.process_mnemonic_batch_on_cpu(batch_id, &pool, &mnemonics)
                    {
                        eprintln!("CPU replay of failed GPU batch failed ({error}).");
                    }
                    mnemonics.zeroize();
                    if self.running.load(Ordering::Acquire) {
                        self.activate_gpu_cpu_fallback(fallback_threads, hybrid);
                    }
                    return;
                }
            }
            batch_id = batch_id.wrapping_add(1);
        }

        if let Some(task) = pending_post
            && let Err(failure) = Self::join_gpu_post_task(task)
        {
            if self.running.load(Ordering::Acquire) {
                self.replay_gpu_post_failure(failure, &pool);
            } else {
                eprintln!(
                    "GPU post-processing failed while stopping ({}).",
                    failure.error
                );
                Self::discard_gpu_post_failure(failure);
            }
        }
    }

    #[cfg(feature = "gpu-pbkdf2")]
    fn spawn_gpu_post_task(
        self: &Arc<Self>,
        pool: Arc<rayon::ThreadPool>,
        batch_id: u64,
        mnemonics: Vec<Mnemonic>,
        seeds: Vec<Seed>,
    ) -> GpuPostTask {
        let mnemonics = Arc::new(mnemonics);
        let seeds = Arc::new(seeds);
        let worker = Arc::clone(self);
        let thread_mnemonics = Arc::clone(&mnemonics);
        let thread_seeds = Arc::clone(&seeds);
        let handle = thread::spawn(move || {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                worker.process_derived_batch(
                    batch_id,
                    &pool,
                    thread_mnemonics.as_ref(),
                    thread_seeds.as_ref(),
                    true,
                )
            }))
            .unwrap_or_else(|_| Err(format!("batch {batch_id} panicked")))
        });
        GpuPostTask {
            batch_id,
            mnemonics,
            seeds,
            handle,
        }
    }

    #[cfg(feature = "gpu-pbkdf2")]
    fn join_gpu_post_task(task: GpuPostTask) -> Result<(), GpuPostFailure> {
        let GpuPostTask {
            batch_id,
            mnemonics,
            seeds,
            handle,
        } = task;
        let result = handle
            .join()
            .unwrap_or_else(|_| Err("post-processing thread panicked".to_string()));
        match result {
            Ok(()) => {
                Self::zeroize_arc_vec(seeds);
                Self::zeroize_arc_vec(mnemonics);
                Ok(())
            }
            Err(error) => Err(GpuPostFailure {
                batch_id,
                mnemonics,
                seeds,
                error,
            }),
        }
    }

    #[cfg(feature = "gpu-pbkdf2")]
    fn zeroize_arc_vec<T: Zeroize>(values: Arc<Vec<T>>) {
        if let Ok(mut values) = Arc::try_unwrap(values) {
            values.zeroize();
        }
    }

    #[cfg(feature = "gpu-pbkdf2")]
    fn discard_gpu_post_failure(failure: GpuPostFailure) {
        Self::zeroize_arc_vec(failure.seeds);
        Self::zeroize_arc_vec(failure.mnemonics);
    }

    #[cfg(feature = "gpu-pbkdf2")]
    fn replay_gpu_post_failure(&self, failure: GpuPostFailure, pool: &rayon::ThreadPool) {
        eprintln!(
            "GPU post-processing failed for batch {} ({}); replaying it on CPU and disabling GPU.",
            failure.batch_id, failure.error
        );
        if self.running.load(Ordering::Acquire)
            && let Err(error) = self.process_mnemonic_batch_on_cpu(
                failure.batch_id,
                pool,
                failure.mnemonics.as_ref(),
            )
        {
            eprintln!("CPU replay of failed post-processing batch failed ({error}).");
        }
        Self::discard_gpu_post_failure(failure);
    }

    #[cfg(feature = "gpu-pbkdf2")]
    fn activate_gpu_cpu_fallback(self: &Arc<Self>, fallback_threads: usize, hybrid: bool) {
        if !self.running.load(Ordering::Acquire) {
            return;
        }
        if hybrid {
            eprintln!(
                "The {fallback_threads} threads reserved for the GPU lane will continue with CPU brute force."
            );
        } else {
            eprintln!("Continuing with CPU brute force ({fallback_threads} threads).");
        }
        self.spawn_cpu_workers(fallback_threads);
    }

    #[cfg(feature = "gpu-pbkdf2")]
    fn derive_cpu_batch(&self, pool: &rayon::ThreadPool, mnemonics: &[Mnemonic]) -> Vec<Seed> {
        pool.install(|| {
            mnemonics
                .par_iter()
                .map(|mnemonic| mnemonic_to_seed(mnemonic, &self.passphrase))
                .collect()
        })
    }

    #[cfg(feature = "gpu-pbkdf2")]
    fn process_mnemonic_batch_on_cpu(
        &self,
        batch_id: u64,
        pool: &rayon::ThreadPool,
        mnemonics: &[Mnemonic],
    ) -> Result<(), String> {
        let mut seeds = self.derive_cpu_batch(pool, mnemonics);
        let result = self.process_derived_batch(batch_id, pool, mnemonics, &seeds, false);
        seeds.zeroize();
        result
    }

    #[cfg(feature = "gpu-pbkdf2")]
    fn process_derived_batch(
        &self,
        batch_id: u64,
        pool: &rayon::ThreadPool,
        mnemonics: &[Mnemonic],
        seeds: &[Seed],
        verify_with_cpu: bool,
    ) -> Result<(), String> {
        if mnemonics.len() != seeds.len() {
            return Err(format!(
                "batch {batch_id} has {} mnemonics but {} seeds",
                mnemonics.len(),
                seeds.len()
            ));
        }

        let matches: Vec<Option<KeyGenerationResult>> = pool.install(|| {
            mnemonics
                .par_iter()
                .zip(seeds.par_iter())
                .map(|(mnemonic, seed)| {
                    self.matching_key_from_seed(mnemonic, seed, verify_with_cpu)
                })
                .collect::<Result<Vec<_>, _>>()
        })?;
        let matches: Vec<KeyGenerationResult> = matches.into_iter().flatten().collect();

        self.wallets_generated
            .fetch_add(mnemonics.len() as u64, Ordering::Relaxed);
        if !matches.is_empty() {
            self.lock_found_wallets().extend(matches);
            self.running.store(false, Ordering::Release);
        }
        Ok(())
    }
}

#[cfg(all(test, feature = "gpu-pbkdf2"))]
mod tests {
    use super::{GpuPostFailure, TronWorker, hybrid_thread_split};
    use crate::vanity::tron::crypto::mnemonic::mnemonic_to_seed;
    use crate::vanity::tron::seed::{BackendMode, ComputeConfig, Seed};
    use crate::vanity::vanity_worker::VanityWorker;
    use bip39::Mnemonic;
    use regex::Regex;
    use std::sync::Arc;
    use std::sync::atomic::Ordering;

    fn worker_matching(pattern: &str) -> TronWorker {
        let mut worker = TronWorker::new(
            12,
            1,
            String::new(),
            1,
            ComputeConfig {
                backend: BackendMode::Cpu,
                adapter: "auto".to_string(),
                gpu_batch_size: 16,
            },
        );
        worker.add_matcher(Regex::new(pattern).unwrap());
        worker.running.store(true, Ordering::Release);
        worker
    }

    fn known_mnemonic() -> Mnemonic {
        Mnemonic::parse(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        )
        .unwrap()
    }

    fn different_valid_seed() -> Seed {
        let mnemonic = Mnemonic::parse(
            "legal winner thank year wave sausage worth useful legal winner thank yellow",
        )
        .unwrap();
        mnemonic_to_seed(&mnemonic, "")
    }

    #[test]
    fn hybrid_reserves_one_quarter_for_gpu_post_processing() {
        assert_eq!(hybrid_thread_split(13), (10, 3));
        assert_eq!(hybrid_thread_split(4), (3, 1));
        assert_eq!(hybrid_thread_split(2), (1, 1));
        assert_eq!(hybrid_thread_split(1), (1, 1));
    }

    #[test]
    fn gpu_post_processing_rejects_seed_mismatch_before_counting() {
        let worker = worker_matching("^T");
        let mnemonic = known_mnemonic();
        let wrong_seed = different_valid_seed();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();

        let error = worker
            .process_derived_batch(7, &pool, &[mnemonic], &[wrong_seed], true)
            .unwrap_err();

        assert!(error.contains("GPU PBKDF2 verification mismatch"));
        assert_eq!(worker.get_generated_wallets_count(), 0);
        assert!(worker.get_found_wallets().is_empty());
    }

    #[test]
    fn cpu_batch_processing_counts_only_after_checking() {
        let worker = worker_matching("^T");
        let mnemonic = known_mnemonic();
        let seed = mnemonic_to_seed(&mnemonic, "");
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();

        worker
            .process_derived_batch(8, &pool, &[mnemonic], &[seed], false)
            .unwrap();

        assert_eq!(worker.get_generated_wallets_count(), 1);
        assert_eq!(worker.get_found_wallets().len(), 1);
        assert!(!worker.running.load(Ordering::Acquire));
    }

    #[test]
    fn gpu_post_failure_replays_saved_batch_once_on_cpu() {
        let worker = worker_matching("^T");
        let mnemonic = known_mnemonic();
        let wrong_seed = different_valid_seed();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        let failure = GpuPostFailure {
            batch_id: 9,
            mnemonics: Arc::new(vec![mnemonic]),
            seeds: Arc::new(vec![wrong_seed]),
            error: "forced failure".to_string(),
        };

        worker.replay_gpu_post_failure(failure, &pool);

        assert_eq!(worker.get_generated_wallets_count(), 1);
        let wallets = worker.get_found_wallets();
        assert_eq!(wallets.len(), 1);
        assert_eq!(wallets[0].address, "TUEZSdKsoDHQMeZwihtdoBiN46zxhGWYdH");
    }
}
