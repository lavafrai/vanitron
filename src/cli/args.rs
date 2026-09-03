use clap::{Parser, ValueEnum};

fn parse_gpu_batch_size(value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("'{value}' is not a positive integer"))?;
    if parsed == 0 {
        return Err("value must be greater than zero".to_string());
    }
    if parsed > 262_144 {
        return Err("value must not exceed 262144".to_string());
    }
    Ok(parsed)
}

fn parse_positive_usize(value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("'{value}' is not a positive integer"))?;
    (parsed > 0)
        .then_some(parsed)
        .ok_or_else(|| "value must be greater than zero".to_string())
}

fn parse_mnemonic_size(value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("'{value}' is not a mnemonic word count"))?;
    [12, 15, 18, 21, 24]
        .contains(&parsed)
        .then_some(parsed)
        .ok_or_else(|| "mnemonic size must be one of 12, 15, 18, 21, or 24".to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum BackendArg {
    Cpu,
    Gpu,
    Hybrid,
}

#[derive(Parser)]
#[command(name = "Vanitron")]
#[command(version = "1.0")]
pub struct Args {
    #[arg(
        long,
        short = 'j',
        value_parser = parse_positive_usize,
        help = "Number of threads to use. By default, it uses all available CPU cores minus one."
    )]
    pub threads: Option<usize>,

    #[arg(
        long,
        short,
        default_value = "false",
        help = "Enable case sensitivity for the pattern matching. By default, it is case insensitive."
    )]
    #[arg(default_value_t = false)]
    pub case_sensitive: bool,

    #[arg(
        long,
        short = 'm',
        default_value = "24",
        value_parser = parse_mnemonic_size,
        help = "Mnemonic size in words"
    )]
    pub mnemonic_size: Option<usize>,

    #[arg(long, short = 'p', help = "BIP-39 passphrase (optional)")]
    pub passphrase: Option<String>,

    #[arg(long, value_enum, default_value_t = BackendArg::Hybrid, help = "Compute backend: cpu, gpu, or hybrid (independent CPU brute force plus GPU PBKDF2)")]
    pub backend: BackendArg,

    #[arg(
        long,
        default_value = "auto",
        help = "GPU adapter: auto, an index from --list-adapters, or a unique name"
    )]
    pub adapter: String,

    #[arg(long, default_value_t = 16_384, value_parser = parse_gpu_batch_size, help = "Number of mnemonics in one GPU batch")]
    pub gpu_batch_size: usize,

    #[arg(long, help = "List GPU adapters and exit")]
    pub list_adapters: bool,

    #[arg(required_unless_present = "list_adapters")]
    pub pattern: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{Args, BackendArg};
    use clap::Parser;

    #[test]
    fn passphrase_is_absent_by_default() {
        let args = Args::try_parse_from(["vanitron", "pattern"]).unwrap();

        assert_eq!(args.passphrase, None);
        assert_eq!(args.backend, BackendArg::Hybrid);
        assert_eq!(args.adapter, "auto");
        assert_eq!(args.gpu_batch_size, 16_384);
    }

    #[test]
    fn explicit_passphrase_is_preserved() {
        let args = Args::try_parse_from(["vanitron", "--passphrase", "secret", "pattern"]).unwrap();

        assert_eq!(args.passphrase.as_deref(), Some("secret"));
    }

    #[test]
    fn adapter_listing_does_not_require_a_pattern() {
        let args = Args::try_parse_from(["vanitron", "--list-adapters"]).unwrap();

        assert!(args.list_adapters);
        assert_eq!(args.pattern, None);
    }

    #[test]
    fn rejects_zero_threads_and_invalid_mnemonic_sizes() {
        assert!(Args::try_parse_from(["vanitron", "--threads", "0", "pattern"]).is_err());
        assert!(Args::try_parse_from(["vanitron", "--mnemonic-size", "13", "pattern"]).is_err());
        assert!(Args::try_parse_from(["vanitron", "--mnemonic-size", "21", "pattern"]).is_ok());
    }

    #[test]
    fn accepts_exactly_three_compute_backends() {
        for backend in ["cpu", "gpu", "hybrid"] {
            assert!(Args::try_parse_from(["vanitron", "--backend", backend, "pattern"]).is_ok());
        }
        assert!(Args::try_parse_from(["vanitron", "--backend", "auto", "pattern"]).is_err());
    }
}
