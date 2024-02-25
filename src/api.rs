use crate::server_impl::response::Response;
use crate::server_impl::server::{AnyResult, Method, Request};
use eyre::bail;

pub fn statement_route(req: Request) -> AnyResult<Response> {
    let body = req.body.unwrap();

    if req.method != Method::GET {
        bail!("Only GET available.")
    }

    // let a = simd_json::from_slice::<Statement>(body.as_str());
    todo!()
}
pub fn transaction_route(req: Request) -> AnyResult<Response> {
    if req.method != Method::POST {
        bail!("Only POST available.")
    }

    todo!()
}
