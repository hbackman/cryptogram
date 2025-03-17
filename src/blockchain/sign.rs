use ed25519_dalek::Signer;
use ed25519_dalek::Signature;
use ed25519_dalek::SigningKey;
use ed25519_dalek::VerifyingKey;
use rand::rngs::OsRng;
use thiserror::Error;

pub struct Keypair {
  pub signing_key: SigningKey,
  pub verifying_key: VerifyingKey,
}

impl Keypair {
  /**
   * Generate a new keypair.
   */
  pub fn new() -> Self {
    let mut csprng = OsRng;
    let sig_key: SigningKey = SigningKey::generate(&mut csprng);
    let ver_key = sig_key.verifying_key();

    Self {signing_key: sig_key, verifying_key: ver_key}
  }

  pub fn sign_message(&self, message: &str) -> String {
    let signature = self.signing_key.sign(message.as_bytes());
    hex::encode(signature.to_bytes())
  }

  pub fn get_public_key(&self) -> String {
    hex::encode(self.verifying_key.to_bytes())
  }
}

#[derive(Debug, Error)]
pub enum ValidationError {
  #[error("Failed to decode hex: {0}")]
  HexDecodeError(#[from] hex::FromHexError),
  #[error("Invalid public key length")]
  InvalidPublicKeyLength,
  #[error("Invalid signature length")]
  InvalidSignatureLength,
  #[error("Invalid public key")]
  InvalidPublicKey,
  #[error("Signature verification failed")]
  SignatureVerificationFailed,
}

pub fn validate_signature(public_key: &str, signature: &str, message: &str) -> Result<(), ValidationError> {
  // Decode public key and check length
  let public_key_bytes = hex::decode(public_key)?;
  let public_key = VerifyingKey::from_bytes(
    &public_key_bytes
      .try_into()
      .map_err(|_| ValidationError::InvalidPublicKeyLength)?,
  ).map_err(|_| ValidationError::InvalidPublicKey)?;

  // Decode signature and check length
  let signature_bytes = hex::decode(signature)?;
  let signature = Signature::from_bytes(
    &signature_bytes
      .try_into()
      .map_err(|_| ValidationError::InvalidSignatureLength)?,
  );

  public_key
    .verify_strict(message.as_bytes(), &signature)
    .map_err(|_| ValidationError::SignatureVerificationFailed)
}
