use fastcrypto::{ed25519::Ed25519KeyPair, traits::{KeyPair, Signer, ToFromBytes}};
use rand::thread_rng;

pub fn sui_ed25519() {
    let kp = Ed25519KeyPair::generate(&mut thread_rng());
    let message = uuid::Uuid::new_v4().to_string();
    println!("message:{}", hex::encode(message.as_bytes()));
    let sign = kp.sign(message.as_bytes());
    println!("sign:{}", hex::encode(sign.sig.to_bytes()));
    println!("verifying_key:{}", hex::encode(kp.public().as_bytes()));
}