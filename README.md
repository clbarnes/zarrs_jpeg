# zarrs_jpeg

The [`jpeg`](https://github.com/zarr-developers/zarr-extensions/pull/66) codec for [`zarrs`](https://zarrs.dev).

## Divergence from the spec

- Only accepts arrays which are 2D (as greyscale XY) or 3D (XYC, where 3 channels is interpreted as RGB and 1 channel is interpreted as greyscale)
  - If you have some other dimensionality, use a different chunking, or use the [sharding](https://docs.rs/zarrs/latest/zarrs/array/codec/array_to_bytes/sharding/index.html) or [reshape](https://docs.rs/zarrs/latest/zarrs/array/codec/array_to_array/reshape/index.html) codecs

## Limitations

The JPEG spec is very flexible and different encoders/ decoders vary greatly in which features they implement and/or default to.

Other than the `quality` setting, this crate uses

- the default encoding settings from the [`jpeg-encoder`](https://crates.io/crates/jpeg-encoder) crate, notably
  - assumes an RGB input where the shape suggests a multichannel image and converts to YCbCr
  - no chroma subsampling where `quality>90`, and then switches to 2x2/ 4:2:0
- the decoding features supported by the [`zune-jpeg`](https://crates.io/crates/zune-jpeg) crate

## Examples

`cargo run --example astronaut` regenerates the data in `output/astronaut*`.

This produces:

- `astronaut.jpeg`: a JPEG, to prove the encoder generally works
- `astronaut_raw.zarr`: a raw zarr array with 4 XY chunks, to prove that writing a zarr array works
- `astronaut_jpeg.zarr`: a JPEG-compressed Zarr array with 4 XY chunks - each chunk should be a valid RGB JFIF
- `astronaut_jpeg_channels.zarr`: a JPEG-compressed Zarr array with 4 XY chunks x 1 chunk for each channel - each chunk should be a valid greyscale JFIF
