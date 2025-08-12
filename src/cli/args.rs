use clap::Parser;

#[derive(Parser)]
#[command(name = "Vanitron")]
#[command(version = "1.0")]
// #[command(about = "Does awesome things", long_about = None)]
pub struct Args {
    #[arg(long, short = 'j', help = "Number of threads to use. By default, it uses all available CPU cores minus one.")]
    pub threads: Option<usize>,

    #[arg(long, short, default_value = "false", help = "Enable case sensitivity for the pattern matching. By default, it is case insensitive.")]
    #[arg(default_value_t = false)]
    pub case_sensitive: bool,

    #[arg(long, short = 'm', default_value = "24", help = "Mnemonic size in words")]
    pub mnemonic_size: Option<usize>,

    #[arg(long, short = 'p', default_value = "<None>", help = "Passphrase for mnemonic generation")]
    pub passphrase: Option<String>,

    pub pattern: String,
}