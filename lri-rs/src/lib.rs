use std::time::Duration;

use block::{Block, ExtractedData, Header};

mod block;
mod types;

pub use types::*;

pub struct LriFile<'lri> {
	pub image_reference_camera: Option<CameraId>,
	pub images: Vec<RawImage<'lri>>,
	pub colors: Vec<ColorInfo>,
	pub camera_infos: Vec<CameraInfo>,

	pub focal_length: Option<i32>,
	pub firmware_version: Option<String>,
	pub image_integration_time: Option<Duration>,
	pub af_achieved: Option<bool>,
	pub image_gain: Option<f32>,
	pub hdr: Option<HdrMode>,
	pub scene: Option<SceneMode>,
	pub on_tripod: Option<bool>,
	pub awb: Option<AwbMode>,
	pub awb_gain: Option<AwbGain>,
}

impl<'lri> LriFile<'lri> {
	/// Read
	pub fn decode(mut data: &'lri [u8]) -> Self {
		let mut images = vec![];
		let mut colors = vec![];
		let mut camera_infos = vec![];

		let mut ext = ExtractedData::default();

		// Read data blocks and extract informtion we care about
		loop {
			if data.is_empty() {
				break;
			}

			let header = Header::ingest(&data[..]);
			let end = header.block_length as usize;

			let block_data = &data[..end];
			data = &data[end..];

			let block = Block {
				header,
				data: block_data,
			};

			block.extract_meaningful_data(&mut ext, &mut images, &mut colors, &mut camera_infos);
		}

		// Further fill in the RawImage's we extracted
		for img in images.iter_mut() {
			if let Some(info) = camera_infos.iter().find(|i| i.camera == img.camera) {
				img.sensor = info.sensor;
			}

			let profiles = colors
				.iter()
				.filter(|c| c.camera == img.camera)
				.map(<_>::clone)
				.collect();

			img.color = profiles;
		}

		LriFile {
			image_reference_camera: ext.reference_camera,
			images,
			colors,
			camera_infos,

			firmware_version: ext.fw_version,
			focal_length: ext.focal_length,
			image_integration_time: ext.image_integration_time,
			af_achieved: ext.af_achieved,
			image_gain: ext.image_gain,
			hdr: ext.hdr,
			scene: ext.scene,
			on_tripod: ext.on_tripod,
			awb: ext.awb,
			awb_gain: ext.awb_gain,
		}
	}

	/// Number of images present in the file
	pub fn image_count(&self) -> usize {
		self.images.len()
	}

	/// Iterator over the images
	pub fn images(&self) -> std::slice::Iter<'_, RawImage> {
		self.images.iter()
	}

	/// Get the image the camera showed in the viewfinder, if it's been
	/// recorded in the file.
	pub fn reference_image(&self) -> Option<&RawImage> {
		self.image_reference_camera
			.map(|irc| self.images().find(|ri| ri.camera == irc))
			.flatten()
	}
}

pub enum RawData<'img> {
	BayerJpeg {
		header: &'img [u8],
		format: u32,
		jpeg0: &'img [u8],
		jpeg1: &'img [u8],
		jpeg2: &'img [u8],
		jpeg3: &'img [u8],
	},
	Packed10bpp {
		data: &'img [u8],
	},
}

pub struct RawImage<'img> {
	/// Camera that captured this image
	pub camera: CameraId,
	/// The model of the sensor of the camera
	pub sensor: SensorModel,

	pub width: usize,
	pub height: usize,

	/// What format the data is in
	pub format: DataFormat,
	pub data: RawData<'img>,
	/// "sensor bayer red offset"
	pub sbro: (i32, i32),
	/// All color information associated with this [CameraId] for different [Whitepoint]s
	pub color: Vec<ColorInfo>,
}

impl<'img> RawImage<'img> {
	/// Get the color profile for noon daylight. First looks for F7 and, if it can't find that, D65
	pub fn daylight(&self) -> Option<&ColorInfo> {
		self.color
			.iter()
			.find(|c| c.whitepoint == Whitepoint::F7)
			.or_else(|| self.color.iter().find(|c| c.whitepoint == Whitepoint::D65))
	}

	/// Get a color profile matching the provided Whitepoint
	pub fn color_info(&self, whitepoint: Whitepoint) -> Option<&ColorInfo> {
		self.color.iter().find(|c| c.whitepoint == whitepoint)
	}

	pub fn cfa_string(&self) -> Option<&'static str> {
		match self.sensor {
			SensorModel::Ar1335Mono => None,
			SensorModel::Ar1335 => self.cfa_string_ar1335(),
			_ => unimplemented!(),
		}
	}

	// The AR1335 seems to be BGGR, which was weird.
	fn cfa_string_ar1335(&self) -> Option<&'static str> {
		//if self.format == DataFormat::BayerJpeg {
		//	Some("BGGR")
		//} else {
		match self.sbro {
			(-1, -1) => None,
			(0, 0) => Some("BGGR"),
			(1, 0) => Some("GRBG"),
			(0, 1) => Some("GBRG"),
			(1, 1) => Some("RGGB"),
			_ => unreachable!(),
		}
		//}
	}

	/// Uses the [SensorModel] to determine if the image's [ColorType].
	/// If the sensor model is unknown, [SensorModel::Unknown], then [ColorType::Grayscale] is returned
	pub fn color_type(&self) -> ColorType {
		match self.sensor {
			SensorModel::Ar1335 | SensorModel::Ar835 | SensorModel::Imx386 => ColorType::Rgb,
			SensorModel::Ar1335Mono | SensorModel::Imx386Mono | SensorModel::Unknown => {
				ColorType::Grayscale
			}
		}
	}
}

pub enum ColorType {
	Rgb,
	Grayscale,
}

#[derive(Copy, Clone, Debug)]
/// Colour information about the camera. Used to correct the image
pub struct ColorInfo {
	/// Which specific colour this image was taken by
	pub camera: CameraId,

	/// The whitepoint that the forward matrix corresponds to.
	pub whitepoint: Whitepoint,

	/// Camera RGB -> XYZ conversion matrix.
	pub forward_matrix: [f32; 9],

	/// A 3x3 Matrix with unclear usage.
	///
	/// If you know what this is or think you have information, PRs are accepted.
	/// Or emails, if you'd rather. gen@nyble.dev
	pub color_matrix: [f32; 9],

	/// Red-green ratio.
	pub rg: f32,
	/// Blue-green ratio.
	pub bg: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct CameraInfo {
	camera: CameraId,
	sensor: SensorModel,
}
