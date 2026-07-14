# zarrs_jpeg

> WARNING: The `jpeg` codec is a work in progress and likely to change; this codec should not be used to write data in production.

The [`jpeg`](https://github.com/zarr-developers/zarr-extensions/pull/66) codec for [`zarrs`](https://zarrs.dev).

## Dependencies

This crate uses bindings to libjpeg-turbo, a C dependency.
These are built and statically linked;
you will need CMake, a C compiler, and NASM (or possibly YASM) to build this crate.

## Examples

`cargo run --example astronaut` regenerates the data in `data/output/`.

This produces:

- `astronaut.jpeg`: a JPEG, to prove the encoder generally works
- `astronaut_raw.zarr`: a raw zarr array with 4 XY chunks, to prove that writing a zarr array works
- `astronaut_jpeg.zarr`: a JPEG-compressed Zarr array with 4 XY chunks - each chunk should be a valid RGB JFIF
- `astronaut_jpeg_channels.zarr`: a JPEG-compressed Zarr array with 4 XY chunks x 1 chunk for each channel - each chunk should be a valid greyscale JFIF
