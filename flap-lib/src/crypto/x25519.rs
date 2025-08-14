//! Utility functions to convert a ed25519 key pair (from iroh) into an X25519 key pair for use with Noise

use iroh::{PublicKey, SecretKey};

pub fn iroh_secret_to_x25519_secret(iroh_secret_key: &SecretKey) -> x25519_dalek::StaticSecret {
    let ed25519_secret = iroh_secret_key.secret();
    let x25519_secret = x25519_dalek::StaticSecret::from(ed25519_secret.to_scalar_bytes());

    x25519_secret
}

pub fn iroh_public_to_x25519_public(iroh_public_key: &PublicKey) -> x25519_dalek::PublicKey {
    let ed25519_public = iroh_public_key.public();

    let x25519_public = x25519_dalek::PublicKey::from(ed25519_public.to_montgomery().0);

    x25519_public
}
