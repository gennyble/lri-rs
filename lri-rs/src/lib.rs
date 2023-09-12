use std::{fmt, time::Duration};

use block::{Block, ExtractedData, Header};
use lri_proto::{
	camera_id::CameraID as PbCameraID, camera_module::camera_module::surface::FormatType,
	color_calibration::color_calibration::IlluminantType,
};

mod block;

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
			if data.len() == 0 {
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

#[derive(Copy, Clone, Debug, PartialEq)]
/// The representation of the raw data in the LRI file
pub enum DataFormat {
	// I'm not sure what this is?? Do we ever see it???
	BayerJpeg,
	Packed10bpp,
	// Never seen
	//Packed12bpp,
	//Packed14bpp,
}

impl fmt::Display for DataFormat {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let str = match self {
			Self::BayerJpeg => "BayerJpeg",
			Self::Packed10bpp => "Packed10bpp",
			//Self::Packed12bpp => "Packed12bpp",
			//Self::Packed14bpp => "Packed14bpp",
		};

		write!(f, "{str}")
	}
}

impl From<FormatType> for DataFormat {
	fn from(proto: FormatType) -> Self {
		match proto {
			FormatType::RAW_BAYER_JPEG => Self::BayerJpeg,
			FormatType::RAW_PACKED_10BPP => Self::Packed10bpp,
			FormatType::RAW_PACKED_12BPP => unreachable!(),
			FormatType::RAW_PACKED_14BPP => unreachable!(),
			FormatType::RAW_RESERVED_0
			| FormatType::RAW_RESERVED_1
			| FormatType::RAW_RESERVED_2
			| FormatType::RAW_RESERVED_3
			| FormatType::RAW_RESERVED_4
			| FormatType::RAW_RESERVED_5 => unimplemented!(),
		}
	}
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CameraId {
	A1,
	A2,
	A3,
	A4,
	A5,
	B1,
	B2,
	B3,
	B4,
	B5,
	C1,
	C2,
	C3,
	C4,
	C5,
	C6,
}

impl From<PbCameraID> for CameraId {
	fn from(pbid: PbCameraID) -> Self {
		match pbid {
			PbCameraID::A1 => Self::A1,
			PbCameraID::A2 => Self::A2,
			PbCameraID::A3 => Self::A3,
			PbCameraID::A4 => Self::A4,
			PbCameraID::A5 => Self::A5,
			PbCameraID::B1 => Self::B1,
			PbCameraID::B2 => Self::B2,
			PbCameraID::B3 => Self::B3,
			PbCameraID::B4 => Self::B4,
			PbCameraID::B5 => Self::B5,
			PbCameraID::C1 => Self::C1,
			PbCameraID::C2 => Self::C2,
			PbCameraID::C3 => Self::C3,
			PbCameraID::C4 => Self::C4,
			PbCameraID::C5 => Self::C5,
			PbCameraID::C6 => Self::C6,
		}
	}
}

impl fmt::Display for CameraId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		// this is good; i write good code
		write!(f, "{self:?}")
	}
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Whitepoint {
	A,
	D50,
	D65,
	D75,
	F2,
	F7,
	F11,
	TL84,
}

impl From<IlluminantType> for Whitepoint {
	fn from(it: IlluminantType) -> Self {
		match it {
			IlluminantType::A => Self::A,
			IlluminantType::D50 => Self::D50,
			IlluminantType::D65 => Self::D65,
			IlluminantType::D75 => Self::D75,
			IlluminantType::F2 => Self::F2,
			IlluminantType::F7 => Self::F7,
			IlluminantType::F11 => Self::F11,
			IlluminantType::TL84 => Self::TL84,
			IlluminantType::UNKNOWN => unimplemented!(),
		}
	}
}

#[derive(Copy, Clone, Debug)]
pub enum SensorModel {
	Unknown,
	Ar835,
	Ar1335,
	Ar1335Mono,
	Imx386,
	Imx386Mono,
}

impl From<lri_proto::sensor_type::SensorType> for SensorModel {
	fn from(pbst: lri_proto::sensor_type::SensorType) -> Self {
		use lri_proto::sensor_type::SensorType as ProtoSt;

		match pbst {
			ProtoSt::SENSOR_UNKNOWN => Self::Unknown,
			ProtoSt::SENSOR_AR835 => Self::Ar835,
			ProtoSt::SENSOR_AR1335 => Self::Ar1335,
			ProtoSt::SENSOR_AR1335_MONO => Self::Ar1335Mono,
			ProtoSt::SENSOR_IMX386 => Self::Imx386,
			ProtoSt::SENSOR_IMX386_MONO => Self::Imx386Mono,
		}
	}
}
