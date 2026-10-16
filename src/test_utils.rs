//! Shared helpers for unit tests.
use std::{
    io::{BufReader, Cursor},
    sync::LazyLock,
};

use crate::codec::JpegCodecTraits;
use crate::*;

pub const PNG_PX: &[u8] = include_bytes!("../data/input/astronaut.png");

pub static RAW_IMG: LazyLock<(JpegShape, Vec<u8>)> = LazyLock::new(read_png_px);
pub static RAW_IMG_GRAYSCALES: LazyLock<(JpegShape, [Vec<u8>; 3])> = LazyLock::new(|| {
    let (shape, px) = RAW_IMG.clone();
    let (r, g, b) = px.as_chunks::<3>().0.iter().fold(
        (Vec::new(), Vec::new(), Vec::new()),
        |mut acc, chunk| {
            acc.0.push(chunk[0]);
            acc.1.push(chunk[1]);
            acc.2.push(chunk[2]);
            acc
        },
    );
    (shape, [r, g, b])
});

/// Read the RGB PNG image
fn read_png_px() -> (JpegShape, Vec<u8>) {
    let decoder = png::Decoder::new(BufReader::new(Cursor::new(PNG_PX)));
    let mut reader = decoder.read_info().unwrap();

    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgb);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    let buf = &buf[..info.buffer_size()];
    (
        JpegShape::try_new(info.width, info.height).unwrap(),
        buf.to_vec(),
    )
}

pub fn check_codec_roundtrip<C: HasJpegCodecConfig + JpegDecoderTrait + JpegEncoderTrait>(
    codec: C,
    epsilon: f32,
    px: &[u8],
    shape: JpegShape,
) {
    let encoded = codec.encode(px, shape).unwrap();
    let (decoded_shape, decoded_px) = codec.decode(&encoded).unwrap();

    assert_eq!(shape, decoded_shape);
    assert_eq!(px.len(), decoded_px.len());
    for (orig, roundtripped) in px.iter().zip(decoded_px.iter()) {
        let diff = (*orig as f32 - *roundtripped as f32).abs() / u8::MAX as f32;
        assert!(
            diff <= epsilon,
            "Pixel value differs by more than {}% of range: {} vs {}",
            epsilon * 100.0,
            orig,
            roundtripped
        );
    }
}

pub fn check_config_roundtrip<T: JpegCodecTraits>(config: JpegCodecConfig, epsilon: f32) {
    let codec = T::try_from(config).unwrap();
    let (shape, px) = RAW_IMG.clone();
    check_codec_roundtrip(codec, epsilon, &px, shape);
}

pub fn check_rgb<T: JpegCodecTraits>() {
    let config = JpegCodecConfig::new(Quality::max(), ColorConfig::Rgb);
    check_config_roundtrip::<T>(config, 0.1);
}

pub fn check_ycbcr<T: JpegCodecTraits>() {
    check_ycbcr_subsampled::<T>(1, 1, 0.1);
}

pub fn check_ycbcr_subsampled<T: JpegCodecTraits>(horizontal: u8, vertical: u8, epsilon: f32) {
    let config = JpegCodecConfig::new(
        Quality::max(),
        ColorConfig::try_new_rgb_to_ycbcr(horizontal, vertical).unwrap(),
    );
    check_config_roundtrip::<T>(config, epsilon);
}

pub fn check_ycbcr_subsampled_21<T: JpegCodecTraits>() {
    check_ycbcr_subsampled::<T>(2, 1, 0.2);
}

pub fn check_ycbcr_subsampled_22<T: JpegCodecTraits>() {
    check_ycbcr_subsampled::<T>(2, 2, 0.3);
}

pub fn check_ycbcr_subsampled_12<T: JpegCodecTraits>() {
    check_ycbcr_subsampled::<T>(1, 2, 0.3);
}

pub fn check_ycbcr_low_qual<T: JpegCodecTraits>() {
    let config = JpegCodecConfig::new(
        Quality::try_from(50).unwrap(),
        ColorConfig::RgbToYCbCr {
            subsampling: SamplingRatios::default(),
        },
    );
    check_config_roundtrip::<T>(config, 0.3);
}

pub fn check_grayscale<T: JpegCodecTraits>() {
    let config = JpegCodecConfig::new(Quality::max(), ColorConfig::Grayscale);
    let epsilon = 0.1;

    let codec = T::try_from(config).unwrap();
    let (shape, [r, _g, _b]) = RAW_IMG_GRAYSCALES.clone();
    check_codec_roundtrip(codec, epsilon, &r, shape);
}

/// Generate `#[test]` functions running every generic `check_*` roundtrip test
/// against the given codec type.
///
/// Invoke inside a `#[cfg(test)]` module, e.g. `impl_roundtrip_test!(TurboCodec);`.
///
/// Tests can be skipped (generated with `#[ignore]`, so they can still be run with `cargo test -- --ignored`):
/// `impl_roundtrip_test!(TurboCodec, skip = [grayscale, ycbcr_subsampled_12]);`
macro_rules! impl_roundtrip_test {
    ($codec:ty $(, skip = [$($skip:ident),* $(,)?])? $(,)?) => {
        // Pass `$` through so the inner macro can declare its own metavariables.
        $crate::test_utils::impl_roundtrip_test!(@dispatch ($) $codec; [$($($skip),*)?]);
    };
    (@dispatch ($d:tt) $codec:ty; [$($skip:ident),*]) => {
        macro_rules! __roundtrip_test {
            $(
                ($skip) => {
                    ::paste::paste! {
                        #[test]
                        #[ignore = "skipped for this codec"]
                        fn [<test_ $skip>]() {
                            $crate::test_utils::[<check_ $skip>]::<$codec>();
                        }
                    }
                };
            )*
            ($d name:ident) => {
                ::paste::paste! {
                    #[test]
                    fn [<test_ $d name>]() {
                        $crate::test_utils::[<check_ $d name>]::<$codec>();
                    }
                }
            };
        }

        __roundtrip_test!(rgb);
        __roundtrip_test!(ycbcr);
        __roundtrip_test!(ycbcr_subsampled_22);
        __roundtrip_test!(ycbcr_subsampled_21);
        __roundtrip_test!(ycbcr_subsampled_12);
        __roundtrip_test!(ycbcr_low_qual);
        __roundtrip_test!(grayscale);
    };
}
pub(crate) use impl_roundtrip_test;
