use crate::api::{statement_route, transaction_route};
use crate::server_impl::response::{Response, StatusCode};
use compact_str::CompactString;
use either::Either;
use eyre::{anyhow, bail, OptionExt};
use fnv::FnvHashMap;
use httparse::{ParserConfig, Status};
use memchr::{memchr, memmem};
use regex_lite::{Match, Regex};
use std::str::FromStr;
use std::sync::OnceLock;
use strum::{EnumString, IntoStaticStr};

pub type AnyResult<T> = eyre::Result<T>;

static ROUTER: OnceLock<Regex> = OnceLock::new();

pub fn get_router() -> &'static Regex {
    ROUTER.get_or_init(|| Regex::new(r#"clientes/(\d)/(transacoes|extrato)"#).unwrap())
}

pub fn process_server_request(buffer: &mut [u8], read_bytes: usize) -> AnyResult<Request> {
    let request = parse_http(buffer)?;

    // let a = match_routes(request.resource.as_str(), request);

    todo!()
}

pub fn match_routes(resource: &str, request: Request) -> Either<Response, Response> {
    let Some(route) = get_router().captures(resource) else {
        return Either::Right(StatusCode::NotFound.into());
    };

    // the fastest router in existence!
    let response = match route.get(2).map(|c| c.as_str()).unwrap() {
        "transacao" => statement_route(request),
        "extrato" => transaction_route(request),
        _ => unreachable!(),
    };

    todo!()
}

#[allow(clippy::upper_case_acronyms, non_camel_case_types)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Header {
    HOST,
    USER_AGENT,
    ACCEPT,
    CONTENT_TYPE,
    CONTENT_LENGTH,
}

impl FromStr for Header {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let res = match s {
            "Host" => Self::HOST,
            "User-Agent" => Self::USER_AGENT,
            "Accept" => Self::ACCEPT,
            "Content-Type" => Self::CONTENT_TYPE,
            "Content-Length" => Self::CONTENT_LENGTH,
            _ => return Err(()),
        };

        Ok(res)
    }
}
#[allow(clippy::upper_case_acronyms, non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, EnumString, IntoStaticStr)]
pub enum Method {
    CONNECT,
    DELETE,
    GET,
    HEAD,
    POST,
    PUT,
}

impl TryFrom<&[u8]> for Method {
    type Error = ();

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let a = unsafe { std::str::from_utf8_unchecked(value) };
        Method::from_str(a).map_err(|_| ())
    }
}

// impl FromStr for Method {
//     type Err = ();
//
//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         let res = match s {
//             "CONNECT" => Self::CONNECT,
//             "DELETE" => Self::DELETE,
//             "GET" => Self::GET,
//             "HEAD" => Self::HEAD,
//             "POST" => Self::POST,
//             "PUT" => Self::PUT,
//             _ => return Err(()),
//         };
//
//         Ok(res)
//     }
// }

// this module won't respect much of the HTTP specification
// it is entirely tailored for the rinha-de-backend tests and don't represent real life.
// https://www.rfc-editor.org/rfc/rfc9110.html#name-requirements-notation

#[derive(Debug)]
pub struct Request<'a> {
    pub method: Method,
    pub headers: FnvHashMap<Header, &'a str>,
    pub resource: &'a str,
    pub body: Option<&'a str>,
}

fn to_str(str_like: &[u8]) -> &str {
    unsafe { std::str::from_utf8_unchecked(str_like) }
}

fn parse_body(body: &[u8]) -> Option<&str> {
    if body.is_empty() {
        return None;
    }

    if body.first() == Some(&b'\0') {
        None
    } else {
        let body_content = memchr(b'\0', body).map(|idx| &body[..idx]).unwrap_or(body);
        // let (body_content, _) = body.split_once(|b| *b == b'\0').unwrap_or((body, &[]));
        Some(to_str(body_content))
    }
}

/// Returns a [Request]
/// Won't handle anything else than a simple request.
/// And probably explode if anything else than a well-formed request is parsed.
#[inline]
pub fn parse_http(request: &[u8]) -> AnyResult<Request> {
    let mut headers = [httparse::EMPTY_HEADER; 4];
    let mut req = httparse::Request::new(&mut headers);
    let body = ParserConfig::default()
        .parse_request(&mut req, request)
        .unwrap();

    let method = Method::from_str(req.method.unwrap()).unwrap();
    let resource = req.path.unwrap();
    let headers = req
        .headers
        .iter()
        .map(|c| (Header::from_str(c.name).unwrap(), to_str(c.value)))
        .collect::<FnvHashMap<_, _>>();

    let body = match body {
        Status::Complete(idx) => parse_body(&request[idx..]),
        Status::Partial => unimplemented!(),
    };

    Ok(Request {
        method,
        resource,
        headers,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_with_body() {
        let sample = b"GET /somepath HTTP/1.1\nHost: ifconfig.me\nUser-Agent: curl/8.5.0\nAccept: */*\nContent-Type: text/html; charset=ISO-8859-4\r\n\r\n{\"json_key\": 10}";

        let request = parse_http(sample).unwrap();
        assert_eq!(request.method, Method::GET);
        assert_eq!(request.resource, "/somepath");
        assert_eq!(
            request.headers.get(&Header::CONTENT_TYPE),
            Some(&"text/html; charset=ISO-8859-4")
        );
        assert_eq!(request.body, Some(r#"{"json_key": 10}"#));
    }

    #[test]
    fn success_without_body() {
        let sample = b"GET /somepath HTTP/1.1\nHost: ifconfig.me\nUser-Agent: curl/8.5.0\nAccept: */*\nContent-Type: text/html; charset=ISO-8859-4\r\n\r\n";

        let request = parse_http(sample).unwrap();
        assert_eq!(request.method, Method::GET);
        assert_eq!(request.resource, "/somepath");
        assert_eq!(request.headers.get(&Header::HOST), Some(&"ifconfig.me"));
        assert_eq!(request.body, None);
    }
}
