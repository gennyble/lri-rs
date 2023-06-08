use std::fs::File;

use lri_rs::Message;

// This code is going to be rough. Just trying to parse this using the technique
// I know: just play with the raw data
fn main() {
	let fname = std::env::args().nth(1).unwrap();
	let data = std::fs::read(fname).unwrap();

	println!("Read {:.2}MB", data.len() as f32 / (1024.0 * 1024.0));

	let header = LightHeader::new(&data[0..32]);
	let whole_data = &data;
	let data = &data[32..];

	let LightHeader {
		magic_number,
		combined_length,
		header_length,
		message_length,
		kind,
		reserved,
	} = header;

	//AHHH it does not seem the combined legth or header length are correct? it seems like nonsense?
	//drat. we'll know when I try and parse the message I think I extracted. 1510 bytes seems too
	//small almost.
	//the thing that makes me suspicious, and think it's right, is that the reserved are all 0x00
	//and then the next byte is data, so.
	println!("\nMagic: {magic_number}\nCombined Length: {combined_length}\nHeader Length: {header_length}\nMessage Length: {message_length}\nKind: {kind}\nReserved: {reserved:?}\nNext 8 Bytes: {:?}", &data[0..8]);

	let message = &data[..message_length as usize];
	//let data = &data[message_length as usize..];

	//let asdf = lri_rs::LightHeader::parse_from_bytes(&data).unwrap();
	//println!("{:?}", asdf.get_image_time_stamp());

	// The camera says wall.lri was taken Jun 7, 2023 at 07:14 PM. Perhaps I can search the data for this
	// to try and find a reference
	// Used protobufpal.com to make this so i can look for it. it's the year/month/day of the date
	let looking = [
		0x08, 0xe7, 0x0f, 0x10, 0x06, 0x18, 0x07, 0x20, 0x13, 0x28, 0x0e,
	];

	println!("\nTook Message of {message_length} bytes");
	println!("{} bytes left", data.len());

	println!("\nLooking for timestamp...");

	for idx in 0..data.len() - looking.len() {
		if &data[idx..idx + looking.len()] == looking.as_slice() {
			println!("Found! Offset {idx}");
		}
	}

	let magic_id = [76, 69, 76, 82];
	let magic_id_skip = 21;
	let reserved = [0, 0, 0, 0, 0, 0, 0];
	let look_length = magic_id.len() + magic_id_skip + reserved.len();

	println!("\nLooking for LELR");
	for idx in 0..whole_data.len() - look_length {
		if &whole_data[idx..idx + magic_id.len()] == magic_id.as_slice() {
			print!("Found! Offset {idx} - ");

			let reserved_start = idx + magic_id.len() + magic_id_skip;
			if &whole_data[reserved_start..reserved_start + reserved.len()] == reserved.as_slice() {
				println!("Reserved matched!");
			} else {
				println!("No reserve match :(");
			}
		}
	}

	let rt = (data.len() as f32 / 2.0).sqrt().floor() as usize;
	println!("\n{} png", rt * rt);

	println!("{:?}", &data[0..32]);

	let width = rt / 8;
	let height = rt * 4;
	let pixels = width * height;

	let file = File::create("ahh.png").unwrap();
	let mut enc = png::Encoder::new(file, width as u32, height as u32);
	enc.set_color(png::ColorType::Grayscale);
	enc.set_depth(png::BitDepth::Sixteen);
	let mut writer = enc.write_header().unwrap();
	writer
		.write_image_data(&data[rt * 3..(rt * 3) + pixels * 2])
		.unwrap();
}

struct LightHeader {
	magic_number: String,
	combined_length: u64,
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
		println!("Combined Length: {:?}", &data[4..12]);

		let header_length = u64::from_le_bytes(data[12..20].try_into().unwrap());
		println!("Header Length: {:?}", &data[12..20]);

		let message_length = u32::from_le_bytes(data[20..24].try_into().unwrap());
		println!("Message Length: {:?}", &data[20..24]);

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
}
