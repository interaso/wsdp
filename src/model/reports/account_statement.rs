use rust_decimal::Decimal;

use crate::LegacyId;

/// Complete legacy `VypisStavuZakaznickehoUctu` report.
#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub struct AccountStatementReport {
    /// Account and billing rows.
    pub account: CustomerAccount,
    /// Requested lexical start timestamp (`DATUM_OD`).
    pub from: String,
    /// Requested lexical end timestamp (`DATUM_DO`).
    pub to: String,
    /// Lexical report creator (`VYHOTOVIL`).
    pub created_by: String,
    /// Lexical report creation timestamp (`VYHOTOVENO`).
    pub created_at: String,
}

/// Customer account section of an account statement.
#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub struct CustomerAccount {
    /// Account username.
    pub username: String,
    /// Exact integral account number.
    pub number: Decimal,
    /// Proceeding identification and any direct mixed-content text.
    pub proceeding: AccountProceeding,
    /// Lexical billing date.
    pub billing_date: String,
    /// Credit exactly as supplied by the legacy string schema.
    pub credit: String,
    /// Person or organization account participant.
    pub participant: AccountParticipant,
    /// All charge groups; a present empty container becomes an empty vector.
    pub groups: Vec<AccountChargeGroup>,
    /// Exact signed total charge.
    pub total: Decimal,
}

/// Proceeding embedded in an account statement.
#[derive(Clone, Eq, PartialEq)]
#[non_exhaustive]
pub struct AccountProceeding {
    /// Direct mixed-content text retained without normalization.
    pub text: String,
    /// Proceeding type code.
    pub kind: String,
    /// Sequence number.
    pub sequence: u32,
    /// Year.
    pub year: u16,
    /// Workplace code.
    pub workplace: u16,
}

/// Person or organization associated with a customer account.
#[derive(Clone, Eq, PartialEq)]
pub enum AccountParticipant {
    /// Natural person with optional titles.
    Person {
        /// Title before the name.
        title_before: Option<String>,
        /// Family name.
        family_name: String,
        /// Given name.
        given_name: String,
        /// Title after the name.
        title_after: Option<String>,
    },
    /// Legal-person/trade name.
    Organization(String),
}

/// Named group of account charge rows.
#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub struct AccountChargeGroup {
    /// Group name.
    pub name: String,
    /// Every row in server order.
    pub rows: Vec<AccountChargeRow>,
}

/// One account charge row.
#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub struct AccountChargeRow {
    /// User who triggered the output.
    pub username: String,
    /// File number (`CJ`).
    pub file_number: String,
    /// Purpose text.
    pub purpose: String,
    /// XSD `unsignedLong` row identifier.
    pub ident: u64,
    /// Lexical start timestamp.
    pub started_at: String,
    /// Exact integral account number associated with the row.
    pub account_number: Decimal,
    /// WSDP function name.
    pub function: String,
    /// Number of units.
    pub units: u32,
    /// Legacy lexical page count.
    pub pages: String,
    /// Exact signed charge.
    pub price: Decimal,
    /// Service/result code.
    pub code: u16,
    /// Optional linked child report; a present empty ID remains distinguishable.
    pub child_report_id: Option<LegacyId>,
}
