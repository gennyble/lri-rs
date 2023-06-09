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

	for byte in testdata {
		up.push(byte);
	}
	up.finish();

	for chnk in up.out.chunks(2) {
		println!("{:08b} {:08b}", chnk[0], chnk[1]);
	}
}
