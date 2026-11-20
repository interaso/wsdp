use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use secrecy::ExposeSecret;

use super::soap::{XmlResult, text_element};
use crate::Credentials;

const PASSWORD_TEXT: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordText";
const BASE64_BINARY: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary";

pub(crate) struct SecurityHeader {
    pub(crate) token_id: String,
    pub(crate) nonce: String,
    pub(crate) created: String,
}

impl SecurityHeader {
    pub(crate) fn fresh() -> Result<Self, getrandom::Error> {
        let mut nonce = [0_u8; 16];
        getrandom::fill(&mut nonce)?;
        let mut token = [0_u8; 16];
        getrandom::fill(&mut token)?;
        Ok(Self {
            token_id: format!("UsernameToken-{}", hex_string(&token)),
            nonce: BASE64.encode(nonce),
            created: jiff::Timestamp::now().to_string(),
        })
    }
}

pub(crate) fn write_security(
    writer: &mut Writer<Vec<u8>>,
    credentials: &Credentials,
    security: &SecurityHeader,
) -> XmlResult {
    let mut security_start = BytesStart::new("wsse:Security");
    security_start.push_attribute(("soap:mustUnderstand", "1"));
    writer.write_event(Event::Start(security_start))?;
    let mut token = BytesStart::new("wsse:UsernameToken");
    token.push_attribute(("wsu:Id", security.token_id.as_str()));
    writer.write_event(Event::Start(token))?;
    text_element(writer, "wsse:Username", credentials.username())?;
    let mut password = BytesStart::new("wsse:Password");
    password.push_attribute(("Type", PASSWORD_TEXT));
    writer.write_event(Event::Start(password))?;
    writer.write_event(Event::Text(BytesText::new(
        credentials.password.expose_secret(),
    )))?;
    writer.write_event(Event::End(BytesEnd::new("wsse:Password")))?;
    let mut nonce = BytesStart::new("wsse:Nonce");
    nonce.push_attribute(("EncodingType", BASE64_BINARY));
    writer.write_event(Event::Start(nonce))?;
    writer.write_event(Event::Text(BytesText::new(&security.nonce)))?;
    writer.write_event(Event::End(BytesEnd::new("wsse:Nonce")))?;
    text_element(writer, "wsu:Created", &security.created)?;
    writer.write_event(Event::End(BytesEnd::new("wsse:UsernameToken")))?;
    writer.write_event(Event::End(BytesEnd::new("wsse:Security")))
}

fn hex_string(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_security_values_are_unique() {
        let first = SecurityHeader::fresh().unwrap();
        let second = SecurityHeader::fresh().unwrap();
        assert_ne!(first.token_id, second.token_id);
        assert_ne!(first.nonce, second.nonce);
    }
}
