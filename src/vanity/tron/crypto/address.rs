use sha2::Sha256;
use sha3::{Digest, Keccak256};

pub fn public_key_to_address(uncompressed_public_key: &[u8; 65]) -> String {
    let pub_key_no_prefix = &uncompressed_public_key[1..];
    let mut hasher = Keccak256::new();
    hasher.update(pub_key_no_prefix);
    let hashed_public_key = hasher.finalize();

    let address_bytes = &hashed_public_key[hashed_public_key.len() - 20..];
    const TRON_ADDRESS_PREFIX: u8 = 0x41;
    let mut address_with_prefix = vec![TRON_ADDRESS_PREFIX];
    address_with_prefix.extend_from_slice(address_bytes);

    let h1 = Sha256::digest(&address_with_prefix);
    let h2 = Sha256::digest(&h1);
    let checksum = &h2[0..4];

    let mut final_address_bytes = address_with_prefix;
    final_address_bytes.extend_from_slice(checksum);
    bs58::encode(final_address_bytes).into_string()
}

#[cfg(test)]
mod tests {
    use super::public_key_to_address;
    use crate::vanity::tron::crypto::keys::{derivate_seed_to_private, private_key_to_public};
    use crate::vanity::tron::crypto::mnemonic::mnemonic_to_seed;
    use bip39::Mnemonic;

    #[test]
    fn derives_known_tron_address_with_empty_passphrase() {
        let mnemonic = Mnemonic::parse(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        )
        .unwrap();
        let seed = mnemonic_to_seed(&mnemonic, "");
        let private_key = derivate_seed_to_private(&seed, 0);
        let public_key = private_key_to_public(&private_key);

        assert_eq!(
            public_key_to_address(&public_key),
            "TUEZSdKsoDHQMeZwihtdoBiN46zxhGWYdH"
        );
    }
}
