use std::fmt;

use secrecy::SecretString;

use crate::{Error, Result};

/// A validated new WSDP password with redacted debug output.
#[derive(Clone)]
pub struct NewPassword(pub(crate) SecretString);

impl NewPassword {
    /// Validates the official 1–64 character password constraint.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if !(1..=64).contains(&value.chars().count()) {
            return Err(Error::invalid_input(
                "new_password",
                "password must contain 1 to 64 characters",
            ));
        }
        Ok(Self(SecretString::new(value.into_boxed_str())))
    }
}

impl fmt::Debug for NewPassword {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NewPassword([REDACTED])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_validation_counts_characters_and_redacts_debug() {
        assert!(NewPassword::new("").is_err());
        assert!(NewPassword::new("x".repeat(65)).is_err());
        let password = NewPassword::new("žluťoučký").unwrap();
        assert!(!format!("{password:?}").contains("žluťoučký"));
    }
}
