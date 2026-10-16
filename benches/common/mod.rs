//! Shared helpers for benchmarks.
#![allow(dead_code)]
use std::io::{BufReader, Cursor};

use zarrs_jpeg::{ColorConfig, JpegCodecConfig, JpegShape, Quality};

const PNG_PX: &[u8] = include_bytes!("../../data/input/astronaut.png");

/// Read the RGB astronaut image as interleaved pixels.
pub fn read_rgb() -> (JpegShape, Vec<u8>) {
    let decoder = png::Decoder::new(BufReader::new(Cursor::new(PNG_PX)));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgb);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    buf.truncate(info.buffer_size());
    (JpegShape::try_new(info.width, info.height).unwrap(), buf)
}

/// Read the red channel of the astronaut image as grayscale pixels.
pub fn read_gray() -> (JpegShape, Vec<u8>) {
    let (shape, px) = read_rgb();
    (shape, px.iter().step_by(3).copied().collect())
}

/// Named configurations to benchmark, with whether they take grayscale input.
pub fn configs() -> Vec<(&'static str, JpegCodecConfig, bool)> {
    let q = Quality::default();
    vec![
        (
            "grayscale",
            JpegCodecConfig::new(q, ColorConfig::Grayscale),
            true,
        ),
        ("rgb", JpegCodecConfig::new(q, ColorConfig::Rgb), false),
        (
            "rgb_to_ycbcr_11",
            JpegCodecConfig::new(q, ColorConfig::try_new_rgb_to_ycbcr(1, 1).unwrap()),
            false,
        ),
        (
            "rgb_to_ycbcr_22",
            JpegCodecConfig::new(q, ColorConfig::try_new_rgb_to_ycbcr(2, 2).unwrap()),
            false,
        ),
    ]
}
