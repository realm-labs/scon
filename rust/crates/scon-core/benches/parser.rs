use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use serde::Deserialize;

#[derive(Deserialize)]
struct AppConfig {
    name: String,
    server: ServerConfig,
    paths: Vec<String>,
    services: Vec<ServiceConfig>,
}

#[derive(Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
    tls: bool,
    url: String,
}

#[derive(Deserialize)]
struct ServiceConfig {
    name: String,
    port: u16,
    enabled: bool,
}

impl AppConfig {
    fn checksum(&self) -> usize {
        self.name.len()
            + self.server.host.len()
            + self.server.port as usize
            + usize::from(self.server.tls)
            + self.server.url.len()
            + self.paths.iter().map(String::len).sum::<usize>()
            + self
                .services
                .iter()
                .map(|service| {
                    service.name.len() + service.port as usize + usize::from(service.enabled)
                })
                .sum::<usize>()
    }
}

fn config_source() -> String {
    let mut source = String::from(
        r#"
defaults {
  host = "127.0.0.1"
  port = 8080
  tls = true
}

name = "demo"
server {
  ...${defaults}
  port = 9090
  url = "http://${server.host}:${server.port}"
}

base_paths = ["/bin", "/usr/bin", "/usr/local/bin"]
paths = [...${base_paths}, "/opt/app/bin"]
services = [
"#,
    );

    for i in 0..100 {
        source.push_str(&format!(
            r#"  {{
    name = "service-{i}"
    port = {}
    enabled = true
  }},
"#,
            10_000 + i
        ));
    }

    source.push_str("]\n");
    source
}

fn simple_config_source() -> String {
    let mut source = String::from(
        r#"
name = "demo"
server {
  host = "127.0.0.1"
  port = 9090
  tls = true
  url = "http://127.0.0.1:9090"
}
paths = ["/bin", "/usr/bin", "/usr/local/bin", "/opt/app/bin"]
services = [
"#,
    );

    for i in 0..100 {
        source.push_str(&format!(
            r#"  {{
    name = "service-{i}"
    port = {}
    enabled = true
  }},
"#,
            10_000 + i
        ));
    }

    source.push_str("]\n");
    source
}

fn bench_parse(c: &mut Criterion) {
    let source = config_source();
    c.bench_function("parse_str medium config", |b| {
        b.iter(|| scon::parse_str(black_box(&source)).unwrap())
    });

    let simple_source = simple_config_source();
    c.bench_function("parse_str simple medium config", |b| {
        b.iter(|| scon::parse_str(black_box(&simple_source)).unwrap())
    });
}

fn bench_deserialize(c: &mut Criterion) {
    let source = config_source();
    c.bench_function("from_str medium config", |b| {
        b.iter(|| {
            let config: AppConfig = scon::from_str(black_box(&source)).unwrap();
            black_box(config.checksum());
        })
    });

    let simple_source = simple_config_source();
    c.bench_function("from_str simple medium config", |b| {
        b.iter(|| {
            let config: AppConfig = scon::from_str(black_box(&simple_source)).unwrap();
            black_box(config.checksum());
        })
    });
}

criterion_group!(benches, bench_parse, bench_deserialize);
criterion_main!(benches);
