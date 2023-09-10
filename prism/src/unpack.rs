const TEN_MASK: u64 = 1023; // ten bits

pub fn tenbit(packd: &[u8], count: usize, upack: &mut [u16]) {
	let required_len_packd = (count as f32 * (10.0 / 8.0)).ceil() as usize;

	println!(
		"requires {required_len_packd} bytes | {} groups of 5",
		count / 4
	);

	if count > upack.len() {
		panic!(
			"expected output buffer to be {count} bytes, got {} bytes",
			upack.len()
		)
	}

	if required_len_packd > packd.len() {
		panic!(
			"expected input to be at least {required_len_packd} bytes, it was {}",
			packd.len()
		)
	}

	let mut packd = packd[..required_len_packd].to_vec();
	packd.reverse();
	let chunker = packd.chunks_exact(5);
	let remain = chunker.remainder();

	for (idx, chnk) in chunker.enumerate() {
		let long = u64::from_be_bytes([
			0x00, 0x00, 0x00, chnk[0], chnk[1], chnk[2], chnk[3], chnk[4],
		]);

		let b4 = long & TEN_MASK;
		let b3 = (long >> 10) & TEN_MASK;
		let b2 = (long >> 20) & TEN_MASK;
		let b1 = (long >> 30) & TEN_MASK;

		let idx = idx * 4;
		upack[idx] = b1 as u16;
		upack[idx + 1] = b2 as u16;
		upack[idx + 2] = b3 as u16;
		upack[idx + 3] = b4 as u16;
	}

	if remain.len() > 0 {
		let mut long_bytes = [0x00; 8];

		for (idx, byte) in remain.iter().enumerate() {
			long_bytes[idx] = *byte;
		}

		let long = u64::from_le_bytes(long_bytes);

		let count_remain = count % 4;
		let start = count - count_remain;
		for idx in 0..count_remain {
			upack[start + idx] = ((long >> (10 * idx)) & TEN_MASK) as u16;
		}
	}
}
