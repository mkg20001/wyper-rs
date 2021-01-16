use std::sync::atomic::Ordering;

use anyhow::Result;

use crate::conf::AConf;
use crate::io::dev::Device;
use crate::io::prng::PRNG;
use crate::progress::dev::PassResult;

use super::create_buf;

pub struct RandomPass<'a> {
    pub prng: PRNG,
    pub device: &'a mut Device,
    pub pass_idx: usize,
    pub pos: u64,
    pub conf: AConf,
}

impl RandomPass<'_> {
    pub fn run(&mut self) -> Result<()> {
        self.pos = 0;

        self.device.progress.current_pass.store(self.pass_idx,Ordering::Release);
        self.device.progress.current_bytes_written.store(0,Ordering::Release);
        self.device.progress.current_bad_sectors.store(0,Ordering::Release);

        let mut buf = create_buf(self.conf.block_size);
        let mut run = true;
        let mut error = None;

        while run {
            assert!(self.pos%(self.conf.block_size as u64) == 0);
            self.prng.read(&mut buf, self.pos).unwrap();
            match self.device.write(&buf, self.pos, self.conf.io_retries, self.conf.block_size as u64) {
                Ok(true) => self.pos += self.conf.block_size as u64,
                Ok(false) => {
                    self.pos += self.conf.block_size as u64;
                    run = false;
                }
                Err(e) => {
                    error = Some(e);
                    break;
                }
            }
            self.device.progress.current_bytes_written.store(self.pos,Ordering::Release);
        }

        if let Err(e) = self.device.flush(self.conf.io_retries) {
            if error.is_none() {
                error = Some(e);
            }
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
}
