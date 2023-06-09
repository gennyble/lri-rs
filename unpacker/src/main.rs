use unpacker::Unpacker;

fn main() {
	// Four bits padding at the end.
	let testdata = vec![
		0b10000000, 0b00010000, 0b00000010, 0b00000000, 0b01000000, 0b00001000, 0b00000001,
		0b00000000, 0b00100000, 0b00000100, 0b00000000, 0b10000000, 0b00010000,
	];

	let mut up = Unpacker {
		out: vec![],
		work: 0,
		work_idx: 0,
	};

	let count = (10.0 as f32 * (10.0 / 8.0)).ceil() as usize;
	for byte in testdata {
		up.push(byte);

		if count == up.out.len() {
			break;
		}
	}
	if count > up.out.len() {
		up.finish();
	}

	for chnk in up.out.chunks(2) {
		println!("{:02b} {:08b}", chnk[1], chnk[0]);
	}
}
