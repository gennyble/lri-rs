use std::{
	collections::HashMap,
	time::{Duration, Instant},
};

use camino::Utf8PathBuf;
use lri_rs::{DataFormat, HdrMode, LriFile, SceneMode, SensorModel};
use owo_colors::OwoColorize;

fn main() {
	match std::env::args().nth(1).as_deref() {
		Some("gather") => gather(),
		_ => (),
	}
}

fn gather() -> ! {
	let path = std::env::args().nth(2).unwrap();
	let data_dir = Utf8PathBuf::from(path);
	let mut files: HashMap<String, Photo> = HashMap::new();

	for entry in data_dir.read_dir_utf8().unwrap() {
		let entry = entry.unwrap();
		let meta = entry.metadata().unwrap();
		let path = entry.path();

		if meta.is_file() {
			let stub = path.file_stem().unwrap().to_owned();

			match path.extension() {
				Some("jpg") => files
					.entry(stub.clone())
					.and_modify(|e| e.jpg = Some(path.to_owned()))
					.or_insert(Photo::new_jpg(&path)),
				Some("lri") => files
					.entry(stub.clone())
					.and_modify(|e| e.lri = Some(path.to_owned()))
					.or_insert(Photo::new_lri(&path)),
				Some("lris") => files
					.entry(stub.clone())
					.and_modify(|e| e.lris = Some(path.to_owned()))
					.or_insert(Photo::new_lris(&path)),
				None | Some(_) => continue,
			};
		}
	}

	let start = Instant::now();

	let mut photos: Vec<Photo> = files.into_values().collect();
	photos.sort_by(|a, b| a.lri.as_deref().unwrap().cmp(b.lri.as_deref().unwrap()));

	for (idx, photo) in photos.into_iter().enumerate() {
		let lri_path = match photo.lri {
			Some(p) => p,
			None => continue,
		};
		let data = match std::fs::read(&lri_path) {
			Ok(d) => d,
			Err(e) => {
				println!("{}: {}", lri_path.red(), e);
				continue;
			}
		};
		let lri = LriFile::decode(&data);

		print!("{} - ", lri_path.file_stem().unwrap());

		let path = format!("{}_{idx}", lri_path.file_stem().unwrap_or_default());
		let dbg = format!("{:#?}", lri.sig);
		std::fs::write(path, dbg.as_bytes()).unwrap();

		if let Some(fwv) = lri.firmware_version.as_ref() {
			print!(
				"[{}] focal:{:<3} iit:{:>2}ms gain:{:2.0} ",
				fwv,
				lri.focal_length.unwrap(),
				lri.image_integration_time
					.unwrap_or(Duration::ZERO)
					.as_millis(),
				lri.image_gain.unwrap_or_default()
			);

			match lri.hdr {
				None => print!("hdr:{} ", "non".dimmed()),
				Some(HdrMode::None) => print!("hdr:{} ", "nop".dimmed()),
				Some(HdrMode::Default) => print!("hdr:hdr "),
				Some(HdrMode::Natural) => print!("hdr:{} ", "nat".bright_green()),
				Some(HdrMode::Surreal) => print!("hdr:{} ", "sur".bright_magenta()),
			}

			match lri.scene {
				None => print!("sc:{} ", "non".dimmed()),
				Some(SceneMode::None) => print!("sc:{} ", "nop".dimmed()),
				Some(SceneMode::Portrait) => print!("sc:prt "),
				Some(SceneMode::Landscape) => print!("sc:lnd "),
				Some(SceneMode::Macro) => print!("sc:mcr "),
				Some(SceneMode::Sport) => print!("sc:spt "),
				Some(SceneMode::Night) => print!("sc:ni  "),
			}

			match lri.on_tripod {
				None => print!("{} ", "tri".dimmed()),
				Some(false) => print!("{} ", "tri".red()),
				Some(true) => print!("{} ", "tri".green()),
			}

			match lri.af_achieved {
				None => print!("{} - ", "af".dimmed()),
				Some(false) => print!("{} - ", "af".red()),
				Some(true) => print!("{} - ", "af".green()),
			}
		}

		for img in lri.images() {
			let sens = match img.sensor {
				SensorModel::Ar1335 => "a13",
				SensorModel::Ar1335Mono => "a1m",
				SensorModel::Ar835 => "!!!ar8",
				SensorModel::Imx386 => "!!!imx",
				SensorModel::Imx386Mono => "!!!imm",
				SensorModel::Unknown => "???",
			};

			match img.format {
				DataFormat::BayerJpeg => print!("{} ", sens.cyan()),
				DataFormat::Packed10bpp => print!("{} ", sens.yellow()),
			}
		}
		println!("");
	}

	println!("        ---\nTook {:.2}s", start.elapsed().as_secs_f32());

	std::process::exit(0)
}

struct Photo {
	jpg: Option<Utf8PathBuf>,
	lri: Option<Utf8PathBuf>,
	lris: Option<Utf8PathBuf>,
}

impl Photo {
	pub fn new_jpg<P: Into<Utf8PathBuf>>(jpg: P) -> Self {
		Self {
			jpg: Some(jpg.into()),
			lri: None,
			lris: None,
		}
	}
	pub fn new_lri<P: Into<Utf8PathBuf>>(lri: P) -> Self {
		Self {
			lri: Some(lri.into()),
			jpg: None,
			lris: None,
		}
	}
	pub fn new_lris<P: Into<Utf8PathBuf>>(lris: P) -> Self {
		Self {
			lris: Some(lris.into()),
			lri: None,
			jpg: None,
		}
	}
}
