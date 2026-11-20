use super::{LegacyOwnershipSheetReport, NtrOwnershipSheet};

/// Ownership-sheet XML technology selected explicitly by a report version.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum OwnershipSheetVersion {
    /// Presentation-shaped legacy LV 1.0 XML.
    Legacy,
    /// Namespaced NTR LV 3.0 XML.
    Ntr3,
}

/// Complete ownership-sheet payload decoded according to its actual XML root.
#[derive(Clone, Eq, PartialEq)]
pub enum OwnershipSheetReport {
    /// Legacy LV 1.0 (`VypisZKatastruNemovitosti`, no namespace).
    Legacy(LegacyOwnershipSheetReport),
    /// NTR LV 3.0 (`vypisZKatastruNemovitosti` in the NTR namespace).
    Ntr3(Box<NtrOwnershipSheet>),
}
