use std::collections::HashMap;

use camino::Utf8PathBuf;
use lri_rs::{AwbGain, CameraId, LriFile, RawData, RawImage, SensorModel, Whitepoint};
use nalgebra::{Matrix3, Matrix3x1};

mod rotate;
mod unpack;

pub struct Entry {
	sensor: SensorModel,
	count: usize,
}

fn main() {
	let args = std::env::args().skip(1);

	if args.len() != 2 {
		eprintln!("Usage: prism <lri_file> <output_directory>");
		std::process::exit(1);
	}

	let file_name = std::env::args().nth(1).unwrap();
	let directory = Utf8PathBuf::from(std::env::args().nth(2).unwrap());

	if !directory.exists() {
		std::fs::create_dir_all(&directory).unwrap();
	}

	let bytes = std::fs::read(file_name).unwrap();
	let lri = LriFile::decode(&bytes);
	let gain = lri.awb_gain.unwrap();

	println!("{} images", lri.image_count());

	if let Some(refimg) = lri.reference_image() {
		make(refimg, directory.join("reference.png"), gain);
	}

	let mut set: HashMap<CameraId, Entry> = HashMap::new();

	for img in lri.images() {
		set.entry(img.camera)
			.and_modify(|e| e.count += 1)
			.or_insert(Entry {
				sensor: img.sensor,
				count: 1,
			});
	}

	/*set.into_iter().for_each(|kv| {
		println!("{} {:?} {}", kv.0, kv.1.sensor, kv.1.count);
	});*/

	for (idx, img) in lri.images().enumerate() {
		make(img, directory.join(format!("image_{idx}.png")), gain);
	}
}

fn make(img: &RawImage, path: Utf8PathBuf, awb_gain: AwbGain) {
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

	let bayered = bayer(data, *width, *height);

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

	if !color.is_empty() {
		print!("\tAvailable whitepoints: ");
		color.iter().for_each(|c| print!("{:?} ", c.whitepoint));
		println!();
	}

	match img.color_info(Whitepoint::D65) {
		Some(c) => {
			println!("\tUsing D65");
			let to_xyz = Matrix3::from_row_slice(&c.forward_matrix);
			// We're using Whitepoint::D65, but there is no D50 profile.
			// If we use the BRUCE_XYZ_RGB_D65 matrix the image
			// comes out too warm.
			let to_srgb = Matrix3::from_row_slice(&BRUCE_XYZ_RGB_D50);

			let premul = to_xyz * to_srgb;

			for chnk in floats.chunks_mut(3) {
				/*let r = chnk[0] * (1.0 / c.rg);
				let g = chnk[1];
				let b = chnk[2] * (1.0 / c.bg);*/
				let r = chnk[0] * awb_gain.r;
				let g = chnk[1];
				let b = chnk[2] * awb_gain.b;

				let px = Matrix3x1::new(r, g, b);
				let rgb = premul * px;

				chnk[0] = srgb_gamma(rgb[0]) * 255.0;
				chnk[1] = srgb_gamma(rgb[1]) * 255.0;
				chnk[2] = srgb_gamma(rgb[2]) * 255.0;
			}
		}
		None => {
			println!("\tColor profile for D65 not found. Doing gamma and nothing else!");
			floats.iter_mut().for_each(|f| *f = srgb_gamma(*f) * 255.0);
		}
	}

	let bytes: Vec<u8> = floats.into_iter().map(|f| f as u8).collect();

	println!("\tWriting {}", &path);
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

fn bayer(data: &RawData<'_>, width: usize, height: usize) -> Vec<u8> {
	match data {
		RawData::Packed10bpp { data } => {
			let size = width * height;
			let mut ten_data = vec![0; size];
			unpack::tenbit(data, width * height, ten_data.as_mut_slice());

			// I've only seen it on one color defintion or
			// something, but there's a black level of 42, so subtract it.
			// without it the image is entirely too red.
			//ten_data.iter_mut().for_each(|p| *p = p.saturating_sub(42));

			ten_data
				.into_iter()
				.map(|p| ((p.saturating_sub(42)) >> 2) as u8)
				.collect()
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
