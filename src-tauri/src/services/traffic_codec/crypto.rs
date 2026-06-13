use std::collections::HashMap;
use std::io::Read;

use base64::Engine;
use cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyInit, KeyIvInit};
use flate2::read::{DeflateDecoder, DeflateEncoder, GzDecoder, GzEncoder};
use flate2::Compression;

#[derive(Debug, Clone, Copy)]
pub enum CodecDirection {
    Decode,
    Encode,
}

pub fn execute_builtin_codec(
    codec: &str,
    input: &[u8],
    direction: CodecDirection,
    config: &HashMap<String, String>,
) -> Result<Vec<u8>, String> {
    match codec {
        "base64" => base64_codec(input, direction, config),
        "url" => url_codec(input, direction),
        "hex" => hex_codec(input, direction),
        "aes-cbc" => aes_cbc_codec(input, direction, config),
        "aes-ecb" => aes_ecb_codec(input, direction, config),
        "gzip" => gzip_codec(input, direction),
        "deflate" => deflate_codec(input, direction),
        "xor" => xor_codec(input, direction, config),
        "sm4" => sm4_codec(input, direction, config),
        "des" => des_codec(input, direction, config),
        "3des" => tdes_codec(input, direction, config),
        "aes-gcm" => aes_gcm_codec(input, direction, config),
        "rsa-pkcs1v15" => rsa_pkcs1_codec(input, direction, config),
        "rsa-oaep" => rsa_oaep_codec(input, direction, config),
        other => Err(format!("unknown codec: {other}")),
    }
}

fn parse_key_bytes(config: &HashMap<String, String>, key_name: &str) -> Result<Vec<u8>, String> {
    let value = config
        .get(key_name)
        .ok_or_else(|| format!("missing config key: {key_name}"))?;

    let format_key = format!("{key_name}_format");
    let format = config
        .get(&format_key)
        .map(String::as_str)
        .unwrap_or("utf8");

    parse_bytes_with_format(value, format, key_name)
}

fn parse_bytes_with_format(value: &str, format: &str, label: &str) -> Result<Vec<u8>, String> {
    match format {
        "hex" => hex::decode(value.trim())
            .map_err(|e| format!("invalid hex for {label}: {e}")),
        "base64" => base64::engine::general_purpose::STANDARD
            .decode(value.trim())
            .or_else(|_| {
                base64::engine::general_purpose::STANDARD_NO_PAD.decode(value.trim())
            })
            .map_err(|e| format!("invalid base64 for {label}: {e}")),
        "utf8" | _ => Ok(value.as_bytes().to_vec()),
    }
}

fn base64_codec(
    input: &[u8],
    direction: CodecDirection,
    config: &HashMap<String, String>,
) -> Result<Vec<u8>, String> {
    let variant = config.get("variant").map(String::as_str).unwrap_or("standard");

    match direction {
        CodecDirection::Encode => {
            let encoded = match variant {
                "url-safe" | "urlsafe" | "url_safe" => {
                    base64::engine::general_purpose::URL_SAFE.encode(input)
                }
                "url-safe-no-pad" | "urlsafe_no_pad" => {
                    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(input)
                }
                _ => base64::engine::general_purpose::STANDARD.encode(input),
            };
            Ok(encoded.into_bytes())
        }
        CodecDirection::Decode => {
            let input_str = std::str::from_utf8(input)
                .map_err(|e| format!("base64 decode requires valid UTF-8 input: {e}"))?;
            let trimmed = input_str.trim();
            let decoded = match variant {
                "url-safe" | "urlsafe" | "url_safe" => {
                    base64::engine::general_purpose::URL_SAFE.decode(trimmed)
                }
                "url-safe-no-pad" | "urlsafe_no_pad" => {
                    base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(trimmed)
                }
                _ => base64::engine::general_purpose::STANDARD
                    .decode(trimmed)
                    .or_else(|_| {
                        base64::engine::general_purpose::STANDARD_NO_PAD.decode(trimmed)
                    }),
            };
            decoded.map_err(|e| format!("base64 decode failed: {e}"))
        }
    }
}

fn url_codec(input: &[u8], direction: CodecDirection) -> Result<Vec<u8>, String> {
    match direction {
        CodecDirection::Encode => Ok(urlencoding::encode_binary(input).into_owned().into_bytes()),
        CodecDirection::Decode => {
            let input_str = std::str::from_utf8(input)
                .map_err(|e| format!("url decode requires valid UTF-8 input: {e}"))?;
            urlencoding::decode(input_str)
                .map(|decoded| decoded.into_owned().into_bytes())
                .map_err(|e| format!("url decode failed: {e}"))
        }
    }
}

fn hex_codec(input: &[u8], direction: CodecDirection) -> Result<Vec<u8>, String> {
    match direction {
        CodecDirection::Encode => Ok(hex::encode(input).into_bytes()),
        CodecDirection::Decode => {
            let input_str = std::str::from_utf8(input)
                .map_err(|e| format!("hex decode requires valid UTF-8 input: {e}"))?;
            hex::decode(input_str.trim())
                .map_err(|e| format!("hex decode failed: {e}"))
        }
    }
}

fn aes_cbc_codec(
    input: &[u8],
    direction: CodecDirection,
    config: &HashMap<String, String>,
) -> Result<Vec<u8>, String> {
    let key = parse_key_bytes(config, "key")?;
    let iv = parse_key_bytes(config, "iv")?;

    if iv.len() != 16 {
        return Err(format!("IV must be 16 bytes, got {}", iv.len()));
    }

    match key.len() {
        16 => aes_cbc_128(&key, &iv, input, direction),
        24 => aes_cbc_192(&key, &iv, input, direction),
        32 => aes_cbc_256(&key, &iv, input, direction),
        len => Err(format!(
            "AES key must be 16, 24, or 32 bytes, got {len}"
        )),
    }
}

fn aes_cbc_128(
    key: &[u8],
    iv: &[u8],
    input: &[u8],
    direction: CodecDirection,
) -> Result<Vec<u8>, String> {
    type Enc = cbc::Encryptor<aes::Aes128>;
    type Dec = cbc::Decryptor<aes::Aes128>;

    match direction {
        CodecDirection::Encode => Ok(Enc::new(key.into(), iv.into())
            .encrypt_padded_vec_mut::<Pkcs7>(input)),
        CodecDirection::Decode => Dec::new(key.into(), iv.into())
            .decrypt_padded_vec_mut::<Pkcs7>(input)
            .map_err(|e| format!("AES-CBC decrypt failed: {e}")),
    }
}

fn aes_cbc_192(
    key: &[u8],
    iv: &[u8],
    input: &[u8],
    direction: CodecDirection,
) -> Result<Vec<u8>, String> {
    type Enc = cbc::Encryptor<aes::Aes192>;
    type Dec = cbc::Decryptor<aes::Aes192>;

    match direction {
        CodecDirection::Encode => Ok(Enc::new(key.into(), iv.into())
            .encrypt_padded_vec_mut::<Pkcs7>(input)),
        CodecDirection::Decode => Dec::new(key.into(), iv.into())
            .decrypt_padded_vec_mut::<Pkcs7>(input)
            .map_err(|e| format!("AES-CBC decrypt failed: {e}")),
    }
}

fn aes_cbc_256(
    key: &[u8],
    iv: &[u8],
    input: &[u8],
    direction: CodecDirection,
) -> Result<Vec<u8>, String> {
    type Enc = cbc::Encryptor<aes::Aes256>;
    type Dec = cbc::Decryptor<aes::Aes256>;

    match direction {
        CodecDirection::Encode => Ok(Enc::new(key.into(), iv.into())
            .encrypt_padded_vec_mut::<Pkcs7>(input)),
        CodecDirection::Decode => Dec::new(key.into(), iv.into())
            .decrypt_padded_vec_mut::<Pkcs7>(input)
            .map_err(|e| format!("AES-CBC decrypt failed: {e}")),
    }
}

fn aes_ecb_codec(
    input: &[u8],
    direction: CodecDirection,
    config: &HashMap<String, String>,
) -> Result<Vec<u8>, String> {
    let key = parse_key_bytes(config, "key")?;

    match key.len() {
        16 => aes_ecb_128(&key, input, direction),
        24 => aes_ecb_192(&key, input, direction),
        32 => aes_ecb_256(&key, input, direction),
        len => Err(format!(
            "AES key must be 16, 24, or 32 bytes, got {len}"
        )),
    }
}

fn aes_ecb_128(key: &[u8], input: &[u8], direction: CodecDirection) -> Result<Vec<u8>, String> {
    type Enc = ecb::Encryptor<aes::Aes128>;
    type Dec = ecb::Decryptor<aes::Aes128>;

    match direction {
        CodecDirection::Encode => Ok(Enc::new(key.into())
            .encrypt_padded_vec_mut::<Pkcs7>(input)),
        CodecDirection::Decode => Dec::new(key.into())
            .decrypt_padded_vec_mut::<Pkcs7>(input)
            .map_err(|e| format!("AES-ECB decrypt failed: {e}")),
    }
}

fn aes_ecb_192(key: &[u8], input: &[u8], direction: CodecDirection) -> Result<Vec<u8>, String> {
    type Enc = ecb::Encryptor<aes::Aes192>;
    type Dec = ecb::Decryptor<aes::Aes192>;

    match direction {
        CodecDirection::Encode => Ok(Enc::new(key.into())
            .encrypt_padded_vec_mut::<Pkcs7>(input)),
        CodecDirection::Decode => Dec::new(key.into())
            .decrypt_padded_vec_mut::<Pkcs7>(input)
            .map_err(|e| format!("AES-ECB decrypt failed: {e}")),
    }
}

fn aes_ecb_256(key: &[u8], input: &[u8], direction: CodecDirection) -> Result<Vec<u8>, String> {
    type Enc = ecb::Encryptor<aes::Aes256>;
    type Dec = ecb::Decryptor<aes::Aes256>;

    match direction {
        CodecDirection::Encode => Ok(Enc::new(key.into())
            .encrypt_padded_vec_mut::<Pkcs7>(input)),
        CodecDirection::Decode => Dec::new(key.into())
            .decrypt_padded_vec_mut::<Pkcs7>(input)
            .map_err(|e| format!("AES-ECB decrypt failed: {e}")),
    }
}

fn gzip_codec(input: &[u8], direction: CodecDirection) -> Result<Vec<u8>, String> {
    match direction {
        CodecDirection::Encode => {
            let mut encoder = GzEncoder::new(input, Compression::default());
            let mut output = Vec::new();
            encoder
                .read_to_end(&mut output)
                .map_err(|e| format!("gzip compress failed: {e}"))?;
            Ok(output)
        }
        CodecDirection::Decode => {
            let mut decoder = GzDecoder::new(input);
            let mut output = Vec::new();
            decoder
                .read_to_end(&mut output)
                .map_err(|e| format!("gzip decompress failed: {e}"))?;
            Ok(output)
        }
    }
}

fn deflate_codec(input: &[u8], direction: CodecDirection) -> Result<Vec<u8>, String> {
    match direction {
        CodecDirection::Encode => {
            let mut encoder = DeflateEncoder::new(input, Compression::default());
            let mut output = Vec::new();
            encoder
                .read_to_end(&mut output)
                .map_err(|e| format!("deflate compress failed: {e}"))?;
            Ok(output)
        }
        CodecDirection::Decode => {
            let mut decoder = DeflateDecoder::new(input);
            let mut output = Vec::new();
            decoder
                .read_to_end(&mut output)
                .map_err(|e| format!("deflate decompress failed: {e}"))?;
            Ok(output)
        }
    }
}

fn xor_codec(
    input: &[u8],
    _direction: CodecDirection,
    config: &HashMap<String, String>,
) -> Result<Vec<u8>, String> {
    let key = parse_key_bytes(config, "key")?;
    if key.is_empty() {
        return Err("XOR key must not be empty".to_string());
    }

    Ok(input
        .iter()
        .enumerate()
        .map(|(index, byte)| byte ^ key[index % key.len()])
        .collect())
}

fn sm4_codec(
    input: &[u8],
    direction: CodecDirection,
    config: &HashMap<String, String>,
) -> Result<Vec<u8>, String> {
    use cbc::{Decryptor, Encryptor};
    use cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
    use sm4::Sm4;

    let key = parse_key_bytes(config, "key")?;
    if key.len() != 16 {
        return Err(format!("SM4 key must be 16 bytes, got {}", key.len()));
    }
    let iv = parse_key_bytes(config, "iv")?;
    if iv.len() != 16 {
        return Err(format!("SM4 IV must be 16 bytes, got {}", iv.len()));
    }

    type Sm4CbcEnc = Encryptor<Sm4>;
    type Sm4CbcDec = Decryptor<Sm4>;

    match direction {
        CodecDirection::Encode => Ok(Sm4CbcEnc::new(key.as_slice().into(), iv.as_slice().into())
            .encrypt_padded_vec_mut::<Pkcs7>(input)),
        CodecDirection::Decode => Sm4CbcDec::new(key.as_slice().into(), iv.as_slice().into())
            .decrypt_padded_vec_mut::<Pkcs7>(input)
            .map_err(|e| format!("SM4 decrypt failed: {e}")),
    }
}

fn des_codec(
    input: &[u8],
    direction: CodecDirection,
    config: &HashMap<String, String>,
) -> Result<Vec<u8>, String> {
    use cbc::{Decryptor, Encryptor};
    use cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
    use des::Des;

    let key = parse_key_bytes(config, "key")?;
    if key.len() != 8 {
        return Err(format!("DES key must be 8 bytes, got {}", key.len()));
    }
    let iv = parse_key_bytes(config, "iv")?;
    if iv.len() != 8 {
        return Err(format!("DES IV must be 8 bytes, got {}", iv.len()));
    }

    type DesCbcEnc = Encryptor<Des>;
    type DesCbcDec = Decryptor<Des>;

    match direction {
        CodecDirection::Encode => Ok(DesCbcEnc::new(key.as_slice().into(), iv.as_slice().into())
            .encrypt_padded_vec_mut::<Pkcs7>(input)),
        CodecDirection::Decode => DesCbcDec::new(key.as_slice().into(), iv.as_slice().into())
            .decrypt_padded_vec_mut::<Pkcs7>(input)
            .map_err(|e| format!("DES decrypt failed: {e}")),
    }
}

fn tdes_codec(
    input: &[u8],
    direction: CodecDirection,
    config: &HashMap<String, String>,
) -> Result<Vec<u8>, String> {
    use cbc::{Decryptor, Encryptor};
    use cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
    use des::TdesEde3;

    let key = parse_key_bytes(config, "key")?;
    if key.len() != 24 {
        return Err(format!("3DES key must be 24 bytes, got {}", key.len()));
    }
    let iv = parse_key_bytes(config, "iv")?;
    if iv.len() != 8 {
        return Err(format!("3DES IV must be 8 bytes, got {}", iv.len()));
    }

    type TdesCbcEnc = Encryptor<TdesEde3>;
    type TdesCbcDec = Decryptor<TdesEde3>;

    match direction {
        CodecDirection::Encode => Ok(TdesCbcEnc::new(key.as_slice().into(), iv.as_slice().into())
            .encrypt_padded_vec_mut::<Pkcs7>(input)),
        CodecDirection::Decode => TdesCbcDec::new(key.as_slice().into(), iv.as_slice().into())
            .decrypt_padded_vec_mut::<Pkcs7>(input)
            .map_err(|e| format!("3DES decrypt failed: {e}")),
    }
}

fn aes_gcm_codec(
    input: &[u8],
    direction: CodecDirection,
    config: &HashMap<String, String>,
) -> Result<Vec<u8>, String> {
    use aes_gcm::{
        aead::{Aead, KeyInit, Payload},
        Aes128Gcm, Aes256Gcm,
    };

    let key = parse_key_bytes(config, "key")?;
    let nonce = parse_key_bytes(config, "iv").or_else(|_| parse_key_bytes(config, "nonce"))?;
    if nonce.len() != 12 {
        return Err(format!("AES-GCM nonce must be 12 bytes, got {}", nonce.len()));
    }

    let aad = config
        .get("aad")
        .map(|value| value.as_bytes().to_vec())
        .unwrap_or_default();
    let payload = Payload {
        msg: input,
        aad: &aad,
    };

    match (direction, key.len()) {
        (CodecDirection::Encode, 16) => {
            let cipher = Aes128Gcm::new_from_slice(&key)
                .map_err(|e| format!("AES-GCM init failed: {e}"))?;
            cipher
                .encrypt(nonce.as_slice().into(), payload)
                .map_err(|e| format!("AES-GCM encrypt failed: {e}"))
        }
        (CodecDirection::Decode, 16) => {
            let cipher = Aes128Gcm::new_from_slice(&key)
                .map_err(|e| format!("AES-GCM init failed: {e}"))?;
            cipher
                .decrypt(nonce.as_slice().into(), payload)
                .map_err(|e| format!("AES-GCM decrypt failed: {e}"))
        }
        (CodecDirection::Encode, 32) => {
            let cipher = Aes256Gcm::new_from_slice(&key)
                .map_err(|e| format!("AES-GCM init failed: {e}"))?;
            cipher
                .encrypt(nonce.as_slice().into(), payload)
                .map_err(|e| format!("AES-GCM encrypt failed: {e}"))
        }
        (CodecDirection::Decode, 32) => {
            let cipher = Aes256Gcm::new_from_slice(&key)
                .map_err(|e| format!("AES-GCM init failed: {e}"))?;
            cipher
                .decrypt(nonce.as_slice().into(), payload)
                .map_err(|e| format!("AES-GCM decrypt failed: {e}"))
        }
        (_, len) => Err(format!(
            "AES-GCM key must be 16 or 32 bytes, got {len}"
        )),
    }
}

fn parse_rsa_key_pem(config: &HashMap<String, String>, key_name: &str) -> Result<String, String> {
    config
        .get(key_name)
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("missing RSA {key_name}"))
}

fn rsa_pkcs1_codec(
    input: &[u8],
    direction: CodecDirection,
    config: &HashMap<String, String>,
) -> Result<Vec<u8>, String> {
    use rsa::pkcs1::DecodeRsaPrivateKey;
    use rsa::pkcs1v15::{DecryptingKey, EncryptingKey};
    use rsa::pkcs8::DecodePublicKey;
    use rsa::traits::{Decryptor, RandomizedEncryptor};
    use rsa::{RsaPrivateKey, RsaPublicKey};

    match direction {
        CodecDirection::Encode => {
            let pem = parse_rsa_key_pem(config, "public_key")?;
            let public_key = RsaPublicKey::from_public_key_pem(&pem)
                .map_err(|e| format!("invalid RSA public key: {e}"))?;
            let encrypting_key = EncryptingKey::new(public_key);
            encrypting_key
                .encrypt_with_rng(&mut rand::thread_rng(), input)
                .map_err(|e| format!("RSA PKCS1v15 encrypt failed: {e}"))
        }
        CodecDirection::Decode => {
            let pem = parse_rsa_key_pem(config, "private_key")?;
            let private_key = RsaPrivateKey::from_pkcs1_pem(&pem)
                .map_err(|e| format!("invalid RSA private key: {e}"))?;
            let decrypting_key = DecryptingKey::new(private_key);
            decrypting_key
                .decrypt(input)
                .map_err(|e| format!("RSA PKCS1v15 decrypt failed: {e}"))
        }
    }
}

fn rsa_oaep_codec(
    input: &[u8],
    direction: CodecDirection,
    config: &HashMap<String, String>,
) -> Result<Vec<u8>, String> {
    use rsa::oaep::Oaep;
    use rsa::pkcs1::DecodeRsaPrivateKey;
    use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};
    use rsa::{RsaPrivateKey, RsaPublicKey};
    use sha2::Sha256;

    let hash_name = config.get("hash").map(String::as_str).unwrap_or("sha256");
    if hash_name != "sha256" {
        return Err(format!("unsupported RSA-OAEP hash: {hash_name}"));
    }

    match direction {
        CodecDirection::Encode => {
            let pem = parse_rsa_key_pem(config, "public_key")?;
            let public_key = RsaPublicKey::from_public_key_pem(&pem)
                .map_err(|e| format!("invalid RSA public key: {e}"))?;
            public_key
                .encrypt(&mut rand::thread_rng(), Oaep::new::<Sha256>(), input)
                .map_err(|e| format!("RSA-OAEP encrypt failed: {e}"))
        }
        CodecDirection::Decode => {
            let pem = parse_rsa_key_pem(config, "private_key")?;
            let private_key = RsaPrivateKey::from_pkcs8_pem(&pem)
                .or_else(|_| RsaPrivateKey::from_pkcs1_pem(&pem))
                .map_err(|e| format!("invalid RSA private key: {e}"))?;
            private_key
                .decrypt(Oaep::new::<Sha256>(), input)
                .map_err(|e| format!("RSA-OAEP decrypt failed: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn base64_roundtrip() {
        let input = b"hello world";
        let config = HashMap::new();
        let encoded = execute_builtin_codec("base64", input, CodecDirection::Encode, &config)
            .unwrap();
        let decoded =
            execute_builtin_codec("base64", &encoded, CodecDirection::Decode, &config).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn xor_is_symmetric() {
        let input = b"secret payload";
        let mut config = HashMap::new();
        config.insert("key".to_string(), "mykey".to_string());
        config.insert("key_format".to_string(), "utf8".to_string());

        let encoded = execute_builtin_codec("xor", input, CodecDirection::Encode, &config).unwrap();
        let decoded =
            execute_builtin_codec("xor", &encoded, CodecDirection::Decode, &config).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn rsa_requires_key_config() {
        let config = HashMap::new();
        let result = execute_builtin_codec("rsa-oaep", b"data", CodecDirection::Encode, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing RSA"));
    }
}
