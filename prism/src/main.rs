use std::collections::HashMap;

use lri_rs::{CameraId, DataFormat, LriFile, RawData, RawImage, SensorModel, Whitepoint};
use nalgebra::{Matrix3, Matrix3x1};

mod rotate;
mod unpack;

pub struct Entry {
	sensor: SensorModel,
	count: usize,
}

fn main() {
	let file_name = std::env::args().nth(1).unwrap();
	let bytes = std::fs::read(file_name).unwrap();
	let lri = LriFile::decode(&bytes);

	println!("{} images", lri.image_count());

	lri.reference_image()
		.map(|raw| make(raw, String::from("reference.png")));

	let mut set: HashMap<CameraId, Entry> = HashMap::new();

	for img in lri.images() {
		set.entry(img.camera)
			.and_modify(|e| e.count += 1)
			.or_insert(Entry {
				sensor: img.sensor,
				count: 1,
			});
	}

	set.into_iter().for_each(|kv| {
		println!("{} {:?} {}", kv.0, kv.1.sensor, kv.1.count);
	});

	for (idx, img) in lri.images().enumerate() {
		/*for color in &img.color {
			println!(
				"{:?} rg = {}  bg = {}",
				color.whitepoint, color.rg, color.bg
			);

			let white =
				Matrix3::from_row_slice(&color.forward_matrix) * Matrix3x1::new(1.0, 1.0, 1.0);

			let white_x = white[0] / (white[0] + white[1] + white[2]);
			let white_y = white[1] / (white[0] + white[1] + white[2]);
			let white_z = 1.0 - white_x - white_y;

			println!("\twhite: x = {} y = {} z = {}", white_x, white_y, white_z);

			println!("\t{:?}", color.forward_matrix);
		}*/
		//std::process::exit(0);

		make(img, format!("image_{idx}.png"));
		//return;
	}
}

fn make(img: &RawImage, path: String) {
	use rawproc::image::RawMetadata;
	use rawproc::{colorspace::BayerRgb, image::Image};

	let RawImage {
		camera,
		sensor,
		width,
		height,
		format,
		data,
		sbro,
		color,
	} = img;

	println!(
		"{camera} {sensor:?} [{}:{}] {width}x{height} {format}",
		sbro.0, sbro.1
	);

	let mut bayered = bayer(
		data,
		*width,
		*height,
		format!("{}_bjpg", &path[..path.len() - 4]),
	);

	bayered.iter_mut().for_each(|p| *p = p.saturating_sub(42));

	let (mut rgb, color_format) = match img.cfa_string() {
		Some(cfa_string) => {
			let rawimg: Image<u8, BayerRgb> = Image::from_raw_parts(
				4160,
				3120,
				// We only care about CFA here because all we're doing is debayering
				RawMetadata {
					whitebalance: [1.0; 3],
					whitelevels: [1024; 3],
					crop: None,
					// ugh CFA isn't exposed, so we pulled in rawloader for now
					cfa: rawloader::CFA::new(cfa_string),
					cam_to_xyz: nalgebra::Matrix3::zeros(),
				},
				bayered,
			);

			(rawimg.debayer().data, png::ColorType::Rgb)
			//(bayered, png::ColorType::Grayscale)
		}
		None => (bayered, png::ColorType::Grayscale),
	};

	rotate::rotate_180(rgb.as_mut_slice());

	let mut floats: Vec<f32> = rgb.into_iter().map(|p| p as f32 / 255.0).collect();

	if color.len() > 0 {
		print!("\t");
		color.iter().for_each(|c| print!("{:?} ", c.whitepoint));
		println!();
	}

	match img.color_info(Whitepoint::F11) {
		Some(c) => {
			//println!("\tApplying color profile: {:?}", c.color_matrix);
			let to_xyz = Matrix3::from_row_slice(&c.forward_matrix);
			let to_srgb = Matrix3::from_row_slice(&BRUCE_XYZ_RGB_D65);
			let color = Matrix3::from_row_slice(&c.color_matrix);
			let d50_d65 = Matrix3::from_row_slice(&BRADFORD_D50_D65);

			let xyz_d65 = to_xyz * d50_d65;

			//println!("{color}");

			let white = xyz_d65 * Matrix3x1::new(1.0, 1.0, 1.0);

			let white_x = white[0] / (white[0] + white[1] + white[2]);
			let white_y = white[1] / (white[0] + white[1] + white[2]);
			let white_z = 1.0 - white_x - white_y;

			/*println!(
				"\t{:?} ||| white: x = {} y = {} z = {}",
				c.whitepoint, white_x, white_y, white_z
			);*/

			let premul = to_xyz * to_srgb;

			let prenorm = premul.normalize();
			//println!("{prenorm}");

			for chnk in floats.chunks_mut(3) {
				let r = chnk[0] * (1.0 / c.rg);
				let g = chnk[1];
				let b = chnk[2] * (1.0 / c.bg);

				let px = Matrix3x1::new(r, g, b);

				//let rgb = premul * px;
				//let px = color * px;
				let xyz = to_xyz * px;
				//let xyz = d50_d65 * xyz;
				//let xyz_white = color * xyz;
				let rgb = to_srgb * xyz;

				chnk[0] = srgb_gamma(rgb[0]) * 255.0;
				chnk[1] = srgb_gamma(rgb[1]) * 255.0;
				chnk[2] = srgb_gamma(rgb[2]) * 255.0;
			}
		}
		None => {
			println!("\tno color profile found");
			floats.iter_mut().for_each(|f| *f = srgb_gamma(*f) * 255.0);
		}
	}

	let bytes: Vec<u8> = floats.into_iter().map(|f| f as u8).collect();

	println!("Writing {}", &path);
	make_png(path, *width, *height, &bytes, color_format)
}

#[rustfmt::skip]
#[allow(dead_code)]
const BRUCE_XYZ_RGB_D50: [f32; 9] = [
	3.1338561,  -1.6168667, -0.4906146,
	-0.9787684,  1.9161415,  0.0334540,
	0.0719453,  -0.2289914,  1.4052427
];

#[rustfmt::skip]
const BRUCE_XYZ_RGB_D65: [f32; 9] = [
	 3.2404542, -1.5371385, -0.4985314,
	-0.9692660,  1.8760108,  0.0415560,
 	 0.0556434, -0.2040259,  1.0572252
];

#[rustfmt::skip]
const BRADFORD_D50_D65: [f32; 9] = [
	 0.9555766, -0.0230393,  0.0631636,
	-0.0282895,  1.0099416,  0.0210077,
	 0.0122982, -0.0204830,  1.3299098,
];

#[inline]
pub fn srgb_gamma(mut float: f32) -> f32 {
	if float <= 0.0031308 {
		float *= 12.92;
	} else {
		float = float.powf(1.0 / 2.4) * 1.055 - 0.055;
	}

	float.clamp(0.0, 1.0)
}

fn bayer(data: &RawData<'_>, width: usize, height: usize, path: String) -> Vec<u8> {
	match data {
		RawData::Packed10bpp { data } => {
			// Assume 10-bit
			let size = width * height;
			let mut ten_data = vec![0; size];
			unpack::tenbit(data, width * height, ten_data.as_mut_slice());

			// I've only seen it on one color defintion or
			// something, but there's a black level of 42, so subtract it
			//ten_data.iter_mut().for_each(|p| *p = p.saturating_sub(42));

			ten_data.into_iter().map(|p| (p >> 2) as u8).collect()
		}
		RawData::BayerJpeg {
			header: _,
			format,
			jpeg0,
			jpeg1,
			jpeg2,
			jpeg3,
		} => {
			let mut bayered = vec![0; width * height];

			match format {
				0 => {
					let mut into = vec![0; (width * height) / 4];

					let mut channel = |jpeg: &[u8], offset: usize| {
						zune_jpeg::JpegDecoder::new(jpeg)
							.decode_into(&mut into)
							.unwrap();

						for idx in 0..into.len() {
							let ww = width / 2;
							let in_x = idx % ww;
							let in_y = idx / ww;

							let bayer_x = (in_x * 2) + (offset % 2);
							let bayer_y = (in_y * 2) + (offset / 2);

							let bayer_idx = bayer_y * width + bayer_x;
							bayered[bayer_idx] = into[idx];
						}
					};

					//BGGR
					//RGGB
					//GRBG
					channel(jpeg0, 0);
					channel(jpeg1, 1);
					channel(jpeg2, 2);
					channel(jpeg3, 3);
				}
				1 => {
					zune_jpeg::JpegDecoder::new(jpeg0)
						.decode_into(&mut bayered)
						.unwrap();
				}
				_ => unreachable!(),
			}

			bayered
		}
	}
}

fn make_png<P: AsRef<std::path::Path>>(
	path: P,
	width: usize,
	height: usize,
	data: &[u8],
	color_format: png::ColorType,
) {
	//return;
	use std::fs::File;

	let file = File::create(path).unwrap();
	let mut enc = png::Encoder::new(file, width as u32, height as u32);
	enc.set_color(color_format);
	enc.set_depth(png::BitDepth::Eight);
	let mut writer = enc.write_header().unwrap();
	writer.write_image_data(data).unwrap();
}
