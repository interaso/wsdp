use std::fmt;

use bytes::Bytes;

use crate::{DownloadedReport, ReportId, ReportMetadata, ServiceMessage};

/// A downloaded binary report, normally a PDF, with its metadata and original bytes.
#[derive(Clone, Eq, PartialEq)]
pub struct BinaryReport {
    /// Metadata returned with the downloaded report.
    pub metadata: ReportMetadata,
    /// All non-error messages returned during download.
    pub messages: Vec<ServiceMessage>,
    bytes: Bytes,
}

impl BinaryReport {
    pub(crate) fn from_downloaded(report: DownloadedReport) -> Self {
        Self {
            metadata: report.metadata,
            messages: report.messages,
            bytes: report.bytes,
        }
    }

    /// Returns the report ID retained for later lifecycle operations.
    pub fn id(&self) -> &ReportId {
        self.metadata.id()
    }

    /// Returns the untouched downloaded bytes.
    pub fn as_bytes(&self) -> &Bytes {
        &self.bytes
    }

    /// Consumes the wrapper and returns the untouched downloaded bytes.
    pub fn into_bytes(self) -> Bytes {
        self.bytes
    }
}

impl fmt::Debug for BinaryReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BinaryReport")
            .field("message_count", &self.messages.len())
            .field("byte_len", &self.bytes.len())
            .finish()
    }
}
