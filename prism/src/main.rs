use lri_rs::LriFile;

fn main() {
    let file_name = std::env::args().nth(1).unwrap();
    let bytes = std::fs::read(file_name).unwrap();
    let lri = LriFile::decode(bytes);

    println!("{} blocks", lri.blocks.len());
    println!("{} images", lri.image_count());
}

/*fn good(models: &[&SensorModel], img: RawImage, img_id: usize) {
    let RawImage {
        sensor_id,
        width,
        height,
        format,
        data,
    } = img;

    println!(
        "{sensor_id} {width}x{height} {format} - {} kB",
        data.len() / 1024
    );
    return;

    for model in models {
        println!("{:?}", model.whitepoint);
    }

    for color in models {
        let size = width * height;
        let mut ten_data = vec![0; size];
        crate::unpack::tenbit(data, width * height, ten_data.as_mut_slice());

        let mut rawimg: Image<u16, BayerRgb> = Image::from_raw_parts(
            4160,
            3120,
            RawMetadata {
                whitebalance: [1.0 / color.rg, 1.0, 1.0 / color.bg],
                whitelevels: [1024, 1024, 1024],
                crop: None,
                cfa: CFA::new("BGGR"),
                cam_to_xyz: Matrix3::from_row_slice(&color.forward_matrix),
            },
            ten_data,
        );

        /*rawimg
        .data
        .iter_mut()
        .for_each(|p| *p = p.saturating_sub(42));*/

        rawimg.whitebalance();
        let img = rawimg.debayer();
        let srgb = img.to_xyz().to_linsrgb().gamma();
        let bytes = srgb.floats().bytes();

        make_png(
            format!("tenbit_{img_id}_{:?}.png", color.whitepoint),
            width,
            height,
            &bytes.data,
        );
    }
}*/

fn make_png<P: AsRef<std::path::Path>>(path: P, width: usize, height: usize, data: &[u8]) {
    //return;
    use std::fs::File;

    let file = File::create(path).unwrap();
    let mut enc = png::Encoder::new(file, width as u32, height as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().unwrap();
    writer.write_image_data(data).unwrap();
}
