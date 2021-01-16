use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::sync::Arc;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::fs::FileExt;
use std::sync::atomic::Ordering;

use libc::O_DIRECT;

use crate::conf::AConf;
use crate::progress::dev::DevProgress;

pub struct Device {
    pub id: usize,
    pub dev_path: Arc<Path>,
    pub file: File,
    pub progress: Arc<DevProgress>,
    pub conf: AConf,
}

impl Device {
    pub fn open_rw(id: usize,dev_path: Arc<Path>,conf: AConf) -> anyhow::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(false)
            .create_new(false)
            .truncate(false)
            .custom_flags(O_DIRECT)
            .open(&dev_path)?;
        Ok(Self{
            id,
            dev_path,
            file,
            progress: Arc::new(DevProgress::default()),
            conf,
        })
    }
    pub fn write(&mut self, mut data: &[u8], mut pos: u64, attempts: usize, write_size_limit: u64) -> anyhow::Result<bool> {
        self.conf.validate();
        if attempts == 0 {
            self.progress.current_bad_sectors.fetch_add(1,Ordering::SeqCst);
            return Ok(true);
        }

        while data.len() > 0 {
            assert!((pos+(data.len() as u64))%(self.conf.sector_size as u64) == 0);
            // read size limited by dest end or read_size_limit
            let write_end = (pos + write_size_limit)/(self.conf.sector_size as u64)*(self.conf.sector_size as u64);
            let write_end = write_end.min(pos+(data.len() as u64));
            match self.file.write_at(&data[..(write_end-pos) as usize],pos) {
                Ok(n) => {
                    pos += n as u64;
                    data = &data[n..];
                },
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {},
                Err(ref e) if e.kind() == ErrorKind::WriteZero => return Ok(false),
                Err(ref e) if e.kind() == ErrorKind::UnexpectedEof => return Ok(false),
                Err(ref e) if RWErrorType::of(e) == RWErrorType::BadSectors => {
                    if data.len() > self.conf.sector_size {
                        // write in individual seectors for the rest of the block
                        return self.write(data, pos, attempts, self.conf.sector_size as u64);
                    }else{
                        // retry for the sector we tried to write
                        match self.write(&data[..(write_end-pos) as usize], pos, attempts-1, write_size_limit) {
                            // entire sector was written
                            Ok(true) => {
                                pos = write_end;
                                data = &data[(write_end-pos) as usize..];
                            },
                            // EOF in sector
                            Ok(false) => return Ok(false),
                            Err(e) => return Err(e.into())
                        }
                    }
                },
                Err(e) => return Err(e.into()),
            }
        }
        Ok(true)
    }
    pub fn flush(&mut self, attempts: usize) -> anyhow::Result<()> {
        if attempts == 0 {
            self.progress.current_bad_sectors.fetch_add(1,Ordering::SeqCst);
            return Ok(());
        }
        match self.file.flush() {
            Ok(()) => {},
            Err(ref e) if RWErrorType::of(e) == RWErrorType::BadSectors => 
                return self.flush(attempts-1),
            Err(e) => return Err(e.into()),
        }
        Ok(())
    }
}

pub enum DWResult {
    Resume,
    Done,
    Fatal,
}

#[derive(PartialEq)]
pub enum RWErrorType {
    BadSectors,
    Fatal,
}

impl RWErrorType {
    pub fn of(e: &std::io::Error) -> Self {
        let code = match e.raw_os_error() {
            Some(c) => c,
            None => return Self::Fatal,
        };

        match code {
            libc::ETIMEDOUT => Self::BadSectors,
            libc::EIO => Self::BadSectors,

            _ => Self::Fatal,
        }
    }
}
