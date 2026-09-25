# zarrs_jpeg

> WARNING: The `jpeg` codec is a work in progress and likely to change; this codec should not be used to write data in production.

The [`jpeg`](https://github.com/zarr-developers/zarr-extensions/pull/66) codec for [`zarrs`](https://zarrs.dev).

## Usage

### Logging

This crate emits log messages using the `log` crate.

Applications should use a [`log`-compatible logging implementation](https://docs.rs/log/latest/log/#available-logging-implementations) or [`tracing` compatibility layer](https://docs.rs/tracing/0.1.44/tracing/#log-compatibility).

### I/O shape

JPEGs are themselves encoded in chunks (Minimum Coded Units), whose size depends on the chroma subsampling configuration (generally 8 or 16, but possibly 32).
The provided JpegCodec has an `mcu_shape` method; the first two dimensions of your chunk shape should be a multiple of this.

## Crate features

### `zarrs`

Allows this crate to be used with the [`zarrs`](https://zarrs.dev) ecosystem.

## Dependencies

This crate uses bindings to libjpeg-turbo, a C dependency.
The library is built and statically linked;
you will need CMake, a C compiler, and NASM (or possibly YASM) to build this crate.

## Examples

`cargo run --example astronaut` regenerates the data in `data/output/`.

This produces:

- `astronaut_raw.zarr`: a raw zarr array with 4 XY chunks, to prove that writing a zarr array works
- `astronaut_jpeg.zarr`: a JPEG-compressed Zarr array with 4 XY chunks - each chunk should be a valid RGB JFIF
- `astronaut_jpeg_channels.zarr`: a JPEG-compressed Zarr array with 4 XY chunks x 1 chunk for each channel - each chunk should be a valid grayscale JFIF

This example uses the [template chunk key encoding extension](https://github.com/zarr-developers/zarr-extensions/tree/main/chunk-key-encodings/template) so that every chunk has the expected `.jpeg` extension.
