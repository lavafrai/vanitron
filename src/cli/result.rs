use crate::cli::fmt::{fmt_duration, fmt_int, fmt_rate};
use crate::vanity::generated_key::KeyGenerationResult;
use anstyle::{AnsiColor, Style};
use std::time::Duration;

pub fn print_result(wallet: &KeyGenerationResult, tries: u64, elapsed: Duration) {
    let rate = tries as f64 / elapsed.as_secs_f64().max(1e-9);

    let ok = Style::new().fg_color(Some(AnsiColor::Green.into())).bold();
    let label = Style::new().bold();
    let val_accent = Style::new().fg_color(Some(AnsiColor::Cyan.into())).bold();

    anstream::println!("{ok}[OK]{ok:#} Done");
    anstream::println!(
        "{label}Address:{label:#} {val}{addr}{val:#}",
        label = label,
        val = val_accent,
        addr = wallet.address
    );
    if !wallet.derivation_path.is_empty() {
        anstream::println!(
            "{label}Derivation path:{label:#} {val}{path}{val:#}",
            label = label,
            val = val_accent,
            path = wallet.derivation_path
        );
    }

    // Mnemonic in a single line, without numbering
    anstream::println!(
        "{label}Mnemonic:{label:#} {val}{mnem}{val:#}",
        label = label,
        val = val_accent,
        mnem = wallet.mnemonic
    );

    anstream::println!(
        "{label}Stats:{label:#} tries {} | time {} | speed {}/s",
        fmt_int(tries),
        fmt_duration(elapsed),
        fmt_rate(rate),
        label = label
    );
}
