use std::fs::File;
use std::io::Read;

use anyhow::{Result, ensure};
use rand_hc::Hc128Rng;
use rand_core::{RngCore, SeedableRng};

use crate::conf::AConf;

use super::pass::create_buf;

type Seed = <Hc128Rng as SeedableRng>::Seed;

pub fn get_random_seed() -> anyhow::Result<Seed> {
    let mut dest = [0;32];
    let mut dev_random = File::open("/dev/random")?;
    dev_random.read_exact(&mut dest)?;
    Ok(dest)
}

pub struct PRNG {
    rng: Hc128Rng,
    pub seed: Seed,
    next_pos: u64,
    conf: AConf,
    buf: Vec<u8>,
}

impl PRNG {
    /// use [`get_random_seed`] to generated seed
    pub fn from_seed(conf: AConf, seed: Seed) -> Self {
        Self{
            rng: Hc128Rng::from_seed(seed.clone()),
            seed,
            next_pos: 0,
            buf: create_buf(conf.block_size),
            conf,
        }
    }
    pub fn new_with_same_seed(&self) -> Self {
        Self::from_seed(self.conf.clone(), self.seed.clone())
    }
    pub fn read(&mut self, dest: &mut [u8], absolute_pos: u64) -> Result<()> {
        assert!(self.next_pos%(self.conf.block_size as u64) == 0);
        ensure!(dest.len() == self.buf.len(),"Invalid Block Size");
        ensure!(absolute_pos+(self.conf.block_size as u64) >= self.next_pos,"block exceeded");
        while self.next_pos < absolute_pos+(self.conf.block_size as u64) {
            self.rng.fill_bytes(&mut self.buf);
            self.next_pos += self.conf.block_size as u64;
        }
        assert_eq!(self.next_pos,absolute_pos+(self.conf.block_size as u64));
        dest.copy_from_slice(&self.buf);
        Ok(())
    }
}

#[allow(unused_imports)]
mod test {
    use crate::conf::get_test_config;
    use crate::io::pass::create_buf;

    use super::{PRNG, get_random_seed};

    #[test]
    fn test_prng() {
        let conf = get_test_config();
        let seed = get_random_seed().unwrap();
        let mut prng = PRNG::from_seed(conf.clone(),seed);
        let mut dest = create_buf(conf.block_size);
        assert!( prng.read(&mut dest, (conf.block_size as u64)*4 ) .is_ok() );
        assert!( prng.read(&mut dest, (conf.block_size as u64)*8 ) .is_ok() );
        assert!( prng.read(&mut dest, (conf.block_size as u64)*2 ) .is_err() );
        assert!( prng.read(&mut dest, (conf.block_size as u64)*8 ) .is_ok() );
        assert!( prng.read(&mut dest, (conf.block_size as u64)*10 ) .is_ok() );
        assert!( prng.read(&mut dest, (conf.block_size as u64)*10 ) .is_ok() );
        assert!( prng.read(&mut dest, (conf.block_size as u64)*10 ) .is_ok() );
        assert!( prng.read(&mut dest, (conf.block_size as u64)*9 ) .is_err() );
        assert!( prng.read(&mut dest, (conf.block_size as u64)*4 ) .is_err() );
        assert!( prng.read(&mut dest, (conf.block_size as u64)*8 ) .is_err() );
        assert!( prng.read(&mut dest, (conf.block_size as u64)*10 ) .is_ok() );
    }
}
