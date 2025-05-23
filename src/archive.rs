// use std::{fs::File, path::PathBuf, str::FromStr};
// use flate2::{read::GzEncoder, Compression};
// use tar::{Archive, Builder};
use std::{fs::File, path::{Path, PathBuf}};
use tar::{Archive};

// /// 归档
// pub fn archive() -> Result<(), std::io::Error> {
//     let tar_gz = File::create("templates/bassinet_coin.tar.gz")?;
//     let enc = GzEncoder::new(tar_gz, Compression::default());
//     let mut ar = Builder::new(enc);
//     let path = PathBuf::from_str("templates/bassinet_coin").unwrap();
//     ar.append_dir_all("bassinet_coin", path).unwrap();
//     ar.finish().unwrap();
//     Ok(())
// }

/// 解压到
pub fn unpack(dest_apth: &PathBuf) -> Result<(), std::io::Error> {
    let template_path = std::env::var("BASSINET_TEMPLATE_PATH").expect("BASSINET_TEMPLATE_PATH must be set");
    let template_path = Path::new(&template_path);
    // let path = "templates/bassinet_coin.tar.gz";
    let path = template_path.join("bassinet_coin.tar.gz");
    let tar_gz = File::open(path)?;
    let mut archive = Archive::new(tar_gz);
    archive.unpack(dest_apth)?;
    Ok(())
}

// /// 归档bassinet_nft
// pub fn archive_bassinet() -> Result<(), std::io::Error> {
//     let tar_gz = File::create("templates/bassinet.tar.gz")?;
//     let enc = GzEncoder::new(tar_gz, Compression::default());
//     let mut ar = Builder::new(enc);
//     let path = PathBuf::from_str("templates/bassinet").unwrap();
//     ar.append_dir_all("", path).unwrap();
//     ar.finish().unwrap();
//     Ok(())
// }

/// 解压到
pub fn unpack_bassinet(dest_apth: &PathBuf) -> Result<(), std::io::Error> {
    let template_path = std::env::var("BASSINET_TEMPLATE_PATH").expect("BASSINET_TEMPLATE_PATH must be set");
    let template_path = Path::new(&template_path);
    // let path = "templates/bassinet.tar.gz";
    let path = template_path.join("bassinet.tar.gz");
    let tar_gz = File::open(path)?;
    let mut archive = Archive::new(tar_gz);
    archive.unpack(dest_apth)?;
    Ok(())
}