use tiny_hderive::bip32::ExtendedPrivKey;

pub fn derivate_seed_to_private(seed: &[u8; 64], child_index: u32) -> [u8; 32] {
    let derivation_path = "m/44'/195'/0'/0/".to_string() + &child_index.to_string();
    let child_key = ExtendedPrivKey::derive(seed, derivation_path.as_str()).expect("Error deriving key");
    let private_key_bytes = child_key.secret();
    private_key_bytes
}

pub fn private_key_to_public(private_key: &[u8; 32]) -> [u8; 65] {
    let secp256k1 = secp256k1::Secp256k1::new();
    let secret_key = secp256k1::SecretKey::from_byte_array(*private_key).expect("Error getting secret key");
    let public_key = secp256k1::PublicKey::from_secret_key(&secp256k1, &secret_key);
    let public_key_bytes = public_key.serialize_uncompressed();
    public_key_bytes
}