use crate::infrastructure::server_impl::server::Header;
use bytes::Bytes;
use compact_str::CompactString;
use derive_more::Deref;
use fnv::FnvHashMap;
use serde::Serialize;
use std::fmt::Write;
use strum::{EnumMessage, EnumString, IntoStaticStr};
use time::OffsetDateTime;

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

impl Response {
    pub fn from_status_code(value: StatusCode, body: impl Into<Option<String>>) -> Self {
        Self {
            headers: Default::default(),
            status_code: value,
            body: body.into(),
        }
    }
}

impl Response {
    pub fn into_http(self) -> Bytes {
        // FIXME: use BufWriter to avoid calling write multiple times
        let mut buf = String::with_capacity(80);
        let status_code: &str = self.status_code.into();
        let status_message = self.status_code.get_message().unwrap();

        write!(
            buf,
            "HTTP/1.1 {status_code} {status_message}\r\n\
             Date: Sun, 06 Nov 1994 08:49:37\r\n\
             Server: localhost\r\n"
        )
        .expect("No reason to fail.");

        if let Some(body) = self.body {
            let length = body.len();
            write!(
                buf,
                "Connection: keep-alive\r\n\
                 Content-Type: application/json; charset-utf-8\r\n\
                 Content-Length: {length}\r\n\r\n{body}"
            )
            .unwrap();
        } else {
            write!(buf, "Content-Length: 0\r\n\r\n").unwrap()
        };

        buf.into()
    }
}

#[derive(Debug, Deref)]
pub struct JsonResponse(pub Response);

impl JsonResponse {
    pub fn from<T>(body: T) -> Self
    where
        T: Serialize,
    {
        let mut response = Response::from_status_code(StatusCode::Ok, None);
        let body = simd_json::to_string(&body).ok();
        response.body = body;
        Self(response)
    }
}
