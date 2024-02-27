use crate::infrastructure::server_impl::server::{Header, Method};
use enum_map::EnumMap;

#[derive(Debug)]
pub struct Request<'a> {
    pub method: Method,
    pub headers: EnumMap<Header, &'a str>,
    pub resource: &'a str,
    pub body: Option<&'a str>,
}
