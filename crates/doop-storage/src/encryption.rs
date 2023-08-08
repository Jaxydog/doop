use std::fmt::Display;
use std::io::{Read, Write};

use argon2::password_hash::rand_core::RngCore;
use argon2::Argon2;
use chacha20poly1305::aead::stream::{DecryptorBE32, EncryptorBE32};
use chacha20poly1305::aead::OsRng;
use chacha20poly1305::{KeyInit, XChaCha20Poly1305};
use zeroize::Zeroize;

use crate::Format;

/// An error that may occur during encryption or decryption.
#[derive(Debug, thiserror::Error)]
pub enum EncryptError<F>
where
    F: Format,
    F::EncodeError: Display,
    F::DecodeError: Display,
{
    /// An error during environment variable fetching.
    #[error(transparent)]
    Var(#[from] std::env::VarError),
    /// An error during IO reading / writing.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// An error during password hashing.
    #[error("{0}")]
    Argon(argon2::Error),
    /// An error during random number generation.
    #[error("{0}")]
    Random(argon2::password_hash::rand_core::Error),
    /// An error during encryption or decription.
    #[error("{0}")]
    Encrypt(chacha20poly1305::Error),
    /// An error during encoding.
    #[error("{0}")]
    Encode(F::EncodeError),
    /// An error during decoding.
    #[error("{0}")]
    Decode(F::DecodeError),
}

/// A format that automatically encrypts and decrypts data.
///
/// It's recommended to use this in conjunction with the [`zeroize`] crate,
/// which can be used to remove sensitive data from memory.
///
/// Please use this at your own risk, as I'm not a security professional and
/// I don't really understand how encryption works.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Encrypt<F>(F)
where
    F: Format,
    F::EncodeError: Display,
    F::DecodeError: Display;

impl<F> Encrypt<F>
where
    F: Format,
    F::EncodeError: Display,
    F::DecodeError: Display,
{
    /// Creates a [`Encrypt`] format containing an inner [`Format`] value.
    #[inline]
    pub const fn new(format: F) -> Self { Self(format) }

    /// Returns a reference to the [`Encrypt`] format's inner [`Format`].
    #[inline]
    pub const fn inner(&self) -> &F { &self.0 }

    /// Returns the encryption password from the environment.
    #[inline]
    fn password() -> Result<Box<[u8]>, std::env::VarError> {
        Ok(std::env::var("ENCRYPT_KEY")?.into_bytes().into())
    }
}

#[cfg(feature = "default-systems")]
impl<F> crate::systems::FileFormat for Encrypt<F>
where
    F: crate::systems::FileFormat,
    F::EncodeError: Display,
    F::DecodeError: Display,
{
    fn extension(&self) -> String { self.inner().extension() + ".x" }
}

impl<F> Format for Encrypt<F>
where
    F: Format,
    F::EncodeError: Display,
    F::DecodeError: Display,
{
    type DecodeError = EncryptError<F>;
    type EncodeError = EncryptError<F>;

    fn decode<T>(&self, mut bytes: &[u8]) -> Result<T, Self::DecodeError>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let (mut salt, mut nonce) = ([0; 32], [0; 19]);

        bytes.read_exact(&mut salt)?;
        bytes.read_exact(&mut nonce)?;

        let password = Self::password()?;
        let config = Argon2::default();
        let mut hash = [0; 32];

        config
            .hash_password_into(&password, &salt, &mut hash)
            .map_err(EncryptError::Argon)?;

        let cipher = XChaCha20Poly1305::new(&hash.into());

        hash.zeroize();

        let stream = DecryptorBE32::from_aead(cipher, &nonce.into());
        let bytes = stream.decrypt_last(bytes).map_err(EncryptError::Encrypt)?;

        salt.zeroize();
        nonce.zeroize();

        self.inner().decode(&bytes).map_err(EncryptError::Decode)
    }

    fn encode<T>(&self, value: &T) -> Result<Vec<u8>, Self::EncodeError>
    where
        T: serde::Serialize,
    {
        let password = Self::password()?;
        let config = Argon2::default();

        let mut salt = [0; 32];
        let mut nonce = [0; 19];
        let mut hash = [0; 32];

        OsRng.try_fill_bytes(&mut salt).map_err(EncryptError::Random)?;
        OsRng.try_fill_bytes(&mut nonce).map_err(EncryptError::Random)?;

        config
            .hash_password_into(&password, &salt, &mut hash)
            .map_err(EncryptError::Argon)?;

        let cipher = XChaCha20Poly1305::new(&hash.into());

        hash.zeroize();

        let bytes = self.inner().encode(value).map_err(EncryptError::Encode)?;
        let stream = EncryptorBE32::from_aead(cipher, &nonce.into());
        let mut buffer = vec![0; salt.len() + nonce.len() + bytes.len()];

        let bytes = stream.encrypt_last(&(*bytes)).map_err(EncryptError::Encrypt)?;

        buffer.write_all(&salt)?;
        buffer.write_all(&nonce)?;
        buffer.write_all(&bytes)?;
        salt.zeroize();
        nonce.zeroize();

        Ok(buffer)
    }
}
