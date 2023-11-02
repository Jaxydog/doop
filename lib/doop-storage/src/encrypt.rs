use std::io::{Read, Write};

use argon2::password_hash::rand_core::RngCore;
use argon2::Argon2;
use chacha20poly1305::aead::stream::{DecryptorBE32, EncryptorBE32};
use chacha20poly1305::aead::OsRng;
use chacha20poly1305::{KeyInit, XChaCha20Poly1305};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::{Error, Format};

/// An error that may occur during encryption or decryption.
#[derive(Debug, thiserror::Error)]
pub enum EncryptError<F: Format> {
    /// A format error.
    #[error(transparent)]
    Crate(#[from] crate::Error<F>),
    /// An error during environment variable fetching.
    #[error(transparent)]
    Var(#[from] std::env::VarError),
    /// An error during password hashing.
    #[error("{0}")]
    Argon(argon2::Error),
    /// An error during random number generation.
    #[error("{0}")]
    Random(argon2::password_hash::rand_core::Error),
    /// An error during encryption or decription.
    #[error("{0}")]
    Encrypt(chacha20poly1305::Error),
}

/// A format that automatically encrypts and decrypts data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Encrypt<F: Format>(F);

impl<F: Format> Encrypt<F> {
    /// Creates a new [`Encrypt<F>`].
    pub const fn new(format: F) -> Self {
        Self(format)
    }

    /// Returns a reference to the inner [`Format`] of this [`Encrypt<F>`].
    pub const fn inner(&self) -> &F {
        &self.0
    }

    /// Returns the encryption password from the environment.
    ///
    /// This can be configured using the `ENCRYPTION_KEY` environment variable.
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn password() -> Result<Box<[u8]>, std::env::VarError> {
        Ok(std::env::var("ENCRYPTION_KEY")?.into_bytes().into_boxed_slice())
    }
}

impl<F: Format> Format for Encrypt<F> {
    type EncodingError = EncryptError<F>;
    type DecodingError = EncryptError<F>;

    fn extension(&self) -> String {
        self.inner().extension() + "z"
    }

    fn encode<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, Self::EncodingError> {
        let password = Self::password()?;
        let config = Argon2::default();

        let mut salt = [0; 32];
        let mut nonce = [0; 19];
        let mut hash = [0; 32];

        OsRng.try_fill_bytes(&mut salt).map_err(EncryptError::Random)?;
        OsRng.try_fill_bytes(&mut nonce).map_err(EncryptError::Random)?;

        config.hash_password_into(&password, &salt, &mut hash).map_err(EncryptError::Argon)?;

        let cipher = XChaCha20Poly1305::new(&hash.into());

        hash.zeroize();

        let bytes = self.inner().encode(value).map_err(Error::Encoding)?;
        let stream = EncryptorBE32::from_aead(cipher, &nonce.into());
        let mut buffer = vec![0u8; salt.len() + nonce.len() + bytes.len()];
        let bytes = stream.encrypt_last(&(*bytes)).map_err(EncryptError::Encrypt)?;

        buffer.write_all(&salt).map_err(Error::from)?;
        salt.zeroize();
        buffer.write_all(&nonce).map_err(Error::from)?;
        nonce.zeroize();
        buffer.write_all(&bytes).map_err(Error::from)?;

        Ok(buffer)
    }

    fn decode<T: for<'de> Deserialize<'de>>(
        &self,
        mut bytes: &[u8],
    ) -> Result<T, Self::DecodingError> {
        let mut salt = [0; 32];
        let mut nonce = [0; 19];

        bytes.read_exact(&mut salt).map_err(Error::from)?;
        bytes.read_exact(&mut nonce).map_err(Error::from)?;

        let password = Self::password()?;
        let config = Argon2::default();
        let mut hash = [0; 32];

        config.hash_password_into(&password, &salt, &mut hash).map_err(EncryptError::Argon)?;

        let cipher = XChaCha20Poly1305::new(&hash.into());

        hash.zeroize();

        let stream = DecryptorBE32::from_aead(cipher, &nonce.into());
        let bytes = stream.decrypt_last(bytes).map_err(EncryptError::Encrypt)?;

        salt.zeroize();
        nonce.zeroize();

        self.inner().decode(&bytes).map_err(|e| Error::Decoding(e).into())
    }
}
