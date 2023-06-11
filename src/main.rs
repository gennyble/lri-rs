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
	let data = std::fs::read(fname).unwrap();

	println!("Read {:.2}MB", data.len() as f32 / (1024.0 * 1024.0));

	let magic_id = [76, 69, 76, 82];
	let magic_id_skip = 21;
	let reserved = [0, 0, 0, 0, 0, 0, 0];
	let look_length = magic_id.len() + magic_id_skip + reserved.len();

	let mut heads = vec![];
	let mut skeptical_heads = vec![];

	println!("\nLooking for LELR");
	for idx in 0..data.len() - look_length {
		if &data[idx..idx + magic_id.len()] == magic_id.as_slice() {
			print!("Found! Offset {idx} - ");

			let reserved_start = idx + magic_id.len() + magic_id_skip;
			if &data[reserved_start..reserved_start + reserved.len()] == reserved.as_slice() {
				println!("Reserved matched!");

				let header = LightHeader::new(&data[idx..]);
				let start = idx;
				let end = start + header.combined_length as usize;

				heads.push(HeaderAndOffset { header, start, end });
			} else {
				println!("No reserve match :(");

				let header = LightHeader::new(&data[idx..]);
				let start = idx;
				let end = start + header.combined_length as usize;

				skeptical_heads.push(HeaderAndOffset { header, start, end });
			}
		}
	}

	// Grabbed, quickly, from the sensor datasheets. (or in the case of the
	// imx386 on some random website (canwe have a datasheet? shit)).
	let ar835 = 3264 * 2448;
	let ar835_6mp = 3264 * 1836;
	let ar1335 = 4208 * 3120;
	let imx386 = 4032 * 3024;

	// Determined by lak experimentally
	let ar1335_crop = 4160 * 3120;

	let known_res = vec![ar835, ar835_6mp, ar1335, imx386, ar1335_crop];

	println!("\nFound {} LightHeaders", heads.len());

	println!("\nLooking for known resolutions!");
	for (idx, head) in heads.iter().enumerate() {
		for res in &known_res {
			if head.header.header_length == *res {
				println!("KNOWN RES: {}", idx);
			}
		}
	}

	println!("\nChecking if there is outlying data...");
	for idx in 1..heads.len() {
		let this = &heads[idx];
		let before = &heads[idx - 1];

		if before.end != this.start {
			println!(
				"Headers {} and {} are gapped by {} bytes",
				idx - 1,
				idx,
				this.start - before.end
			);
		} else {
			println!("{} and {} are consecutive with no gap!", idx - 1, idx);
		}
	}

	let end_difference = heads.last().unwrap().end - data.len();
	if end_difference > 0 {
		println!("{} bytes at the end", end_difference);
	} else {
		println!("File has no extraneous data at the end!");
	}

	println!("\nDumping header info..");
	heads.iter().for_each(|h| h.header.nice_info());

	println!("\nDumping skeptical header info..");
	skeptical_heads.iter().for_each(|h| h.header.bin_info());

	println!("\nWriting large ones to disk and collecting the smalls!");
	let mut small: Vec<u8> = vec![];
	for (idx, head) in heads.iter().enumerate() {
		if head.header.header_length > 1024 * 1024 {
			// I guess we only care if it's at least a megabyte
			let name = format!("{idx}.lri_part");
			let mut file = File::create(&name).unwrap();
			file.write_all(&data[head.start..head.end]).unwrap();
			println!(
				"\nWrote {:.2}MB to disk as {name}",
				head.header.combined_length as f32 / (1024.0 * 1024.0)
			);
			head.header.print_info();
		} else {
			small.extend(&data[head.start..head.end]);
		}
	}

	let mut file = File::create("small.lri_part").unwrap();
	file.write_all(&small).unwrap();
	println!(
		"Wrote {:.2}MB to disk as small.lri_part",
		small.len() as f32 / (1024.0 * 1024.0)
	);

	let stamp = [
		08, 0xe7, 0x0f, 0x10, 0x06, 0x18, 0x07, 0x20, 0x13, 0x28, 0x0e,
	];
	println!("\nLooking for timestamps!");
	for (idx, head) in find_pattern(&heads, &data, &stamp) {
		println!("Found stamp in {idx}");
	}

	println!("\nAttemtping to unpack image in idx0");
	let head = &heads[0];
	let mut msg = body(head, &data);
	for AHH in 0..2 {
		let mut up = Unpacker::new();
		for idx in (0..16224000).rev() {
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
			3120,
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
		make_png(&png, 4160, 3120, ColorType::Rgb, BitDepth::Eight, &img.data);
		println!("Wrote {png}");

		msg = &msg[16224000..]; // + head.header.message_length as usize * 2..];
	}

	let msg = &msg[16224000..16224000 + head.header.message_length as usize];
	dump(msg, "afterimg2");

	match lri_rs::proto::camera_module::CameraModule::parse_from_bytes(msg) {
		Ok(o) => println!("parsed"),
		Err(e) => println!("failed {e}"),
	}

	let question = &msg[..4352];
	let next = &msg[4352..];

	/*println!(
		"Up out is {} bytes. Expecte {}. Difference {} [work: {:0b} - idx {}]",
		up.out.len(),
		ar1335_crop * 2,
		up.out.len() as isize - (ar1335_crop * 2) as isize,
		up.work,
		up.work_idx
	);*/

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
	}
}

fn dump_body(head: &HeaderAndOffset, data: &[u8], path: &str) {
	let msg = body(head, data);
	dump(msg, path)
}

fn dump(data: &[u8], path: &str) {
	let mut file = File::create(&path).unwrap();
	file.write_all(data).unwrap();
	println!(
		"Wrote {:.2}KB to disk as {path}",
		data.len() as f32 / 1024.0
	);
}

fn body<'a>(head: &HeaderAndOffset, data: &'a [u8]) -> &'a [u8] {
	if head.header.header_length == 32 {
		&data[head.start + head.header.header_length as usize
			..head.start + head.header.header_length as usize + head.header.message_length as usize]
	} else {
		&data[head.start + 32..head.end]
	}
}

fn find_pattern<'a>(
	heads: &'a [HeaderAndOffset],
	data: &[u8],
	pattern: &[u8],
) -> Vec<(usize, &'a HeaderAndOffset)> {
	let mut finds = vec![];

	for (head_idx, head) in heads.iter().enumerate() {
		for idx in head.start..head.end - pattern.len() {
			if &data[idx..idx + pattern.len()] == pattern {
				finds.push((head_idx, head));
			}
		}
	}

	finds
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
struct HeaderAndOffset {
	header: LightHeader,
	// Inclusive
	start: usize,
	// Exclusive
	end: usize,
}

#[derive(Clone, Debug)]
struct LightHeader {
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

impl LightHeader {
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

		LightHeader {
			magic_number,
			combined_length,
			header_length,
			message_length,
			kind,
			reserved,
		}
	}

	pub fn print_info(&self) {
		let LightHeader {
			magic_number,
			combined_length,
			header_length,
			message_length,
			kind,
			reserved,
		} = self;

		println!("Magic: {magic_number}\nCombined Length: {combined_length}\nHeader Length: {header_length}\nMessage Length: {message_length}\nKind: {kind}\nReserved: {reserved:?}");
	}

	pub fn nice_info(&self) {
		let LightHeader {
			magic_number,
			combined_length,
			header_length,
			message_length,
			kind,
			reserved,
		} = self;

		println!(
			"Content length: {:.2}KB | Kind {kind}",
			*header_length as f32 / 1024.0
		);
	}

	pub fn bin_info(&self) {
		let LightHeader {
			magic_number,
			combined_length,
			header_length,
			message_length,
			kind,
			reserved,
		} = self;

		println!("{magic_number} {:b}", combined_length);
	}
}
