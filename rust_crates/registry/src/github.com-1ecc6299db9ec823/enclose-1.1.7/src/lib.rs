//Copyright (c) 2019 #UlinProject Denis Kotlyarov (Денис Котляров)

//Permission is hereby granted, free of charge, to any person obtaining a copy
//of this software and associated documentation files (the "Software"), to deal
//in the Software without restriction, including without limitation the rights
//to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//copies of the Software, and to permit persons to whom the Software is
//furnished to do so, subject to the following conditions:

//The above copyright notice and this permission notice shall be included in all
//copies or substantial portions of the Software.

//THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//SOFTWARE.

// #Ulin Project 1819

/*!
A convenient macro for cloning values into a closure.

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
	let thread = thread::spawn( enclose!((mutex_data => d) move || {
		let mut lock = match d.lock() {
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

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::thread;

use enclose::enclose;

fn main() {
	let data1 = Arc::new(Mutex::new( 0 ));
	let data2 = Arc::new(RwLock::new( (0, 2, 3, 4) ));

	let count_thread = 5;
	let mut waits = Vec::with_capacity(count_thread);

	for _a in 0..count_thread {
		waits.push({
			thread::spawn( enclose!((data1, data2) move || {
				//(data1, data2) -> 
				//let data1 = 'root.data1.clone();
				//let data2 = 'root.data2.clone();
				
				let mut v_lock = match data1.lock() {
					Ok(a) => a,
					Err(e) => e.into_inner(),
				};
				*v_lock += 1;

				drop( data2 ); //ignore warning
			}))
		});
	}
	for a in waits {
		a.join().unwrap();
	}
	
	
	{	
		//Check data1_lock
		let data1_lock = match data1.lock() {
			Ok(a) => a,
			Err(e) => e.into_inner(),
		};
		assert_eq!(*data1_lock, 5);
	}
	
	{	
		//Check data2_lock
		let data2_lock = match data2.write() {
			Ok(a) => a,
			Err(e) => e.into_inner(),
		};
		assert_eq!(*data2_lock, (0, 2, 3, 4));
	}
}
```

# Use 3

```
use enclose::enclose;
use std::sync::Arc;

fn main() {
	let clone_data = Arc::new(0);
	let add_data = Arc::new(100);
	
	my_enclose( enclose!((mut *clone_data, *add_data) move || {
		println!("#0 {:?}", clone_data);
		clone_data += add_data;
		println!("#1 {:?}", clone_data);
		
		assert_eq!(clone_data, 100);
	}));
	
	assert_eq!(*clone_data, 0);
}

fn my_enclose<F: FnOnce() -> R, R>(a: F) -> R {
	a()
}
```
*/

///Macro for cloning values to close.
#[macro_export]
macro_rules! enclose {
	[( $($tt:tt)* ) $b:expr ] => {{
		$crate::enclose_data! {
			$( $tt )*
		}

		$b
	}};
	
	[() $b: expr] => {$b};
	[$b: expr] => {$b};
}

///Macro for cloning values to close. Alternative short record.
#[macro_export]
macro_rules! enc {
	[$($tt:tt)*] => {
		$crate::enclose!{ $($tt)* }
	};
}


#[doc(hidden)]
#[macro_export]
macro_rules! enclose_data {
	[ *$a: ident => mut $b: ident,  $($tt:tt)*] => {
		let mut $b = *$a;
		
		$crate::enclose_data!{ $($tt)* }
	};
	
	[ $a: ident => mut $b: ident,  $($tt:tt)*] => {
		let mut $b = $a.clone();
		
		$crate::enclose_data!{ $($tt)* }
	};
	
	[ *$a: ident => $b: ident,  $($tt:tt)*] => {
		let $b = *$a;
		
		$crate::enclose_data!{ $($tt)* }
	};
	
	[ $a: ident => $b: ident,  $($tt:tt)*] => {
		let $b = $a.clone();
		
		$crate::enclose_data!{ $($tt)* }
	};
	
	[ mut *$a: ident,  $($tt:tt)*] => {
		let mut $a = *$a;
		
		$crate::enclose_data!{ $($tt)* }
	};
	
	[ mut $a: ident,  $($tt:tt)*] => {
		let mut $a = $a.clone();
		
		$crate::enclose_data!{ $($tt)* }
	};
	
	[ *$a: ident,  $($tt:tt)*] => {
		let $a = *$a;
		
		$crate::enclose_data!{ $($tt)* }
	};
	
	[ $a: ident,  $($tt:tt)*] => {
		let $a = $a.clone();
		
		$crate::enclose_data!{ $($tt)* }
	};
	
	
	
	//NO ,!
	[ *$a: ident => mut $b: ident] => {
		let mut $b = *$a;
	};
	
	[ $a: ident => mut $b: ident] => {
		let mut $b = $a.clone();
	};
	
	[ *$a: ident => $b: ident] => {
		let $b = *$a;
	};
	
	[ $a: ident => $b: ident] => {
		let $b = $a.clone();
	};
	
	[ mut *$a: ident] => {
		let $a = *$a;
	};
	
	[ mut $a: ident] => {
		let mut $a = $a.clone();
	};
	
	[ *$a: ident] => {
		let $a = *$a;
	};
	
	[ $a: ident] => {
		let $a = $a.clone();
	};
	
	
	() => ()
}




#[cfg(test)]
mod tests {
	use std::thread;
	use std::sync::Arc;
	use std::sync::Mutex;
	use std::sync::MutexGuard;
	use std::sync::RwLock;
	
	
	struct MutexSafeData(Mutex<usize>);
		
	impl MutexSafeData {
		#[inline]
		pub fn new(def: usize) -> Self {
			MutexSafeData(Mutex::new(def))	
		}
		pub fn set(&self, size: usize) {
			*self.get_mut() = size;
		}
		pub fn get_mut<'a>(&'a self) -> MutexGuard<'a, usize> {
			match self.0.lock() {
				Ok(a) => a,
				Err(e) => e.into_inner(),
			}
		}
	}
	
	
	#[test]
	fn easy() {
		let mutex_data = Arc::new(MutexSafeData::new(0));
		let thread = thread::spawn( enclose!((mutex_data) move || { //NEW THREAD, NEW ARC!
			//let data = data.clone();
			
			mutex_data.set(10);
		}));

		thread.join().unwrap();
		assert_eq!(*mutex_data.get_mut(), 10);
	}
	#[test]
	fn easy_extract() {
		let mutex_data = Arc::new(MutexSafeData::new(0));
		let thread = thread::spawn( enclose!((mutex_data => new_data) move || { //NEW THREAD, NEW ARC!
			//let data = data.clone();
			
			new_data.set(10);
		}));

		thread.join().unwrap();
		assert_eq!(*mutex_data.get_mut(), 10);
	}

	#[test]
	fn easy_2name() {
		let safe_data = Arc::new(MutexSafeData::new(0));
		let v2 = Arc::new(RwLock::new( (0, 2, 3, 4) ));

		let count_thread = 5;
		let mut join_all = Vec::with_capacity(count_thread);

		for _a in 0..count_thread {
			join_all.push({
				thread::spawn( enclose!((safe_data, v2) move || {
					*safe_data.get_mut() += 1;

					drop( v2 ); //ignore warning
				}))
			});
		}
		for a in join_all {
			a.join().unwrap();
		}
		assert_eq!(*safe_data.get_mut(), 5);
	}
	
	#[test]
	fn clone_mut_data() {
		let data = 10;
		
		enclose!((data => mut new_data) move || {
			//let mut new_data = data;
			new_data += 1;

			assert_eq!(new_data, 11);
		});
		
		assert_eq!(data, 10);
	}

}
