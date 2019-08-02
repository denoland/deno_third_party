
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use enclose::enclose;

fn main() {
	let mutex_left_data = Arc::new(Mutex::new( 0 ));
	let right_data = Arc::new(1);
	
	let thread = thread::spawn( enclose!((mutex_left_data, right_data) move || {
		let mut lock = match mutex_left_data.lock() {
			Ok(a) => a,
			Err(e) => e.into_inner(),
		};
		*lock += *right_data;
	}));

	thread.join().unwrap();
	{
		let left_data = match mutex_left_data.lock() {
			Ok(a) => a,
			Err(e) => e.into_inner(),
		};
		
		assert_eq!(*left_data, *right_data);
		// if *left_data == *right_data
	}
}

