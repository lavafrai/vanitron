use bip39::{Language, Mnemonic};

pub(crate) fn generate_mnemonic(size: usize) -> Mnemonic {
    Mnemonic::generate_in(Language::English, size).expect("Error generating mnemonic")
}

pub fn mnemonic_to_seed(mnemonic: &Mnemonic, passphrase: &str) -> [u8; 64] {
    mnemonic.to_seed(passphrase)
}
