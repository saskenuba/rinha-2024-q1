use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use httparse::{Header, ParserConfig, Request};
use rinha_de_backend::server_impl::server::parse_http;

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
            ParserConfig::default().parse_request(&mut req, black_box(SAMPLE));
            assert_eq!(req.path, Some("/somepath"));
        })
    });
}

criterion_group!(benches, bench_http_parsing);
criterion_main!(benches);
