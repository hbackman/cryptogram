use ed25519_dalek::Signature;
use ed25519_dalek::VerifyingKey;
use thiserror::Error;

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
