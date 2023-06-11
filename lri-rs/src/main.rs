mod proto;

use std::io::Read;

use protobuf::Message;

use proto::lightheader::LightHeader;

fn read() -> anyhow::Result<()> {
    let mut f = std::fs::File::open("/home/dllu/pictures/l16/L16_00078.lri")?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;

    let asdf = LightHeader::parse_from_bytes(&buf)?;
    dbg!(&asdf.get_device_model_name());
    dbg!(&asdf.get_device_fw_version());
    Ok(())
}

fn main() {
    read().unwrap();
}
