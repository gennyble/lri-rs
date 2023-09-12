pub fn rotate_180<T: Copy>(data: &mut [T]) {
	let mut rat = vec![data[0]; data.len()];

	for (idx, px) in data.chunks(3).rev().enumerate() {
		rat[idx * 3] = px[0];
		rat[idx * 3 + 1] = px[1];
		rat[idx * 3 + 2] = px[2];
	}

	data.copy_from_slice(&rat);
}
