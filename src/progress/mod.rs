use crate::io::pass::PassType;

use self::dev::DevProgress;

pub mod dev;

pub struct DeviceInfo {
    passes: Vec<PassType>,
    stats: DevProgress,
}
