//! Compare encoding speed of the mozjpeg-rs and turbojpeg encoders.
use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use zarrs_jpeg::codec::{MozJpegRsEncoder, TurboCodec};
use zarrs_jpeg::{ColorSpace, JpegEncoderTrait};

mod common;

fn bench_encode(c: &mut Criterion) {
    let rgb = common::read_rgb();
    let gray = common::read_gray();

    for (name, config, is_gray) in common::configs() {
        let (shape, px) = if is_gray { &gray } else { &rgb };
        let mut group = c.benchmark_group(format!("encode/{name}"));
        group.throughput(Throughput::Bytes(px.len() as u64));

        let turbo = TurboCodec::try_from(config).unwrap();
        group.bench_function("turbojpeg", |b| {
            b.iter(|| turbo.encode(black_box(px), *shape).unwrap())
        });

        let encoded = config.color_config().color_spaces().1;
        if encoded != ColorSpace::Rgb {
            let moz = MozJpegRsEncoder::try_from(config).unwrap();
            group.bench_function("mozjpeg-rs", |b| {
                b.iter(|| moz.encode(black_box(px), *shape).unwrap())
            });
        }

        group.finish();
    }
}

criterion_group!(benches, bench_encode);
criterion_main!(benches);
