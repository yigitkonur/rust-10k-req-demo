//! Throughput benchmarks for Blaze API.

use criterion::{criterion_group, criterion_main, Criterion, Throughput};

fn benchmark_request_parsing(c: &mut Criterion) {
    let sample_json = r#"{"input": "What is the capital of France?"}"#;

    let mut group = c.benchmark_group("parsing");
    group.throughput(Throughput::Elements(1));

    group.bench_function("parse_request", |b| {
        b.iter(|| {
            let _: blaze_api::ApiRequest = serde_json::from_str(sample_json).unwrap();
        });
    });

    group.finish();
}

fn benchmark_load_balancer(c: &mut Criterion) {
    use blaze_api::{EndpointConfig, LoadBalancer};

    let configs = vec![
        EndpointConfig {
            url: "http://a.test".to_string(),
            weight: 1,
            api_key: None,
            model: None,
            max_concurrent: 100,
        },
        EndpointConfig {
            url: "http://b.test".to_string(),
            weight: 2,
            api_key: None,
            model: None,
            max_concurrent: 100,
        },
        EndpointConfig {
            url: "http://c.test".to_string(),
            weight: 3,
            api_key: None,
            model: None,
            max_concurrent: 100,
        },
    ];

    let lb = LoadBalancer::new(configs).unwrap();

    let mut group = c.benchmark_group("load_balancer");
    group.throughput(Throughput::Elements(1));

    group.bench_function("select_endpoint", |b| {
        b.iter(|| {
            let _ = lb.select();
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_request_parsing, benchmark_load_balancer);
criterion_main!(benches);
