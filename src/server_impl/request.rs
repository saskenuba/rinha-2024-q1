use crate::server_impl::server::{Header, Method};
use ahash::AHashMap;
use enum_map::EnumMap;

#[derive(Debug)]
pub struct Request<'a> {
    pub method: Method,
    pub headers: EnumMap<Header, &'a str>,
    pub resource: &'a str,
    pub body: Option<&'a str>,
}
