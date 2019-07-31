

use enclose::enclose;

fn main() {
	let clone_data = 0;
	let add_data = 100;
	my_enclose( enclose!((mut clone_data, add_data) move || {
		println!("#0 {:?}", clone_data);
		clone_data += add_data;
		println!("#1 {:?}", clone_data);
		
		assert_eq!(clone_data, 100);
	}));
	
	assert_eq!(clone_data, 0);
}

fn my_enclose<F: FnOnce() -> R, R>(a: F) -> R {
	a()
}