# zarrs_jpeg

> WARNING: The `jpeg` codec is a work in progress and likely to change; this codec should not be used to write data in production.

The [`jpeg`](https://github.com/zarr-developers/zarr-extensions/pull/66) codec for Zarr.

While originally written for use with [`zarrs`](https://zarrs.dev) (and still supporting it with the default `zarrs` feature), this can be wrapped for use with other zarr v3 implementations.

## Usage

The core provided types are `JpegCodecConfig`, which implements `serde::{Serialize, Deserialize}` for the representation used in the `"configuration"` object of the codec, i.e.

```jsonc
{
  "zarr_format": 3,
  "node_type": "array",
  // ...
  "codecs": [
    {
      "name": "jpeg",
      "configuration": {
        "decoded_color_space": "rgb",            // <----
        "encoded_color_space": "ycbcr",          // <---- this object
        "subsampling": [[2, 2], [1, 1], [1, 1]]  // <----
      }
    }
  ]
}
```

There is a type-level validity guarantee.

You can then use `TryInto` to convert this into a `JpegCodec`, which implements `JpegEncoderTrait` and `JpegDecoderTrait` to actually encode and decode JPEG data.

The `JpegCodec` itself implements `Serialize` and `Deserialize` (by going via `JpegCodecConfig`).

### Logging

This crate emits log messages using the `log` crate.

Applications should use a [`log`-compatible logging implementation](https://docs.rs/log/latest/log/#available-logging-implementations) or [`tracing` compatibility layer](https://docs.rs/tracing/0.1.44/tracing/#log-compatibility).

### I/O shape

JPEGs are themselves encoded in chunks (Minimum Coded Units), whose size depends on the chroma subsampling configuration (generally 8 or 16, but possibly 32).
The provided JpegCodec has an `mcu_shape` method; the first two dimensions of your chunk shape should be a multiple of this.

## Crate features

### `zarrs` (default)

Allows this crate to be used with the [`zarrs`](https://zarrs.dev) ecosystem.

### `backend-turbojpeg` (default)

Enables encoding and decoding using bindings to libjpeg-turbo, a C dependency.
The library is built and statically linked;
you will need CMake, a C compiler, and NASM (or possible YASM) to build with this feature.

This is the best-performing backend for both encoding and decoding.

### `backend-encode-mozjpeg-rs`

Enables encoding using the [mozjpeg-rs](https://crates.io/crates/mozjpeg-rs) crate,
which is a rust implementation of mozjpeg-like optimisations with no relationship to Mozilla (to my knowledge)
and makes heavy use of LLMs in the development and release process.

Does not yet support direct RGB->RGB encoding, [but will soon](https://github.com/imazen/mozjpeg-rs/pull/11).
Does not yet support accurate estimates of maximum encoded buffer size, [but may at some point](https://github.com/imazen/mozjpeg-rs/issues/12).

### `backend-decode-zune-jpeg`

Enables decoding using the [zune-jpeg](https://crates.io/crates/zune-jpeg) crate,
which is a rust implementation aiming for (and close to) libjpeg-turbo speeds.

### `backend-rust`

Alias for `backend-encode-mozjpeg-rs` and `backend-decode-zune-jpeg`.

## Examples

`cargo run --example astronaut` regenerates the data in `data/output/`.

This produces:

- `astronaut.jpeg`: a single JPEG, to prove that pixels can be written through the encoder
- `astronaut_raw.zarr`: a raw zarr array with XY chunks, to prove that writing a zarr array works
- `astronaut_jpeg.zarr`: a JPEG-compressed Zarr array with XY chunks - each chunk should be a valid RGB JFIF
- `astronaut_jpeg_channels.zarr`: a JPEG-compressed Zarr array with XY chunks and 1 chunk for each channel - each chunk should be a valid grayscale JFIF

This example uses the [template chunk key encoding extension](https://github.com/zarr-developers/zarr-extensions/tree/main/chunk-key-encodings/template) so that every chunk has the expected `.jpeg` suffix.
