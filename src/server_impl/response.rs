use crate::server_impl::server::Header;
use bytes::Bytes;
use compact_str::CompactString;
use fnv::FnvHashMap;
use std::fmt::Write;
use std::str::FromStr;
use strum::{EnumMessage, EnumString, IntoStaticStr};

#[allow(clippy::upper_case_acronyms, non_camel_case_types)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, IntoStaticStr, EnumString, EnumMessage)]
pub enum StatusCode {
    #[strum(serialize = "200", message = "OK")]
    Ok,
    #[strum(serialize = "404", message = "Not Found.")]
    NotFound,
    #[strum(serialize = "422", message = "Stop with this shit.")]
    UnprocessableEntity,
}

#[derive(Debug)]
pub struct Response {
    pub headers: FnvHashMap<Header, CompactString>,
    pub status_code: StatusCode,
    pub body: Option<String>,
}

impl From<StatusCode> for Response {
    fn from(value: StatusCode) -> Self {
        Self {
            headers: Default::default(),
            status_code: value,
            body: None,
        }
    }
}

impl Response {
    pub fn into_http(self) -> Bytes {
        let mut buf = String::new();
        let status_code: &str = self.status_code.into();
        let status_message = self.status_code.get_message().unwrap();
        write!(
            buf,
            "HTTP/1.1 {status_code} {status_message}\n\
            Content-Type: application/json; charset-utf-8\
            Connection: keep-alive"
        )
        .expect("No reason to fail.");

        buf.into()
    }
}
