use std::{fmt::Debug, num::NonZeroU16};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JpegShape {
    pub width: NonZeroU16,
    pub height: NonZeroU16,
}

impl JpegShape {
    pub fn try_new<N: TryInto<u16> + Debug + Copy>(width: N, height: N) -> crate::Result<Self> {
        let msg = || {
            crate::Error::general(format!(
                "Invalid JPEG shape: width={width:?}, height={height:?}"
            ))
        };

        let w16 = width.try_into().map_err(|_| msg())?;
        let width = NonZeroU16::new(w16).ok_or_else(msg)?;

        let h16 = height.try_into().map_err(|_| msg())?;
        let height = NonZeroU16::new(h16).ok_or_else(msg)?;
        Ok(Self { width, height })
    }
}
