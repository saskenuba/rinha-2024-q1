use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use http::response::Builder;
use httparse::{ParserConfig, Request};
use rinha_de_backend::infrastructure::server_impl::response::{Response, StatusCode};
use rinha_de_backend::infrastructure::server_impl::server::parse_http;

const SAMPLE: &[u8] = b"GET /somepath HTTP/1.1\nHost: ifconfig.me\nUser-Agent: curl/8.5.0\nAccept: */*\nContent-Type: text/html; charset=ISO-8859-4\r\n\r\n{\"json_key\": 10}";

fn bench_http_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("http_parse");

    group.bench_function(BenchmarkId::new("My function", "sample http"), |c| {
        c.iter(|| parse_http(black_box(SAMPLE)))
    });
    group.bench_function(BenchmarkId::new("HTTP parse", "sample http"), |c| {
        c.iter(move || {
            let mut headers = [httparse::EMPTY_HEADER; 4];
            let mut req = Request::new(&mut headers);
            ParserConfig::default()
                .parse_request(black_box(&mut req), black_box(SAMPLE))
                .unwrap();
            assert_eq!(req.path, Some("/somepath"));
        })
    });
}

fn bench_http_response_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("response_build");

    group.bench_function(BenchmarkId::new("My response", "sample http"), |c| {
        c.iter(|| {
            let response = Response {
                headers: Default::default(),
                status_code: StatusCode::Ok,
                body: None,
            };
            Response::into_http(black_box(response));
        })
    });
    group.bench_function(
        BenchmarkId::new("HTTP crate response", "sample http"),
        |c| {
            c.iter(move || {
                let response: Builder =
                    http::Response::builder().status(http::StatusCode::from_u16(200).unwrap());
                Builder::body(black_box(response), black_box(())).unwrap();
            })
        },
    );
}

criterion_group!(http_parse, bench_http_parsing);
criterion_group!(http_response, bench_http_response_build);

criterion_main!(http_parse, http_response);
