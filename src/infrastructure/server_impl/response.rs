use crate::infrastructure::server_impl::server::Header;
use bytes::Bytes;
use compact_str::CompactString;
use fnv::FnvHashMap;
use serde::Serialize;
use std::fmt::Write;
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
        // FIXME: use BufWriter to avoid calling write multiple times
        let mut buf = String::with_capacity(80);
        let status_code: &str = self.status_code.into();
        let status_message = self.status_code.get_message().unwrap();

        write!(
            buf,
            "HTTP/1.1 {status_code} {status_message}\r\n\
            Content-Type: application/json; charset-utf-8\r\n\
            Connection: keep-alive\r\n"
        )
        .expect("No reason to fail.");

        if let Some(body) = self.body {
            let length = body.len();
            write!(buf, "Content-Length: {length}\r\n\r\n{body}").unwrap();
        }

        buf.into()
    }
}

#[derive(Debug)]
pub struct JsonResponse(pub(crate) Response);

impl JsonResponse {
    pub fn from<T>(body: T) -> Self
    where
        T: Serialize,
    {
        let mut response = Response::from(StatusCode::Ok);
        let body = simd_json::to_string(&body).ok();
        println!("body: {:?}", body);
        response.body = body;
        Self(response)
    }
}
