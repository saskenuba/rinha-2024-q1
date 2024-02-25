use crate::api::{statement_route, transaction_route};
use crate::server_impl::response::{Response, StatusCode};
use compact_str::CompactString;
use either::Either;
use eyre::{anyhow, bail, OptionExt};
use fnv::FnvHashMap;
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

fn parse_headers(headers: &[u8]) -> FnvHashMap<Header, &str> {
    headers
        .split(|c| *c == b'\n')
        .filter_map(|hdr_line| {
            let idx = memchr::memchr(b':', hdr_line)?;
            Some((&hdr_line[..idx], &hdr_line[idx + 1..]))
        })
        .flat_map(|(header, content)| {
            let header_str = to_str(header);

            let Ok(header) = header_str.parse::<Header>() else {
                return None;
            };

            let content = to_str(content).trim();
            Some((header, content))
        })
        .collect::<FnvHashMap<_, _>>()
}

fn parse_body(body: &[u8]) -> Option<&str> {
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
    // https://www.rfc-editor.org/rfc/rfc9110.html#name-protocol-version
    let Some(index) = memmem::find(request, b"HTTP/1.1") else {
        bail!("err");
    };

    let method_and_resource = &request[..index];
    let rest = &request[index..];

    let mut split = method_and_resource.split(|c| c.is_ascii_whitespace());
    let method = split
        .next()
        .map(Method::try_from)
        .ok_or_eyre("Malformed request.")?
        .map_err(|_| anyhow!("Unknown http method."))?;
    let resource = split.next().ok_or_eyre("Could not find resource.")?;

    let maybe_body = memmem::find(rest, b"\r\n\r\n").map(|idx| (&rest[..idx], &rest[idx + 4..]));

    let (headers, body) = if let Some((headers, body)) = maybe_body {
        (parse_headers(headers), parse_body(body))
    } else {
        (parse_headers(rest), None)
    };

    Ok(Request {
        method,
        resource: to_str(resource),
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
        assert_eq!(request.headers.get(&Header::HOST), Some(&"ifconfig.me"));
        assert_eq!(request.body, Some(r#"{"json_key": 10}"#));
    }

    #[test]
    fn success_without_body() {
        let sample = b"GET /somepath HTTP/1.1\nHost: ifconfig.me\nUser-Agent: curl/8.5.0\nAccept: */*\nContent-Type: text/html; charset=ISO-8859-4\n";

        let request = parse_http(sample).unwrap();
        assert_eq!(request.method, Method::GET);
        assert_eq!(request.resource, "/somepath");
        assert_eq!(request.headers.get(&Header::HOST), Some(&"ifconfig.me"));
        assert_eq!(request.body, None);
    }

    #[test]
    fn success_no_route() {
        let sample = b"GET /clientes/1/transacao HTTP/1.1\nHost: localhost\nUser-Agent: curl/8.5.0\nAccept: */*\nContent-Type: text/html; charset=ISO-8859-4\r\n\r\n{\"json_key\": 10}";

        let request = parse_http(sample).unwrap();
        assert_eq!(request.method, Method::GET);
        assert_eq!(request.resource, "/somepath");
        assert_eq!(request.headers.get(&Header::HOST), Some(&"ifconfig.me"));
        assert_eq!(request.body, Some(r#"{"json_key": 10}"#));
    }
}
