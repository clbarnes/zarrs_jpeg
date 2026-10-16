//! Compare decoding speed of the zune-jpeg and turbojpeg decoders.
use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use zarrs_jpeg::codec::{TurboCodec, ZuneDecoder};
use zarrs_jpeg::{JpegDecoderTrait, JpegEncoderTrait};

mod common;

fn bench_decode(c: &mut Criterion) {
    let rgb = common::read_rgb();
    let gray = common::read_gray();

    for (name, config, is_gray) in common::configs() {
        let (shape, px) = if is_gray { &gray } else { &rgb };
        let turbo = TurboCodec::try_from(config).unwrap();
        // Use the same encoded bytes for both decoders.
        let encoded = turbo.encode(px, *shape).unwrap();

        let mut group = c.benchmark_group(format!("decode/{name}"));
        // Throughput in terms of decoded bytes produced.
        group.throughput(Throughput::Bytes(px.len() as u64));

        group.bench_function("turbojpeg", |b| {
            b.iter(|| turbo.decode(black_box(&encoded)).unwrap())
        });

        let zune = ZuneDecoder::from(config);
        group.bench_function("zune-jpeg", |b| {
            b.iter(|| zune.decode(black_box(&encoded)).unwrap())
        });

        group.finish();
    }
}

criterion_group!(benches, bench_decode);
criterion_main!(benches);
