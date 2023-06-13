use std::{fs::File, io::Write, os::unix::prelude::FileExt, path::Path};

use lri_rs::{proto::camera_module::CameraModule, Message};
use nalgebra::Matrix3;
use png::{BitDepth, ColorType};
use rawloader::CFA;
use rawproc::{
	colorspace::BayerRgb,
	image::{Image, RawMetadata},
};
use unpacker::Unpacker;

// This code is going to be rough. Just trying to parse this using the technique
// I know: just play with the raw data
fn main() {
	let fname = std::env::args().nth(1).unwrap();
	let mut data = std::fs::read(fname).unwrap();

	println!("Read {:.2}MB", data.len() as f32 / (1024.0 * 1024.0));

	let mut blocks = vec![];

	loop {
		let header = DataHeader::new(&data[..]);
		let end = header.combined_length as usize;
		if end == data.len() {
			blocks.push(Block { header, data });
			break;
		} else {
			let remain = data.split_off(end);
			blocks.push(Block { header, data });
			data = remain;
		}
	}

	println!("Found {} blocks", blocks.len());

	for (idx, block) in blocks.iter().enumerate() {
		if block.is_sensor() {
			println!("\nIDX {idx}");
			block.header.print_info();
			fuckwithsensordata(block, idx);
		} else {
			block.header.nice_info();
		}
	}

	/*
	// Grabbed, quickly, from the sensor datasheets. (or in the case of the
	// imx386 on some random website (canwe have a datasheet? shit)).
	let ar835 = 3264 * 2448;
	let ar835_6mp = 3264 * 1836;
	let ar1335 = 4208 * 3120;
	let imx386 = 4032 * 3024;

	// Determined by lak experimentally
	let ar1335_crop = 4160 * 3120;

	println!("\nAttemtping to unpack image in idx0");
	let head = &heads[0];
	let mut msg = body(head, &data);
	for AHH in 0..2 {
		let mut up = Unpacker::new();
		for idx in (0..16224000 * 2).rev() {
			up.push(msg[idx]);
		}
		up.finish();

		dump(&msg[..16224000], "fordatadog.packed");

		let mut imgdata = vec![];
		for (idx, chnk) in up.out.chunks(2).enumerate() {
			let mut sixteen = (u16::from_le_bytes([chnk[0], chnk[1]]) as f32 / 1024.0) * 255.0;

			imgdata.push(sixteen.min(255.0) as u8);
		}

		let rawimg: Image<u8, BayerRgb> = Image::from_raw_parts(
			4160,
			3120 * 2,
			RawMetadata {
				whitebalance: [1.0, 1.0, 1.35],
				whitelevels: [1024, 1024, 1024],
				crop: None,
				cfa: CFA::new("BGGR"),
				cam_to_xyz: Matrix3::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0),
			},
			imgdata.clone(),
		);
		let mut img = rawimg.debayer();

		for px in img.data.chunks_mut(3) {
			px[0] = (px[0] as f32 * 1.95).min(255.0) as u8;
			px[2] = (px[2] as f32 * 1.36).min(255.0) as u8;
		}

		let png = format!("image_{AHH}.png");
		make_png(
			&png,
			4160,
			3120 * 2,
			ColorType::Rgb,
			BitDepth::Eight,
			&img.data,
		);
		println!("Wrote {png}");

		msg = &msg[16224000..]; // + head.header.message_length as usize * 2..];
	}

	println!("\nDumping the Message of idx 4");
	dump_body(&heads[4], &data, "msg4.lri_part");

	let mut modules = vec![];
	let mut sensor_data = vec![];

	for (idx, head) in heads.iter().enumerate() {
		print!("Head {idx} - ");
		let msg = body(head, &data);

		match (head.header.header_length == 32, head.header.kind) {
			(true, 1) => {
				match lri_rs::proto::view_preferences::ViewPreferences::parse_from_bytes(msg) {
					Ok(_) => println!("View Preferences: Parsed"),
					Err(e) => println!("View Preferences, failed: {e}"),
				}
			}
			(true, 0) => match lri_rs::proto::lightheader::LightHeader::parse_from_bytes(msg) {
				Ok(data) => {
					let mods = &data.modules;
					let datas = &data.sensor_data;

					print!(
						" [claimed: {} | actual: {}] - ",
						head.header.message_length,
						data.compute_size()
					);

					println!(
						"LightHeader! Modules: {} - Datas: {} \\ ModCal: {}",
						mods.len(),
						datas.len(),
						data.module_calibration.len()
					);
					modules.extend_from_slice(&mods);
					sensor_data.extend_from_slice(&datas);

					if false && data.module_calibration.len() > 0 {
						for modc in data.module_calibration {
							print!(" - {:?}", modc.get_camera_id());
						}
						println!("");
					}
				}
				Err(e) => println!("LightHeader, failed: {e}"),
			},
			(true, knd) => {
				println!("Unknown header kind [{knd}] and header_length is 32, skipping...");
			}
			(false, _) => {
				println!("SensorData! Skipping for now...");
			}
		}
	}*/
}

fn fuckwithsensordata(block: &Block, idx: usize) {
	let Block { header, data } = block;

	let clen = header.combined_length;
	let hlen = header.header_length;
	let mlen = header.message_length;

	println!("\n== Fuck With Sensor Data {idx} ==");

	println!("Combined: {clen}");
	println!("Header:   {hlen}");
	println!("Message:  {mlen}\n");

	let width = 4160;
	let height = 3120;
	let pixel_count = width * height;
	let packed_count = ((pixel_count as f32 * 10.0) / 8.0) as usize;

	println!("Assuming {width}x{height} [{pixel_count}] [packed: {packed_count}]");

	let mut data = block.body();
	// I'm lazy and don't want to manually increment
	for x in 0..10 {
		let fname = format!("block{idx}_image{x}.png");

		// Use my really efficient (read that sarcastically, please) 10-bit unpacker
		let mut up = Unpacker::new();
		for idx in (0..packed_count).rev() {
			up.push(data[idx]);
		}
		up.finish();

		// Sixteen - eightbits
		let mut imgdata = vec![];
		for chnk in up.out.chunks(2) {
			let sixteen = (u16::from_le_bytes([chnk[0], chnk[1]]) as f32 / 1024.0) * 255.0;

			imgdata.push(sixteen.min(255.0) as u8);
		}

		// we want it to be RGB not weird bayer
		let rawimg: Image<u8, BayerRgb> = Image::from_raw_parts(
			width,
			height,
			// use mostly fake data except the CFA
			RawMetadata {
				whitebalance: [1.0, 1.0, 1.0],
				whitelevels: [1024, 1024, 1024],
				crop: None,
				cfa: CFA::new("BGGR"),
				cam_to_xyz: Matrix3::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0),
			},
			imgdata,
		);
		let img = rawimg.debayer();

		// Yay PNG
		make_png(
			&fname,
			width,
			height,
			ColorType::Rgb,
			BitDepth::Eight,
			&img.data,
		);
		println!("Wrote file {fname}");

		let skip = packed_count + mlen as usize;
		if data.len() <= skip + packed_count {
			println!(
				"Only {} bytes will be left in data after output! Which is not enough",
				data.len() - skip
			);
			break;
		} else {
			data = &data[skip..]
		}
	}

	println!("===================================\n");
}

fn dump(data: &[u8], path: &str) {
	let mut file = File::create(&path).unwrap();
	file.write_all(data).unwrap();
	println!(
		"Wrote {:.2}KB to disk as {path}",
		data.len() as f32 / 1024.0
	);
}

fn make_png<P: AsRef<Path>>(
	path: P,
	width: usize,
	height: usize,
	color: ColorType,
	depth: BitDepth,
	data: &[u8],
) {
	let bpp = match (color, depth) {
		(ColorType::Grayscale, BitDepth::Eight) => 1,
		(ColorType::Grayscale, BitDepth::Sixteen) => 2,
		(ColorType::Rgb, BitDepth::Eight) => 3,
		(ColorType::Rgb, BitDepth::Sixteen) => 6,
		_ => panic!("unsupported color or depth"),
	};

	let pix = width * height;

	let file = File::create(path).unwrap();
	let mut enc = png::Encoder::new(file, width as u32, height as u32);
	enc.set_color(color);
	enc.set_depth(depth);
	let mut writer = enc.write_header().unwrap();
	writer.write_image_data(&data[..pix * bpp]).unwrap();
}

#[derive(Clone, Debug)]
struct Block {
	header: DataHeader,
	data: Vec<u8>,
}

impl Block {
	pub fn body(&self) -> &[u8] {
		&self.data[32..]
	}

	/// Block contains sensor data.
	pub fn is_sensor(&self) -> bool {
		self.header.header_length != 32
	}
}

#[derive(Clone, Debug)]
struct DataHeader {
	magic_number: String,
	combined_length: u64,
	//FIXME: This appears to be the content length and not the header length? I thought
	//it was weird that they were putting the header length here. Is the java decomp
	//wrong?
	header_length: u64,
	message_length: u32,
	// type
	kind: u8,
	reserved: [u8; 7],
}

impl DataHeader {
	pub fn new(data: &[u8]) -> Self {
		let magic_number = String::from_utf8(data[0..4].to_vec()).unwrap();
		let combined_length = u64::from_le_bytes(data[4..12].try_into().unwrap());
		//println!("Combined Length: {:?}", &data[4..12]);

		let header_length = u64::from_le_bytes(data[12..20].try_into().unwrap());
		//println!("Header Length: {:?}", &data[12..20]);

		let message_length = u32::from_le_bytes(data[20..24].try_into().unwrap());
		//println!("Message Length: {:?}", &data[20..24]);

		let kind = data[24];
		let reserved = data[25..32].try_into().unwrap();

		DataHeader {
			magic_number,
			combined_length,
			header_length,
			message_length,
			kind,
			reserved,
		}
	}

	pub fn print_info(&self) {
		let Self {
			magic_number,
			combined_length,
			header_length,
			message_length,
			kind,
			reserved,
		} = self;

		let combined_human = humanish(*combined_length as usize);
		let header_human = humanish(*header_length as usize);
		let message_human = humanish(*message_length as usize);

		println!("Magic: {magic_number}\nCombined Length: {combined_human}\nHeader Length: {header_human}\nMessage Length: {message_human}\nKind: {kind}\nReserved: {reserved:?}");
	}

	pub fn nice_info(&self) {
		let Self {
			magic_number: _a,
			combined_length: _b,
			header_length,
			message_length: _c,
			kind,
			reserved: _d,
		} = self;

		println!(
			"Content length: {:.2}KB | Kind {kind}",
			*header_length as f32 / 1024.0
		);
	}

	pub fn bin_info(&self) {
		let Self {
			magic_number,
			combined_length,
			header_length: _a,
			message_length: _b,
			kind: _c,
			reserved: _d,
		} = self;

		println!("{magic_number} {:b}", combined_length);
	}
}

pub fn humanish(bytes: usize) -> String {
	if bytes > 1024 * 10 {
		// Ehhhhh 10KB
		format!("{:.2} KB", bytes as f32 / 1024.0)
	} else if bytes > 1024 * 1024 {
		// A MB is enough to justify this I guess
		format!("{:.2} MB", bytes as f32 / 1024.0 * 1024.0)
	} else {
		format!("{}", bytes)
	}
}
