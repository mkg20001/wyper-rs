use std::sync::atomic::Ordering;

use anyhow::Result;

use crate::conf::AConf;
use crate::io::dev::Device;
use crate::progress::dev::PassResult;

pub struct PatternPass<'a> {
    pub pattern: Vec<u8>,
    pub device: &'a mut Device,
    pub pass_idx: usize,
    pub pos: u64,
    pub conf: AConf,
}

impl PatternPass<'_> {
    pub fn run(&mut self) -> Result<()> {
        assert_eq!(self.pattern.len(),self.conf.block_size);
        self.pos = 0;

        self.device.progress.current_pass.store(self.pass_idx,Ordering::Release);
        self.device.progress.current_bytes_written.store(0,Ordering::Release);
        self.device.progress.current_bad_sectors.store(0,Ordering::Release);

        let mut run = true;
        let mut error = None;

        while run {
            assert!(self.pos%(self.conf.block_size as u64) == 0);
            match self.device.write(&self.pattern, self.pos, self.conf.io_retries, self.conf.block_size as u64) {
                Ok(true) => self.pos += self.conf.block_size as u64,
                Ok(false) => {
                    self.pos += self.conf.block_size as u64;
                    run = false;
                }
                Err(e) => {
                    run = false;
                    error = Some(e);
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
