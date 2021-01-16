use std::io::ErrorKind;
use std::sync::atomic::Ordering;
use std::os::unix::fs::FileExt;

use crate::conf::AConf;
use crate::io::dev::{Device, RWErrorType};
use crate::io::prng::PRNG;
use crate::progress::dev::PassResult;

use super::create_buf;

pub struct ReadRandomPass<'a> {
    pub prng: PRNG,
    pub device: &'a mut Device,
    pub pass_idx: usize,
    pub pos: u64,
    pub conf: AConf,
}

impl ReadRandomPass<'_> {
    pub fn run(&mut self) -> anyhow::Result<()> {
        self.pos = 0;

        self.device.progress.current_pass.store(self.pass_idx,Ordering::Release);
        self.device.progress.current_bytes_written.store(0,Ordering::Release);
        self.device.progress.current_bad_sectors.store(0,Ordering::Release);

        let mut buf = create_buf(self.conf.block_size);
        let mut buf2 = create_buf(self.conf.block_size);
        let mut run = true;
        let mut error = None;

        while run {
            assert!(self.pos%(self.conf.block_size as u64) == 0);

            self.prng.read(&mut buf, self.pos).unwrap();
            
            let ret_read;
            match self.read(&mut buf2,self.pos,self.conf.io_retries, self.conf.block_size as u64) {
                Ok((true,n)) => {
                    assert_eq!(n,buf.len() as u64);
                    ret_read = n as usize;
                },
                Ok((false,n)) => {
                    ret_read = n as usize;
                    run = false;
                },
                Err(e) => {
                    error = Some(e);
                    break;
                }
            }

            let comp1 = &buf[..ret_read];
            let comp2 = &buf2[..ret_read];

            for (a,b) in comp1.chunks(self.conf.block_size).zip( comp2.chunks(self.conf.block_size) ) {
                assert_eq!(a.len(),b.len());
                if a != b {
                    self.device.progress.current_bad_sectors.fetch_add(1,Ordering::SeqCst);
                }
            }

            self.device.progress.current_bytes_written.store(self.pos,Ordering::Release);
        }

        let result = PassResult{
            bytes_written: self.pos,
            bad_sectors: self.device.progress.current_bad_sectors.load(Ordering::Acquire),
            failure: error,
        };

        let mut lock = self.device.progress.pass_result.lock().unwrap();
        assert_eq!(lock.len(),self.pass_idx);
        lock.push(result);

        Ok(())
    }
    fn read(&mut self, mut dest: &mut [u8], mut pos: u64, attempts: usize, read_size_limit: u64) -> anyhow::Result<(bool,u64)> {
        // if dead sectors, keep zero'd area, and don't log (will be logged when compared to RNG)
        if attempts == 0 {
            for b in &mut dest[..] {
                *b = 0;
            }
            return Ok((true,dest.len() as u64));
        }

        let mut ret_read: u64 = 0;

        while dest.len() > 0 {
            assert!((pos+(dest.len() as u64))%(self.conf.sector_size as u64) == 0);
            // read size limited by dest end or read_size_limit
            let read_end = (pos + read_size_limit)/(self.conf.sector_size as u64)*(self.conf.sector_size as u64);
            let read_end = read_end.min(pos+(dest.len() as u64));
            match self.device.file.read_at(&mut dest[..(read_end-pos) as usize],pos) {
                Ok(n) => {
                    ret_read += n as u64;
                    pos += n as u64;
                    dest = &mut dest[n..];
                },
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {},
                Err(ref e) if e.kind() == ErrorKind::UnexpectedEof => {
                    // fill rest of dest with zeros
                    for b in &mut dest[..] {
                        *b = 0;
                    }
                    return Ok((false,ret_read));
                },
                Err(ref e) if RWErrorType::of(e) == RWErrorType::BadSectors => {
                    if dest.len() > self.conf.sector_size {
                        // try to read indiviual sectors for the rest of the block
                        match self.read(dest, pos, attempts, self.conf.sector_size as u64) {
                            Ok((s,n)) => return Ok((s,ret_read+n)),
                            Err(e) => return Err(e),
                        }
                    }else{
                        // retry for the sector we tried to read
                        match self.read(&mut dest[..(read_end-pos) as usize], pos, attempts-1, read_size_limit) {
                            Ok((true,n)) => {
                                // entire sector was read
                                assert_eq!(read_end-pos,n);
                                ret_read += n;
                                pos += n;
                                dest = &mut dest[n as usize..];
                            },
                            // EOF in sector read
                            Ok((false,n)) => return Ok((false,ret_read+n)),
                            Err(e) => return Err(e.into())
                        }
                    }
                },
                Err(e) => return Err(e.into()),
            }
        }
        Ok((true,ret_read))
    }
}
