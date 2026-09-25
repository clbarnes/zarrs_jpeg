use std::{fs::remove_dir_all, io::BufReader, path::PathBuf, sync::Arc};

use png::OutputInfo;
use template_cke::zarrs::TemplateChunkKeyEncoding;
use zarrs_jpeg::{
    JpegCodec, JpegEncoderTrait,
    config::{ColorConfig, JpegCodecConfig, Quality, SamplingRatios},
};

fn data_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("data");
    path
}

fn input_dir() -> PathBuf {
    let mut path = data_dir();
    path.push("input");
    path
}

fn output_dir() -> PathBuf {
    let mut path = data_dir();
    path.push("output");
    if !path.is_dir() {
        std::fs::create_dir_all(&path).unwrap();
    }
    path
}

fn read_astro() -> (OutputInfo, Vec<u8>) {
    let path = input_dir().join("astronaut.png");
    let f = std::fs::OpenOptions::new().read(true).open(&path).unwrap();
    let decoder = png::Decoder::new(BufReader::new(f));
    let mut reader = decoder.read_info().unwrap();

    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgb);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    let buf = &buf[..info.buffer_size()];
    (info, buf.to_vec())
}

fn write_astro_zarr(
    info: &OutputInfo,
    data: &[u8],
    name: &str,
    slice_channels: bool,
    codec: Option<JpegCodec>,
) {
    let path = output_dir().join(name);
    if path.is_dir() {
        remove_dir_all(&path).unwrap();
    }
    let store: zarrs::storage::ReadableWritableListableStorage =
        Arc::new(zarrs::filesystem::FilesystemStore::new(&path).unwrap());
    let c_chunking = if slice_channels { 1 } else { 3 };
    let mut builder = zarrs::array::ArrayBuilder::new(
        vec![info.height as u64, info.width as u64, 3],
        vec![info.height as u64, info.width as u64 / 2, c_chunking],
        zarrs::array::data_type::uint8(),
        0,
    );

    if let Some(c) = codec {
        // So that each chunk has the JPEG extension for easy viewing.
        builder
            .chunk_key_encoding(TemplateChunkKeyEncoding::try_new("{*}.jpeg", Some("/")).unwrap());
        builder.array_to_bytes_codec(Arc::new(c));
    }

    let array = builder.build(store.clone(), "/").unwrap();
    array
        .store_array_subset(&[0..info.height as u64, 0..info.width as u64, 0..3], data)
        .unwrap();
    array.store_metadata().unwrap();
}

fn write_astro_zarr_raw(info: &OutputInfo, data: &[u8]) {
    write_astro_zarr(info, data, "astronaut_raw.zarr", false, None);
}

fn write_astro_zarr_jpeg(info: &OutputInfo, data: &[u8]) {
    let config = JpegCodecConfig::new(
        Quality::default(),
        ColorConfig::RgbToYCbCr {
            subsampling: SamplingRatios::try_new(2, 2).unwrap(),
        },
    );
    write_astro_zarr(
        info,
        data,
        "astronaut_jpeg.zarr",
        false,
        Some(JpegCodec::try_from(config).unwrap()),
    );
}

fn write_astro_zarr_jpeg_channels(info: &OutputInfo, data: &[u8]) {
    let config = JpegCodecConfig::new(Quality::default(), ColorConfig::Grayscale);
    write_astro_zarr(
        info,
        data,
        "astronaut_jpeg_channels.zarr",
        true,
        Some(JpegCodec::try_from(config).unwrap()),
    );
}

fn write_astro_jpeg(info: &OutputInfo, data: &[u8]) {
    let config = JpegCodecConfig::new(
        Quality::default(),
        ColorConfig::RgbToYCbCr {
            subsampling: SamplingRatios::try_new(2, 2).unwrap(),
        },
    );
    let codec = JpegCodec::try_from(config).unwrap();
    let shape = zarrs_jpeg::JpegShape::try_new(info.width as u64, info.height as u64).unwrap();
    let jpeg_data = codec.encode(data, shape).unwrap();
    let path = output_dir().join("astronaut.jpeg");
    std::fs::write(path, jpeg_data).unwrap();
}

fn main() {
    let (info, pixels) = read_astro();
    write_astro_jpeg(&info, &pixels);
    write_astro_zarr_raw(&info, &pixels);
    write_astro_zarr_jpeg(&info, &pixels);
    write_astro_zarr_jpeg_channels(&info, &pixels);
}
