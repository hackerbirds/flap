use rand::{TryRngCore, rngs::OsRng};

pub mod encryption_stream;
pub mod file_key;
pub mod master_key;
pub mod nonce;
pub mod transfer_id;
pub mod x25519;

pub fn random_array<const N: usize>() -> [u8; N] {
    let mut array = [0u8; N];

    OsRng
        .try_fill_bytes(array.as_mut_slice())
        .expect("OS rng is always available");

    array
}
