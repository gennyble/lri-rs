use std::{fs::File, io::Write, os::unix::prelude::FileExt, path::Path};

use lri_rs::Message;
use png::{BitDepth, ColorType};

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
			}
		}
	}

	let ar835 = 3264 * 2448;
	let ar835_6mp = 3264 * 1836;
	let ar1335 = 4208 * 3120;
	let imx386 = 4032 * 3024;

	let known_res = vec![ar835, ar835_6mp, ar1335, imx386];

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
	heads.iter().for_each(|h| h.header.print_info());

	println!("\nWriting large ones to disk!");
	for (idx, head) in heads.iter().enumerate() {
		if head.header.header_length > 1024 * 1024 {
			// I guess we only care if it's at least a megabyte
			let name = format!("{idx}.lri_part");
			let mut file = File::create(&name).unwrap();
			file.write_all(&data[head.start..head.end]).unwrap();
			println!(
				"Wrote {:.2}MB to disk as {name}",
				head.header.combined_length as f32 / (1024.0 * 1024.0)
			);
		}

		if idx == 2 {
			let data = &data[head.start + 32..head.end];

			let mut first = vec![0; data.len() / 2];
			let mut second = vec![0; data.len() / 2];
			for (idx, chnk) in data.chunks(2).enumerate() {
				first[idx] = chnk[0];
				second[idx] = chnk[0];
			}

			let name = format!("{idx}_first.lri_part");
			let mut file = File::create(&name).unwrap();
			file.write_all(&first).unwrap();

			let name = format!("{idx}_second.lri_part");
			let mut file = File::create(&name).unwrap();
			file.write_all(&second).unwrap();
		}
	}
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

	let file = File::create("ahh.png").unwrap();
	let mut enc = png::Encoder::new(file, width as u32, height as u32);
	enc.set_color(color);
	enc.set_depth(depth);
	let mut writer = enc.write_header().unwrap();
	writer.write_image_data(&data[..pix * bpp]).unwrap();
}

struct HeaderAndOffset {
	header: LightHeader,
	// Inclusive
	start: usize,
	// Exclusive
	end: usize,
}

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

		println!("\nMagic: {magic_number}\nCombined Length: {combined_length}\nHeader Length: {header_length}\nMessage Length: {message_length}\nKind: {kind}\nReserved: {reserved:?}");
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
}
