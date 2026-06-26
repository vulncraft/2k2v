use std::ptr::read;

use anyhow::{Context, Result};
use crc32fast::Hasher;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub enum RecordVersion {
    Version1 = 1,
}
pub enum RecordType {
    DataOp = 1,
    NoopFence = 2,
}

#[derive(Debug, PartialEq, Eq)]
struct Record {
    pub len: u32,
    pub _type: u16,
    pub version: u16,
    pub seq: u64,
    pub data: Vec<u8>,
}

#[derive(Error, Debug)]
pub enum RecordError {
    #[error("CRC did not match")]
    HashMismatch,
}

impl Record {
    const VERSION_1: u16 = 1;
    pub fn init_v1_op(seq: u64, data: Vec<u8>) -> Self {
        let len = data.len() + 16;
        assert!(len < (u32::MAX as usize) + 18);
        Record {
            version: RecordVersion::Version1 as u16,
            len: len as u32,
            _type: RecordType::DataOp as u16,
            seq,
            data,
        }
    }

    pub async fn write_to<W>(&self, writer: &mut W) -> tokio::io::Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        // Len
        writer.write_all(&self.len.to_be_bytes()).await?;
        // Type
        writer.write_all(&self._type.to_be_bytes()).await?;
        // version
        writer.write_all(&self.version.to_be_bytes()).await?;
        // seq
        writer.write_all(&self.seq.to_be_bytes()).await?;
        // data
        writer.write_all(&self.data).await?;

        writer.write_all(&self.crc().to_be_bytes()).await?;
        Ok(())
    }

    fn crc(&self) -> u32 {
        let mut hasher = Hasher::new();
        // Len
        hasher.update(&self.len.to_be_bytes());
        // Type
        hasher.update(&self._type.to_be_bytes());
        // version
        hasher.update(&self.version.to_be_bytes());
        // seq
        hasher.update(&self.seq.to_be_bytes());

        // data
        hasher.update(&self.data);

        // crc
        hasher.finalize()
    }

    pub async fn read_from<R>(reader: &mut R) -> Result<Record>
    where
        R: AsyncRead + Unpin,
    {
        // Len
        let len = reader
            .read_u32()
            .await
            .context("len(u32) insufficient length")?;
        dbg!(len);

        let _type = reader
            .read_u16()
            .await
            .context("_type(u16) insufficient length")?;
        dbg!(_type);

        let version = reader
            .read_u16()
            .await
            .context("version(u16) insufficient length")?;
        dbg!(version);

        let seq = reader
            .read_u64()
            .await
            .context("seq(u64) insufficient length")?;

        let mut data = vec![0x41; (len - 16) as usize];
        reader
            .read_exact(&mut data)
            .await
            .context("data insufficient length")?;
        dbg!(&data);

        let actual_crc = reader
            .read_u32()
            .await
            .context("CRC(u32) insufficient length")?;

        let record = Record {
            len,
            _type,
            version,
            seq,
            data,
        };

        if actual_crc == record.crc() {
            Ok(record)
        } else {
            Err(RecordError::HashMismatch)?
        }
    }
}

#[cfg(test)]
mod tests {

    use std::io::Cursor;

    use super::*;

    #[tokio::test]
    async fn test_write_v1() {
        let data = vec![0x41; 20];
        let record = Record::init_v1_op(4, data.clone());
        let mut actual = Vec::with_capacity(2048);
        let _ = record.write_to(&mut actual).await;
        dbg!(actual.len());
        // assert_eq!(actual, data);
        let mut cursor = Cursor::new(actual);
        let read_record = Record::read_from(&mut cursor).await;
        assert_eq!(record, read_record.unwrap());
    }
    #[tokio::test]
    async fn test_write_v1_corrupted() {
        let data = vec![0x41; 20];
        let record = Record::init_v1_op(4, data.clone());
        let mut actual = Vec::with_capacity(2048);
        let _ = record.write_to(&mut actual).await;
        actual[4] = 0x42;

        let mut cursor = Cursor::new(actual);
        let read_record = Record::read_from(&mut cursor).await;
        assert!(read_record.is_err());
    }
    #[tokio::test]
    async fn test_write_v1_torn() {
        let data = vec![0x41; 20];
        let record = Record::init_v1_op(4, data.clone());
        let mut actual = Vec::with_capacity(2048);
        let _ = record.write_to(&mut actual).await;
        actual.truncate(actual.len() - 5);

        let mut cursor = Cursor::new(actual);
        let read_record = Record::read_from(&mut cursor).await;

        assert!(read_record.is_err());
    }
    #[tokio::test]
    async fn sequential_reads() {
        let data = vec![0x41; 20];
        let record = Record::init_v1_op(4, data.clone());
        let data2 = vec![0x43; 20];
        let record2 = Record::init_v1_op(5, data2.clone());
        let mut actual = Vec::with_capacity(2048);
        let _ = record.write_to(&mut actual).await;
        let _ = record2.write_to(&mut actual).await;

        let mut cursor = Cursor::new(actual);
        let read_record = Record::read_from(&mut cursor).await;
        assert_eq!(read_record.unwrap(), record);
        let read_record2 = Record::read_from(&mut cursor).await;
        assert_eq!(read_record2.unwrap(), record2);
    }
}
