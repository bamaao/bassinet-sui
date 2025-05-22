use ed25519_dalek::{ed25519::signature::SignerMut, Signature, SigningKey, VerifyingKey};
use hex::FromHex;

pub fn ed25519_dalek() {
    let mut csprng = rand::rngs::OsRng;
    let mut signing_key = SigningKey::generate(&mut csprng);
    let message = uuid::Uuid::new_v4().to_string();
    println!("message: {}", hex::encode(message.as_bytes()));
    let sign = signing_key.sign(message.as_bytes());
    println!("sign:{}", hex::encode(sign.to_bytes()));
    let verifying_key = signing_key.verifying_key();
    println!("verifying_key: {:?}", hex::encode(verifying_key.to_bytes()));
    println!("private_key:{:?}", hex::encode(signing_key.as_bytes()))
}

pub fn validate() {
    let decode = <[u8;32]>::from_hex("af9503084802a3da2266a45e75d9d076f640c041ba926a2e4c60f6bd5c2b1bfc").unwrap();
    let verifying_key = VerifyingKey::from_bytes(&decode).unwrap();
    let sign_bytes = hex::decode("8cceb3b69c43e909eda1bd482f7c892443566eb0434194feffcf5bebfa22d3134d6295bb301921ad3375e70211d55872456f920013d6680a949b89d95a563308").unwrap();
    let signature = Signature::from_slice(&sign_bytes.as_slice()).unwrap();
    let verifying_result = verifying_key.verify_strict(hex::decode("62333430343161622d643661642d343635652d616463312d363764643466356537653939").unwrap().as_slice(), &signature);
    if verifying_result.is_err() {
        println!("error:{:?}", verifying_result.err());
    }else {
        println!("验证通过");
    }
}