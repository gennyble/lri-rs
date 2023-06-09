#[derive(Debug)]
pub struct Unpacker {
	pub out: Vec<u8>,
	pub work: u16,
	pub work_idx: usize,
}

impl Unpacker {
	pub fn new() -> Self {
		Self {
			out: vec![],
			work: 0,
			work_idx: 0,
		}
	}

	pub fn push(&mut self, byte: u8) {
		self.work = self.work << 8;
		self.work |= byte as u16;
		self.work_idx += 8;

		//println!("[{work_idx}]");

		if self.work_idx >= 10 {
			let to_front = self.work_idx - 10;
			let fronted = self.work >> to_front;
			let masked = fronted & 0b000_000_111_11_111_11;

			let fixwork = fronted << to_front;

			self.out.extend(masked.to_le_bytes());
			self.work_idx -= 10;
			self.work ^= fixwork;
		}
	}

	pub fn finish(&mut self) {
		if self.work_idx > 0 {
			let remain = 10 - self.work_idx;
			let out = self.work << remain;
			self.out.extend(out.to_le_bytes())
		}
	}
}
