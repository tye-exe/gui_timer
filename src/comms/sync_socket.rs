use bincode::{
    Decode, Encode,
    error::{DecodeError, EncodeError},
};

use super::{BINCODE_CONF, BincodeConfiguration};

/// An error encountered when reading a data structure with [`ReadObj`].
#[derive(thiserror::Error, Debug)]
pub enum ReadError {
    /// Unable to read from the data source.
    #[error("Unable to read data.")]
    Read(#[from] std::io::Error),
    /// Unable to decode data read from the source.
    #[error("Unable to encode data.")]
    Decode(#[from] DecodeError),
}

/// For reading data structures from a comptable source.
pub trait ReadObj {
    /// Reads the data structure from this source.
    fn read_obj<Obj>(&mut self) -> Result<Obj, ReadError>
    where
        Obj: Decode<BincodeConfiguration>;
}

impl<From: std::io::Read> ReadObj for From {
    fn read_obj<Obj>(&mut self) -> Result<Obj, ReadError>
    where
        Obj: Decode<BincodeConfiguration>,
    {
        let buf = &mut [0; (usize::BITS / 8) as usize];
        self.read_exact(buf)?;
        log::trace!("Sync Read Len: {}", usize::from_ne_bytes(*buf));

        let mut buf = vec![0; usize::from_ne_bytes(*buf)].into_boxed_slice();
        self.read_exact(&mut buf)?;
        log::trace!("Sync Read Data: {:?}", buf);

        Ok(bincode::decode_from_slice_with_context(&buf, BINCODE_CONF, BINCODE_CONF)?.0)

        // Ok(bincode::decode_from_std_read_with_context(
        //     self,
        //     BINCODE_CONF,
        //     BINCODE_CONF,
        // )?)
    }
}

/// An error encounctered when writing a data structure with [`WriteObj`].
#[derive(thiserror::Error, Debug)]
pub enum WriteError {
    /// Unable to encoded data for writing.
    #[error("Unable to encode data.")]
    Encode(#[from] EncodeError),
    /// Unable to write data to the output.
    #[error("Unable to write data.")]
    Write(#[from] std::io::Error),
}

/// For writing data structure to a compatible output.
pub trait WriteObj {
    /// Writes the given data to this output.
    fn write_obj<Obj: Encode>(&mut self, data: Obj) -> Result<(), WriteError>;
}

impl<To: std::io::Write> WriteObj for To {
    fn write_obj<Obj: Encode>(&mut self, data: Obj) -> Result<(), WriteError> {
        let data = bincode::encode_to_vec(data, BINCODE_CONF)?;
        log::trace!("Sync Write Len: {}", data.len());
        log::trace!("Sync Write Data: {:?}", data);

        self.write_all(&data.len().to_ne_bytes())?;
        self.write_all(data.as_slice())?;

        self.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::comms::sync_socket::{ReadObj as _, WriteObj as _};

    #[derive(bincode::Decode, bincode::Encode, Debug, PartialEq)]
    enum TestData {
        VariantOne,
        Second,
    }

    #[tokio::test]
    async fn async_write() {
        let mut buf = vec![0u8; 12].into_boxed_slice();

        buf.as_mut()
            .write_obj(TestData::Second)
            .expect("Can write to buf");

        assert_eq!(
            buf,
            vec![4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1].into_boxed_slice()
        );

        buf.as_mut()
            .write_obj(TestData::VariantOne)
            .expect("Can write to buf");

        assert_eq!(
            buf,
            vec![4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0].into_boxed_slice()
        );
    }

    #[tokio::test]
    async fn async_read() {
        let buf: Box<[u8]> = vec![4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0].into_boxed_slice();

        let data: TestData = buf.as_ref().read_obj().expect("Able to read from buf");
        assert_eq!(data, TestData::VariantOne);

        let buf: Box<[u8]> = vec![4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1].into_boxed_slice();

        let data: TestData = buf.as_ref().read_obj().expect("Able to read from buf");
        assert_eq!(data, TestData::Second);
    }
}
