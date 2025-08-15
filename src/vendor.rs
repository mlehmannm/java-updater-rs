//! Vendor.
//!
//! This module contains the vendor.

// The id for Azul as vendor.
#[cfg(feature = "azul")]
#[doc(hidden)]
const AZUL_ID: &str = "azul";

// The name for Azul as vendor.
#[cfg(feature = "azul")]
#[doc(hidden)]
const AZUL_NAME: &str = "Azul";

// The id for Eclipse as vendor.
#[cfg(feature = "eclipse")]
#[doc(hidden)]
const ECLIPSE_ID: &str = "eclipse";

// The name for Eclipse as vendor.
#[cfg(feature = "eclipse")]
#[doc(hidden)]
const ECLIPSE_NAME: &str = "Eclipse";

/// Enumeration of supported vendors.
#[derive(Debug)]
pub(crate) enum Vendor {
    #[cfg(feature = "azul")]
    /// Azul
    Azul,
    #[cfg(feature = "eclipse")]
    /// Eclipse
    Eclipse,
}

impl Vendor {
    /// Returns the id of the vendor.
    pub(crate) fn id(&self) -> &str {
        match self {
            #[cfg(feature = "azul")]
            Self::Azul => AZUL_ID,
            #[cfg(feature = "eclipse")]
            Self::Eclipse => ECLIPSE_ID,
            #[expect(unreachable_patterns)]
            _ => unreachable!(),
        }
    }

    /// Returns the name of the vendor.
    pub(crate) fn name(&self) -> &str {
        match self {
            #[cfg(feature = "azul")]
            Self::Azul => AZUL_NAME,
            #[cfg(feature = "eclipse")]
            Self::Eclipse => ECLIPSE_NAME,
            #[expect(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}

impl std::fmt::Display for Vendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl TryFrom<&str> for Vendor {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value = value.trim().to_lowercase();
        match value.as_str() {
            #[cfg(feature = "azul")]
            AZUL_ID => Ok(Self::Azul),
            #[cfg(feature = "eclipse")]
            ECLIPSE_ID => Ok(Self::Eclipse),
            _ => Err("unsupported vendor"),
        }
    }
}
