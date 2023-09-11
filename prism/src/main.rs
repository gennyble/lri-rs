use lri_rs::{LriFile, RawImage, Whitepoint};
use nalgebra::{Matrix3, Matrix3x1};

mod unpack;

fn main() {
	let file_name = std::env::args().nth(1).unwrap();
	let bytes = std::fs::read(file_name).unwrap();
	let lri = LriFile::decode(&bytes);

	println!("{} images", lri.image_count());

	for (idx, img) in lri.images().enumerate() {
		make(img, format!("image_{idx}.png"));
	}
}

// R G R G
// G B G B
// R G R G

const CFAS: &[&'static str] = &["RGGB", "GRBG", "GBRG", "BGGR"];

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

	// Assume 10-bit
	let size = width * height;
	let mut ten_data = vec![0; size];
	unpack::tenbit(data, width * height, ten_data.as_mut_slice());

	// I've only seen it on one color defintion or
	// something, but there's a black level of 42, so subtract it
	ten_data.iter_mut().for_each(|p| *p = p.saturating_sub(42));

	// B G B G B G
	// G R G R G R

	// A1 - 1:0
	// A2 - -1:-1
	// A3 - 1:0
	// A4 - 1:0
	// A5 - 0:1

	// B1 - NO
	// B2 - RO
	// B3 - RO
	// B4 - RO
	// B5 - NO

	// C1 - NO
	// C2 - RO
	// C3 - NO
	// C4 - RO
	// C5 - RO
	// C6 - -1:-1

	let (rgb, color_format) = match img.cfa_string() {
		Some(cfa_string) => {
			let rawimg: Image<u16, BayerRgb> = Image::from_raw_parts(
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
				ten_data,
			);

			(rawimg.debayer().data, png::ColorType::Rgb)
		}
		None => (ten_data, png::ColorType::Grayscale),
	};

	let mut floats: Vec<f32> = rgb.into_iter().map(|p| p as f32 / 1023.0).collect();

	print!("\t");
	color.iter().for_each(|c| print!("{:?} ", c.whitepoint));
	println!();

	match img.daylight() {
		Some(c) => {
			//println!("\tApplying color profile: {:?}", c.color_matrix);
			let to_xyz = Matrix3::from_row_slice(&c.forward_matrix);
			let to_srgb = Matrix3::from_row_slice(&BRUCE_XYZ_RGB_D65);
			//let color = Matrix3::from_row_slice(&c.color_matrix);

			let premul = to_xyz * to_srgb;

			for chnk in floats.chunks_mut(3) {
				let r = chnk[0] * (1.0 / c.rg);
				let g = chnk[1];
				let b = chnk[2] * (1.0 / c.bg);

				let px = Matrix3x1::new(r, g, b);

				//let rgb = premul * px;
				let xyz = to_xyz * px;
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
	3.2404542,  -1.5371385, -0.4985314,
	-0.9692660,  1.8760108,  0.0415560,
 	0.0556434,  -0.2040259,  1.0572252
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
