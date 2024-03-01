use std::str::FromStr;
use std::sync::OnceLock;

use crate::AnyResult;
use either::Either;
use enum_map::{Enum, EnumMap};
use httparse::{ParserConfig, Status};
use memchr::memchr;
use regex_lite::Regex;
use strum::{EnumIter, EnumString, IntoEnumIterator, IntoStaticStr};

use crate::api::{statement_route, transaction_route};
use crate::application::ServerData;
use crate::infrastructure::server_impl::request::Request;
use crate::infrastructure::server_impl::response::{Response, StatusCode};

static ROUTER: OnceLock<Regex> = OnceLock::new();

pub fn get_router() -> &'static Regex {
    ROUTER.get_or_init(|| Regex::new(r#"clientes/(\d)/(transacoes|extrato)"#).unwrap())
}

pub async fn match_routes(
    server_data: &ServerData,
    request: Request<'_>,
) -> Either<Response, Response> {
    let Some(route) = get_router().captures(request.resource) else {
        return Either::Right(Response::from_status_code(
            StatusCode::NotFound,
            "route error".to_string(),
        ));
    };
    let client_id = route
        .get(1)
        .and_then(|c| i32::from_str(c.as_str()).ok())
        .unwrap();

    // the fastest router in existence!
    let response = match route.get(2).map(|c| c.as_str()).unwrap() {
        "extrato" => statement_route(server_data, request, client_id).await,
        "transacoes" => transaction_route(server_data, request, client_id).await,
        _ => {
            return Either::Right(Response::from_status_code(
                StatusCode::NotFound,
                "route not found".to_string(),
            ))
        }
    };

    Either::Left(response.unwrap())
}

#[allow(clippy::upper_case_acronyms, non_camel_case_types)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Enum, IntoStaticStr, EnumIter)]
#[non_exhaustive]
pub enum Header {
    #[strum(serialize = "accept")]
    ACCEPT,
    #[strum(serialize = "accept-encoding")]
    ACCEPT_ENCODING,
    #[strum(serialize = "content-length")]
    CONTENT_LENGTH,
    #[strum(serialize = "content-type")]
    CONTENT_TYPE,
    #[strum(serialize = "connection")]
    CONNECTION,
    #[strum(serialize = "host")]
    HOST,
    #[strum(serialize = "user-agent")]
    USER_AGENT,
}

impl FromStr for Header {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::iter().find(|c| unicase::eq(c.into(), s)).ok_or(())
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
        let a = to_str(value);
        Method::from_str(a).map_err(|_| ())
    }
}

fn to_str(str_like: &[u8]) -> &str {
    unsafe { std::str::from_utf8_unchecked(str_like) }
}

fn parse_body(body: &[u8]) -> Option<&[u8]> {
    if body.is_empty() || body.first() == Some(&b'\0') {
        return None;
    }

    let body_content = memchr(b'\0', body).map(|idx| &body[..idx]).unwrap_or(body);
    Some(body_content)
}

/// Returns a [Request]
/// Won't handle anything else than a simple request.
/// And probably explode if anything else than a well-formed request is parsed.
pub fn parse_http(request: &[u8]) -> AnyResult<Request> {
    let mut headers = [httparse::EMPTY_HEADER; 7];
    let mut req = httparse::Request::new(&mut headers);
    let body = ParserConfig::default()
        .parse_request(&mut req, request)
        .unwrap();

    let method = Method::from_str(req.method.unwrap()).unwrap();
    let resource = req.path.unwrap();

    let headers = req
        .headers
        .iter()
        .filter_map(|c| Header::from_str(c.name).ok().map(|h| (h, to_str(c.value))))
        .collect::<EnumMap<_, _>>();

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
            request.headers[Header::CONTENT_TYPE],
            "text/html; charset=ISO-8859-4"
        );
        assert_eq!(to_str(request.body.unwrap()), r#"{"json_key": 10}"#);
    }

    #[test]
    fn success_without_body() {
        let sample = b"GET /somepath HTTP/1.1\nHost: ifconfig.me\nUser-Agent: curl/8.5.0\nAccept: */*\nContent-Type: text/html; charset=ISO-8859-4\r\n\r\n";

        let request = parse_http(sample).unwrap();
        assert_eq!(request.method, Method::GET);
        assert_eq!(request.resource, "/somepath");
        assert_eq!(request.headers[Header::HOST], "ifconfig.me");
        assert_eq!(request.body, None);
    }
}
