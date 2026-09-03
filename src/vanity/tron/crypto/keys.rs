use std::borrow::Cow;

use tiny_hderive::bip32::ExtendedPrivKey;

pub const DEFAULT_TRON_DERIVATION_PATH: &str = "m/44'/195'/0'/0/0";

thread_local! {
    static SECP256K1: secp256k1::Secp256k1<secp256k1::All> = secp256k1::Secp256k1::new();
}

pub fn tron_derivation_path(child_index: u32) -> Cow<'static, str> {
    if child_index == 0 {
        Cow::Borrowed(DEFAULT_TRON_DERIVATION_PATH)
    } else {
        Cow::Owned(format!("m/44'/195'/0'/0/{child_index}"))
    }
}

pub fn derivate_seed_to_private(seed: &[u8; 64], child_index: u32) -> Option<[u8; 32]> {
    let derivation_path = tron_derivation_path(child_index);
    ExtendedPrivKey::derive(seed, derivation_path.as_ref())
        .ok()
        .map(|child_key| child_key.secret())
}

pub fn private_key_to_public(private_key: &[u8; 32]) -> Option<[u8; 65]> {
    let secret_key = secp256k1::SecretKey::from_byte_array(*private_key).ok()?;
    Some(SECP256K1.with(|secp256k1| {
        secp256k1::PublicKey::from_secret_key(secp256k1, &secret_key).serialize_uncompressed()
    }))
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_TRON_DERIVATION_PATH, private_key_to_public, tron_derivation_path};
    use std::borrow::Cow;

    #[test]
    fn child_zero_path_is_borrowed() {
        assert_eq!(
            tron_derivation_path(0),
            Cow::Borrowed(DEFAULT_TRON_DERIVATION_PATH)
        );
    }

    #[test]
    fn invalid_private_key_is_rejected_without_panic() {
        assert_eq!(private_key_to_public(&[0u8; 32]), None);
    }
}
