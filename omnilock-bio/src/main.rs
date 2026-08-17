mod crypto;
mod error;
mod flash;
mod hw_tables;
mod hwkey;
mod init_flash;
mod sensor;
mod tls;
mod usb;

use error::{Error, Result};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn read_sysinfo() -> Result<hwkey::SystemInfo> {
    hwkey::SystemInfo::read().ok_or_else(|| Error::Other("SMBIOS table not available".into()))
}

fn probe() -> Result<()> {
    let dev = usb::Device::open()?;
    let info = read_sysinfo()?;
    println!("hw_key (SMBIOS-derived): {}", hex::encode(hwkey::hw_key_bytes(&info)));

    let mut tls = init_flash::open_common(dev, &info)?;

    match sensor::RomInfo::get(&mut tls) {
        Ok(rom) => println!(
            "RomInfo: timestamp=0x{:08x} build=0x{:08x} major={} minor={} product={}",
            rom.timestamp, rom.build, rom.major, rom.minor, rom.product
        ),
        Err(e) => println!("RomInfo failed: {e}"),
    }

    match sensor::identify_sensor(&mut tls) {
        Ok(dev) => println!(
            "Sensor: major=0x{:04x} type=0x{:04x} name={}",
            dev.major, dev.dev_type, dev.name
        ),
        Err(e) => println!("identify_sensor failed: {e}"),
    }

    match flash::get_flash_info(&mut tls) {
        Ok(fi) => {
            println!(
                "Flash: IC={} ({} B), {} blocks x {} B",
                fi.ic.name, fi.ic.size, fi.blocks, fi.blocksize
            );
            for p in &fi.partitions {
                println!(
                    "  partition {}: type={} access_lvl={} offset=0x{:08x} size=0x{:08x}",
                    p.id, p.ty, p.access_lvl, p.offset, p.size
                );
            }
        }
        Err(e) => println!("get_flash_info failed: {e}"),
    }

    match sensor::get_prg_status(&mut tls) {
        Ok(rsp) => println!("prg_status: {rsp:02x?}"),
        Err(e) => println!("get_prg_status failed: {e}"),
    }

    println!("TLS session active — probe OK.");
    Ok(())
}

fn reprovision() -> Result<()> {
    let dev = usb::Device::open()?;
    let info = read_sysinfo()?;
    let mut tls = init_flash::open_common(dev, &info)?;

    let info_fi = flash::get_flash_info(&mut tls)?;
    println!("Erasing all partitions (template DB + certs)...");
    for p in info_fi.partitions.iter().map(|p| p.id) {
        match flash::erase_flash(&mut tls, p) {
            Ok(()) => println!("  partition {p} erased"),
            Err(e) => println!("  partition {p}: {e}"),
        }
    }
    println!("Done. Sensor will be re-paired on next startup (init_flash clean-slate).");
    Ok(())
}

fn print_hwkey() -> Result<()> {
    let info = read_sysinfo()?;
    println!("Product:  {}", info.product_name);
    println!("Serial:   {}", info.serial_number);
    println!("hw_key:   {}", hex::encode(hwkey::hw_key_bytes(&info)));
    Ok(())
}

fn run(cmd: &str) -> Result<()> {
    match cmd {
        "probe" => probe(),
        "reprovision" => reprovision(),
        "hwkey" => print_hwkey(),
        "reboot" => {
            let dev = usb::Device::open()?;
            let info = read_sysinfo()?;
            let mut tls = init_flash::open_common(dev, &info)?;
            sensor::reboot(&mut tls)
        }
        "version" => {
            println!("omnilock-bio {VERSION}");
            Ok(())
        }
        other => Err(Error::Other(format!(
            "unknown command '{other}' (expected: probe | reprovision | reboot | hwkey | version)"
        ))),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(|s| s.as_str()).unwrap_or("probe");

    let result = run(cmd);

    match result {
        Ok(()) => std::process::exit(0),
        Err(Error::PairingFailed) => {
            eprintln!(
                "Signature verification failed. This device was probably paired with another \
                 computer (e.g. by the Windows Hello driver)."
            );
            eprintln!("Run 'omnilock-bio reprovision' to erase it and re-pair with this machine.");
            std::process::exit(3);
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}