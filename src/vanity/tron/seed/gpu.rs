use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use bip39::Mnemonic;
use zeroize::Zeroize;

use super::{CpuSeedBatchDeriver, Seed, SeedBatchDeriver};

const RECORD_WORDS: usize = 57;
const RECORD_DATA_BYTES: usize = 224;
const OUTPUT_WORDS: usize = 16;
const WORKGROUP_SIZE: u32 = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterDescription {
    pub index: usize,
    pub name: String,
    pub backend: String,
    pub device_type: String,
    pub compatible: bool,
}

impl AdapterDescription {
    pub fn display_line(&self) -> String {
        let compatibility = if self.compatible {
            "PBKDF2 compatible"
        } else {
            "missing SHADER_INT64"
        };
        format!(
            "[{}] {} [{} / {}] - {}",
            self.index, self.name, self.backend, self.device_type, compatibility
        )
    }
}

struct CatalogEntry {
    description: AdapterDescription,
    adapter: wgpu::Adapter,
}

fn platform_backends() -> wgpu::Backends {
    #[cfg(target_os = "macos")]
    {
        wgpu::Backends::METAL
    }
    #[cfg(target_os = "windows")]
    {
        wgpu::Backends::DX12
    }
    #[cfg(target_os = "linux")]
    {
        wgpu::Backends::VULKAN
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        wgpu::Backends::PRIMARY
    }
}

fn create_instance() -> wgpu::Instance {
    let descriptor = wgpu::InstanceDescriptor {
        backends: platform_backends(),
        flags: wgpu::InstanceFlags::from_build_config(),
        memory_budget_thresholds: Default::default(),
        backend_options: platform_backend_options(),
        display: None,
    };
    wgpu::Instance::new(descriptor)
}

#[cfg(target_os = "windows")]
fn platform_backend_options() -> wgpu::BackendOptions {
    let mut options = wgpu::BackendOptions::default();
    options.dx12.shader_compiler = wgpu::Dx12Compiler::StaticDxc;
    options
}

#[cfg(not(target_os = "windows"))]
fn platform_backend_options() -> wgpu::BackendOptions {
    wgpu::BackendOptions::default()
}

fn enumerate_catalog() -> Result<Vec<CatalogEntry>, String> {
    let instance = create_instance();
    let adapters = pollster::block_on(instance.enumerate_adapters(platform_backends()));
    Ok(adapters
        .into_iter()
        .enumerate()
        .map(|(index, adapter)| {
            let info = adapter.get_info();
            let compatible = adapter.features().contains(wgpu::Features::SHADER_INT64);
            CatalogEntry {
                description: AdapterDescription {
                    index,
                    name: info.name,
                    backend: format!("{:?}", info.backend),
                    device_type: format!("{:?}", info.device_type),
                    compatible,
                },
                adapter,
            }
        })
        .collect())
}

pub fn enumerate_adapters() -> Result<Vec<AdapterDescription>, String> {
    Ok(enumerate_catalog()?
        .into_iter()
        .map(|entry| entry.description)
        .collect())
}

pub fn resolve_adapter(
    adapters: &[AdapterDescription],
    selector: &str,
) -> Result<Option<usize>, String> {
    if selector.eq_ignore_ascii_case("auto") {
        return Ok(None);
    }

    if let Ok(index) = selector.parse::<usize>() {
        return adapters
            .iter()
            .find(|adapter| adapter.index == index)
            .map(|adapter| Some(adapter.index))
            .ok_or_else(|| adapter_selection_error(selector, adapters));
    }

    let exact: Vec<_> = adapters
        .iter()
        .filter(|adapter| adapter.name.eq_ignore_ascii_case(selector))
        .collect();
    if exact.len() == 1 {
        return Ok(Some(exact[0].index));
    }
    if exact.len() > 1 {
        return Err(ambiguous_adapter_error(selector, &exact));
    }

    let selector_lower = selector.to_lowercase();
    let partial: Vec<_> = adapters
        .iter()
        .filter(|adapter| adapter.name.to_lowercase().contains(&selector_lower))
        .collect();
    match partial.as_slice() {
        [adapter] => Ok(Some(adapter.index)),
        [] => Err(adapter_selection_error(selector, adapters)),
        _ => Err(ambiguous_adapter_error(selector, &partial)),
    }
}

fn adapter_selection_error(selector: &str, adapters: &[AdapterDescription]) -> String {
    let available = if adapters.is_empty() {
        "no adapters were detected".to_string()
    } else {
        adapters
            .iter()
            .map(AdapterDescription::display_line)
            .collect::<Vec<_>>()
            .join("; ")
    };
    format!("GPU adapter '{selector}' was not found ({available})")
}

fn ambiguous_adapter_error(selector: &str, adapters: &[&AdapterDescription]) -> String {
    let candidates = adapters
        .iter()
        .map(|adapter| adapter.display_line())
        .collect::<Vec<_>>()
        .join("; ");
    format!("GPU adapter selector '{selector}' is ambiguous: {candidates}")
}

pub fn validate_manual_selector(selector: &str) -> Result<(), String> {
    if selector.eq_ignore_ascii_case("auto") {
        return Ok(());
    }
    let adapters = enumerate_adapters()?;
    resolve_adapter(&adapters, selector).map(|_| ())
}

pub struct GpuCoordinator {
    lanes: Vec<GpuLane>,
    next_lane: usize,
}

impl GpuCoordinator {
    pub fn initialize(selector: &str, max_batch_size: usize) -> Result<Self, String> {
        let catalog = enumerate_catalog()?;
        if catalog.is_empty() {
            return Err("no GPU adapters were detected".to_string());
        }
        let descriptions: Vec<_> = catalog
            .iter()
            .map(|entry| entry.description.clone())
            .collect();
        let selected_index = resolve_adapter(&descriptions, selector)?;

        let candidates: Vec<_> = catalog
            .into_iter()
            .filter(|entry| {
                entry.description.compatible
                    && selected_index
                        .map(|index| entry.description.index == index)
                        .unwrap_or(true)
            })
            .collect();
        if candidates.is_empty() {
            return match selected_index {
                Some(index) => Err(format!(
                    "selected GPU adapter [{}] {} does not support SHADER_INT64",
                    index, descriptions[index].name
                )),
                None => Err("no GPU adapter supports SHADER_INT64".to_string()),
            };
        }

        let mut best: Option<(GpuLane, f64)> = None;
        let mut failures = Vec::new();
        for candidate in candidates {
            let name = candidate.description.name.clone();
            match GpuLane::new(candidate, max_batch_size) {
                Ok(mut lane) => match lane.benchmark() {
                    Ok(rate) => {
                        if best
                            .as_ref()
                            .map(|(_, old_rate)| rate > *old_rate)
                            .unwrap_or(true)
                        {
                            best = Some((lane, rate));
                        }
                    }
                    Err(error) => failures.push(format!("{name}: {error}")),
                },
                Err(error) => failures.push(format!("{name}: {error}")),
            }
        }

        let (lane, _) = best.ok_or_else(|| {
            if failures.is_empty() {
                "no usable GPU adapter was found".to_string()
            } else {
                format!(
                    "all GPU adapters failed initialization: {}",
                    failures.join("; ")
                )
            }
        })?;
        Ok(Self {
            lanes: vec![lane],
            next_lane: 0,
        })
    }

    pub fn adapter_name(&self) -> &str {
        &self.lanes[0].description.name
    }

    pub fn adapter_backend(&self) -> &str {
        &self.lanes[0].description.backend
    }

    pub fn measured_rate(&self) -> f64 {
        self.lanes[0].measured_rate
    }
}

impl SeedBatchDeriver for GpuCoordinator {
    fn derive_batch(&mut self, mnemonics: &[Mnemonic]) -> Result<Vec<Seed>, String> {
        if self.lanes.is_empty() {
            return Err("GPU coordinator has no active lanes".to_string());
        }
        let lane_index = self.next_lane % self.lanes.len();
        self.next_lane = (lane_index + 1) % self.lanes.len();
        self.lanes[lane_index].derive_batch(mnemonics)
    }
}

struct GpuLane {
    description: AdapterDescription,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    input_buffer: wgpu::Buffer,
    output_buffer: wgpu::Buffer,
    staging_buffer: wgpu::Buffer,
    max_batch_size: usize,
    measured_rate: f64,
    device_lost: Arc<Mutex<Option<String>>>,
}

impl GpuLane {
    fn new(entry: CatalogEntry, max_batch_size: usize) -> Result<Self, String> {
        if max_batch_size == 0 {
            return Err("GPU batch size must be greater than zero".to_string());
        }
        let requested_features = wgpu::Features::SHADER_INT64;
        let (device, queue) =
            pollster::block_on(entry.adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("vanitron PBKDF2 device"),
                required_features: requested_features,
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            }))
            .map_err(|error| format!("could not create GPU device: {error}"))?;

        let device_lost = Arc::new(Mutex::new(None));
        let device_lost_callback = Arc::clone(&device_lost);
        device.set_device_lost_callback(move |reason, message| {
            *device_lost_callback.lock().unwrap() =
                Some(format!("device lost ({reason:?}): {message}"));
        });

        let out_of_memory_scope = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let validation_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vanitron PBKDF2-SHA512 shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("pbkdf2.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("vanitron PBKDF2-SHA512 pipeline"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let input_size = ((4 + max_batch_size * RECORD_WORDS) * size_of::<u32>()) as u64;
        let output_size = (max_batch_size * OUTPUT_WORDS * size_of::<u32>()) as u64;
        let input_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vanitron mnemonic input"),
            size: input_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vanitron seed output"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vanitron seed readback"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vanitron PBKDF2 bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });
        if let Some(error) = pollster::block_on(validation_scope.pop()) {
            return Err(format!("GPU pipeline validation failed: {error}"));
        }
        if let Some(error) = pollster::block_on(out_of_memory_scope.pop()) {
            return Err(format!("GPU allocation failed: {error}"));
        }

        let mut lane = Self {
            description: entry.description,
            device,
            queue,
            pipeline,
            bind_group,
            input_buffer,
            output_buffer,
            staging_buffer,
            max_batch_size,
            measured_rate: 0.0,
            device_lost,
        };
        lane.self_test()?;
        Ok(lane)
    }

    fn self_test(&mut self) -> Result<(), String> {
        let short_mnemonic = Mnemonic::parse(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        )
        .map_err(|error| format!("could not create PBKDF2 self-test vector: {error}"))?;
        let long_mnemonic = Mnemonic::parse(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art",
        )
        .map_err(|error| format!("could not create long PBKDF2 self-test vector: {error}"))?;
        let mnemonics = vec![short_mnemonic, long_mnemonic];
        let expected = CpuSeedBatchDeriver::new("").derive_batch(&mnemonics)?;
        let mut actual = Vec::with_capacity(mnemonics.len());
        for chunk in mnemonics.chunks(self.max_batch_size) {
            actual.extend(self.derive_batch_internal(chunk)?);
        }
        if let Some(mismatch_index) = actual
            .iter()
            .zip(expected.iter())
            .position(|(actual, expected)| actual != expected)
        {
            let expected_hex = seed_hex(&expected[mismatch_index]);
            let actual_hex = seed_hex(&actual[mismatch_index]);
            return Err(format!(
                "GPU PBKDF2 self-test vector {mismatch_index} produced a different BIP-39 seed (expected {expected_hex}, got {actual_hex})"
            ));
        }
        Ok(())
    }

    fn benchmark(&mut self) -> Result<f64, String> {
        let mnemonic = Mnemonic::parse(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        )
        .map_err(|error| format!("could not create benchmark mnemonic: {error}"))?;
        let sample_size = self.max_batch_size.min(4096);
        let sample = vec![mnemonic; sample_size];
        let started = Instant::now();
        let mut seeds = self.derive_batch_internal(&sample)?;
        let elapsed = started.elapsed().as_secs_f64();
        seeds.zeroize();
        self.measured_rate = sample_size as f64 / elapsed.max(f64::EPSILON);
        Ok(self.measured_rate)
    }

    fn derive_batch_internal(&mut self, mnemonics: &[Mnemonic]) -> Result<Vec<Seed>, String> {
        if mnemonics.is_empty() {
            return Ok(Vec::new());
        }
        if mnemonics.len() > self.max_batch_size {
            return Err(format!(
                "GPU batch contains {} mnemonics, maximum is {}",
                mnemonics.len(),
                self.max_batch_size
            ));
        }
        if let Some(error) = self.device_lost.lock().unwrap().clone() {
            return Err(error);
        }

        let mut packed = pack_mnemonics(mnemonics)?;
        self.queue
            .write_buffer(&self.input_buffer, 0, bytemuck::cast_slice(&packed));
        packed.zeroize();

        let output_size = (mnemonics.len() * OUTPUT_WORDS * size_of::<u32>()) as u64;
        let error_scope = self.device.push_error_scope(wgpu::ErrorFilter::Internal);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("vanitron PBKDF2 command encoder"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("vanitron PBKDF2 pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups((mnemonics.len() as u32).div_ceil(WORKGROUP_SIZE), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&self.output_buffer, 0, &self.staging_buffer, 0, output_size);
        let submission = self.queue.submit(Some(encoder.finish()));
        let slice = self.staging_buffer.slice(0..output_size);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result.map_err(|error| error.to_string()));
        });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(120)),
            })
            .map_err(|error| format!("GPU execution failed: {error}"))?;
        receiver
            .recv_timeout(Duration::from_secs(1))
            .map_err(|error| format!("GPU readback callback failed: {error}"))??;
        if let Some(error) = pollster::block_on(error_scope.pop()) {
            self.staging_buffer.unmap();
            return Err(format!("GPU execution failed: {error}"));
        }
        if let Some(error) = self.device_lost.lock().unwrap().clone() {
            self.staging_buffer.unmap();
            return Err(error);
        }

        let mapped = slice
            .get_mapped_range()
            .map_err(|error| format!("GPU readback mapping failed: {error}"))?;
        let words: &[u32] = bytemuck::cast_slice(&mapped);
        let mut seeds = Vec::with_capacity(mnemonics.len());
        for candidate_words in words.chunks_exact(OUTPUT_WORDS) {
            let mut seed = [0u8; 64];
            for word_index in 0..OUTPUT_WORDS {
                seed[word_index * 4..word_index * 4 + 4]
                    .copy_from_slice(&candidate_words[word_index].to_be_bytes());
            }
            seeds.push(seed);
        }
        drop(mapped);
        self.staging_buffer.unmap();

        let mut clear_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("vanitron sensitive buffer clear"),
                });
        clear_encoder.clear_buffer(&self.input_buffer, 0, None);
        clear_encoder.clear_buffer(&self.output_buffer, 0, None);
        clear_encoder.clear_buffer(&self.staging_buffer, 0, None);
        self.queue.submit(Some(clear_encoder.finish()));
        Ok(seeds)
    }
}

impl SeedBatchDeriver for GpuLane {
    fn derive_batch(&mut self, mnemonics: &[Mnemonic]) -> Result<Vec<Seed>, String> {
        self.derive_batch_internal(mnemonics)
    }
}

fn pack_mnemonics(mnemonics: &[Mnemonic]) -> Result<Vec<u32>, String> {
    let mut words = vec![0u32; 4 + mnemonics.len() * RECORD_WORDS];
    words[0] = mnemonics.len() as u32;
    for (candidate_index, mnemonic) in mnemonics.iter().enumerate() {
        let mut phrase = mnemonic.to_string();
        let bytes = phrase.as_bytes();
        if bytes.len() > RECORD_DATA_BYTES {
            let phrase_len = bytes.len();
            phrase.zeroize();
            return Err(format!(
                "mnemonic is {} bytes long, GPU limit is {} bytes",
                phrase_len, RECORD_DATA_BYTES
            ));
        }
        let record_base = 4 + candidate_index * RECORD_WORDS;
        words[record_base] = bytes.len() as u32;
        for (byte_index, byte) in bytes.iter().enumerate() {
            words[record_base + 1 + byte_index / 4] |= u32::from(*byte) << ((byte_index % 4) * 8);
        }
        phrase.zeroize();
    }
    Ok(words)
}

fn seed_hex(seed: &Seed) -> String {
    seed.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::{AdapterDescription, RECORD_DATA_BYTES, pack_mnemonics, resolve_adapter};
    use bip39::Mnemonic;

    fn adapter(index: usize, name: &str) -> AdapterDescription {
        AdapterDescription {
            index,
            name: name.to_string(),
            backend: "Test".to_string(),
            device_type: "DiscreteGpu".to_string(),
            compatible: true,
        }
    }

    #[test]
    fn resolves_adapter_by_index_exact_name_and_unique_substring() {
        let adapters = vec![adapter(0, "Apple M4 Pro"), adapter(1, "NVIDIA RTX 4090")];

        assert_eq!(resolve_adapter(&adapters, "1").unwrap(), Some(1));
        assert_eq!(resolve_adapter(&adapters, "apple m4 pro").unwrap(), Some(0));
        assert_eq!(resolve_adapter(&adapters, "RTX").unwrap(), Some(1));
        assert_eq!(resolve_adapter(&adapters, "auto").unwrap(), None);
    }

    #[test]
    fn rejects_ambiguous_or_missing_adapter() {
        let adapters = vec![adapter(0, "GPU A"), adapter(1, "GPU B")];

        assert!(
            resolve_adapter(&adapters, "GPU")
                .unwrap_err()
                .contains("ambiguous")
        );
        assert!(
            resolve_adapter(&adapters, "other")
                .unwrap_err()
                .contains("not found")
        );
    }

    #[test]
    fn packs_long_mnemonic_inside_fixed_record() {
        let mnemonic = Mnemonic::parse(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art",
        )
        .unwrap();
        let phrase = mnemonic.to_string();
        assert!(phrase.len() > 128);
        assert!(phrase.len() <= RECORD_DATA_BYTES);

        let packed = pack_mnemonics(&[mnemonic]).unwrap();
        assert_eq!(packed[4] as usize, phrase.len());
    }

    #[test]
    #[ignore = "requires a compatible physical GPU"]
    fn physical_gpu_matches_cpu_at_batch_boundaries() {
        use crate::vanity::tron::seed::{CpuSeedBatchDeriver, SeedBatchDeriver};
        use bip39::Language;

        let mnemonic = Mnemonic::parse(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        )
        .unwrap();
        let mut gpu = super::GpuCoordinator::initialize("auto", 16_384).unwrap();
        let mut cpu = CpuSeedBatchDeriver::new("");
        for size in [1, 63, 64, 65, 16_384] {
            let batch = vec![mnemonic.clone(); size];
            assert_eq!(
                gpu.derive_batch(&batch).unwrap(),
                cpu.derive_batch(&batch).unwrap()
            );
        }

        for word_count in [12, 15, 18, 21, 24] {
            let batch: Vec<_> = (0..4)
                .map(|_| Mnemonic::generate_in(Language::English, word_count).unwrap())
                .collect();
            assert_eq!(
                gpu.derive_batch(&batch).unwrap(),
                cpu.derive_batch(&batch).unwrap()
            );
        }
    }
}
