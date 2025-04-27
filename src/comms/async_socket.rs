use std::future::Future;

use super::{BINCODE_CONF, BincodeConfiguration};
use bincode::{
    Decode, Encode,
    error::{DecodeError, EncodeError},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// An error encountered when trying to read data from an asynchronous source with [`AsyncReadObj`].
#[derive(thiserror::Error, Debug)]
pub enum AsyncReadError {
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    /// A more fine variant of [`AsyncReadError::InvalidData`].
    #[error("The number of bytes read does not match the number of bytes requested.")]
    BufferMissMatch { expected: usize, read: usize },
    /// The data received cannot be decoded into the specified type.
    #[error("Data received is not valid.")]
    InvalidData(#[from] DecodeError),
}

/// Reads a data structure from a compatible asynchronous source.
pub trait AsyncReadObj {
    /// Reads the data structure from this source asynchronously.
    ///
    /// # Cancel Safety
    /// This method is **not** cancel safe.
    ///
    /// An arbitrary amount of data can be read and will be disguarded if this is cancelled.
    fn read_obj<Obj>(&mut self) -> impl Future<Output = Result<Obj, AsyncReadError>>
    where
        Obj: Decode<BincodeConfiguration>;
}

impl<From> AsyncReadObj for From
where
    From: AsyncReadExt + Unpin,
{
    fn read_obj<Obj>(&mut self) -> impl Future<Output = Result<Obj, AsyncReadError>>
    where
        Obj: Decode<BincodeConfiguration>,
    {
        async {
            let buf = &mut [0; (usize::BITS / 8) as usize];
            self.read_exact(buf).await?;
            let len = usize::from_ne_bytes(*buf);
            log::trace!("Async Read Len: {len}");

            let mut buf = vec![0u8; len].into_boxed_slice();

            let read_exact = self.read_exact(&mut buf).await?;
            if read_exact != len as usize {
                return Err(AsyncReadError::BufferMissMatch {
                    expected: len,
                    read: read_exact,
                });
            }

            log::trace!("Async Read Data: {:?}", buf);

            Ok(bincode::decode_from_slice_with_context(&buf, BINCODE_CONF, BINCODE_CONF)?.0)
        }
    }
}

/// An error encountered when trying to write data to an asynchronous output with [`AsyncWriteObj`].
#[derive(thiserror::Error, Debug)]
pub enum AsyncWriteError {
    /// Unable to encode the data for writing.
    #[error(transparent)]
    Encode(#[from] EncodeError),
    /// Unable to write data to the output.
    #[error(transparent)]
    Write(#[from] std::io::Error),
}

/// Writes a data structure to a compatible asynchronous output.
pub trait AsyncWriteObj {
    /// Writes the data structure to this output asynchronously.
    ///
    /// # Cancel Safety
    /// This method is **not** cancel safe.
    ///
    /// An arbitrary amount of data can might have been sent if this is cancelled.
    fn write_obj<Obj>(&mut self, data: Obj) -> impl Future<Output = Result<(), AsyncWriteError>>
    where
        Obj: Encode;
}

impl<To> AsyncWriteObj for To
where
    To: AsyncWriteExt + Unpin,
{
    fn write_obj<Obj>(&mut self, data: Obj) -> impl Future<Output = Result<(), AsyncWriteError>>
    where
        Obj: Encode,
    {
        async {
            let data = bincode::encode_to_vec(data, BINCODE_CONF)?;
            log::trace!("Async Write Len: {}", data.len());
            log::trace!("Async Write Data: {:?}", data);

            self.write_all(&data.len().to_ne_bytes()).await?;
            self.write_all(data.as_slice()).await?;

            self.flush().await?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::comms::sync_socket::{ReadObj, WriteObj};
    use tokio_util::bytes::{Buf, BufMut};

    #[derive(bincode::Decode, bincode::Encode, Debug, PartialEq)]
    enum TestData {
        VariantOne,
        Second,
    }

    #[tokio::test]
    async fn async_write() {
        let mut buf = [0u8; 12];

        buf.writer()
            .write_obj(TestData::Second)
            .expect("Can write to buf");

        assert_eq!(buf, [4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);

        buf.writer()
            .write_obj(TestData::VariantOne)
            .expect("Can write to buf");

        assert_eq!(buf, [4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[tokio::test]
    async fn async_read() {
        let buf: [u8; 12] = [4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        let data: TestData = buf.reader().read_obj().expect("Able to read from buf");
        assert_eq!(data, TestData::VariantOne);

        let buf: [u8; 12] = [4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];

        let data: TestData = buf.reader().read_obj().expect("Able to read from buf");
        assert_eq!(data, TestData::Second);
    }
}
