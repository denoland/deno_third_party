# Enclose
A convenient macro for cloning values into a closure.

[![Build Status](https://travis-ci.org/clucompany/Enclose.svg?branch=master)](https://travis-ci.org/clucompany/Enclose)
[![Apache licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![crates.io](http://meritbadge.herokuapp.com/enclose)](https://crates.io/crates/enclose)
[![Documentation](https://docs.rs/enclose/badge.svg)](https://docs.rs/enclose)


# Use
```
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
```

# Use 1

```
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use enclose::enclose;

fn main() {
	let mutex_data = Arc::new(Mutex::new( 0 ));
	let thread = thread::spawn( enclose!((mutex_data => data) move || {
		let mut lock = match data.lock() {
			Ok(a) => a,
			Err(e) => e.into_inner(),
		};
		*lock += 1;
	}));

	thread.join().unwrap();
	{
		let lock = match mutex_data.lock() {
			Ok(a) => a,
			Err(e) => e.into_inner(),
		};
		assert_eq!(*lock, 1);
	}
}
```

# Use 2
```
use std::thread;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use enclose::enclose;


let v = Arc::new(Mutex::new( 0 ));
let v2 = Arc::new(RwLock::new( (0, 2, 3, 4) ));

let count_thread = 5;
let mut wait_all = Vec::with_capacity(count_thread);

for _a in 0..count_thread {
	wait_all.push({
		thread::spawn( enclose!((v, v2) move || {
			let mut v_lock = match v.lock() {
				Ok(a) => a,
				Err(e) => e.into_inner(),
			};
			*v_lock += 1;

			drop( v2 ); //ignore warning
		}))
	});
}
for a in wait_all {
	a.join().unwrap();
}
{	
	//Test result
	let v_lock = match v.lock() {
		Ok(a) => a,
		Err(e) => e.into_inner(),
	};
	assert_eq!(*v_lock, 5);
}
```

# Use 3

```
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use enclose::enclose;

let v = Arc::new(Mutex::new( 0 ));
let thread = thread::spawn( enclose!((v => MY_LOCKER) move || {
	let mut v_lock = match MY_LOCKER.lock() {
		Ok(a) => a,
		Err(e) => e.into_inner(),
	};
	*v_lock += 1;
 }));

thread.join().unwrap();
{
	let v_lock = match v.lock() {
		Ok(a) => a,
		Err(e) => e.into_inner(),
	};
	assert_eq!(*v_lock, 1);
}
```

# License

Copyright 2019 #UlinProject (Denis Kotlyarov) Денис Котляров

Licensed under the MIT License
