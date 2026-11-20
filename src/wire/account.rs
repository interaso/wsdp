use quick_xml::Writer;

use super::soap::{self, XmlResult};
use crate::NewPassword;
use crate::client::Operation;
use crate::model::ServiceKind;

pub(crate) fn change_password_operation() -> Operation {
    Operation::new(
        ServiceKind::Account,
        "zmenHeslo",
        "ZmenHesloRequest",
        "ZmenHesloResponse",
    )
}

pub(crate) fn write_change_password(
    writer: &mut Writer<Vec<u8>>,
    password: &NewPassword,
) -> XmlResult {
    use secrecy::ExposeSecret as _;
    soap::text_element(writer, "noveHeslo", password.0.expose_secret())
}
