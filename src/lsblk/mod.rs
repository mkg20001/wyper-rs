use std::process::Command;

use anyhow::ensure;
use serde_derive::*;

#[derive(Deserialize,Debug)]
pub struct Lsblk {
    pub blockdevices: Vec<LsblkBlockDevice>,
}

#[derive(Deserialize,Debug)]
pub struct LsblkBlockDevice {
    pub name: String,
    pub rm: bool,
    pub size: String,
    pub ro: bool,
    pub r#type: String,
    pub mountpoint: Option<String>,
    pub hotplug: bool,
    pub label: Option<String>,
    pub uuid: Option<String>,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub rev: Option<String>,
    pub vendor: Option<String>,
    pub hctl: Option<String>,
}

impl Lsblk {
    pub fn retrieve() -> anyhow::Result<Self> {
        let result = Command::new("lsblk")
            .arg("-Jo")
            .arg("name,rm,size,ro,type,mountpoint,hotplug,label,uuid,model,serial,rev,vendor,hctl")
            .output()?;
        
        ensure!(result.status.success(),"lsblk error code {:?}",result.status.code());
        let data: Lsblk = serde_json::from_slice(&result.stdout[..])?;

        Ok(data)
    }
}

#[test]
fn test_retrieve() {
    println!("{:?}",Lsblk::retrieve().unwrap());
}
