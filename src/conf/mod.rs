use std::sync::Arc;

pub struct Conf {
    /// block size for f, must be multiple of sector_size e.g. 16M
    pub block_size: usize,
    /// must be physical sector size of hdd e.g. 4K
    pub sector_size: usize,
    pub io_retries: usize,
}

impl Conf {
    pub fn default() -> Self {
        Self {
            block_size: 16<<20,
            sector_size: 0,
            io_retries: 4,
        }
    }
    pub fn validate(&self) {
        assert_ne!(self.block_size,0);
        assert_ne!(self.sector_size,0);
        assert_ne!(self.io_retries,0);
        assert!(self.block_size%self.sector_size == 0);
    }
}

pub type AConf = Arc<Conf>;

pub fn get_test_config() -> AConf {
    Arc::new(Conf{
        block_size: 16<<20,
        sector_size: 4096,
        io_retries: 4,
    })
}
