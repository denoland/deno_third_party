/*
 *
 * Copyright 2018 Google Inc. All rights reserved.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::collections::HashMap;

extern crate quickcheck;

extern crate flatbuffers;
extern crate rust_usage_test;
use rust_usage_test::monster_test_generated::my_game;
//use rust_usage_test::namespace_test::NamespaceA;

//pub use my_game::Example;

//mod my_game;
//#include "flatbuffers/flatbuffers.h"
//#include "flatbuffers/idl.h"
//#include "flatbuffers/minireflect.h"
//#include "flatbuffers/registry.h"
//#include "flatbuffers/util.h"
//
//#include "monster_test_generated.h"
//#include "namespace_test/namespace_test1_generated.h"
//#include "namespace_test/namespace_test2_generated.h"
//#include "union_vector/union_vector_generated.h"

// Include simple random number generator to ensure results will be the
// same cross platform.
// http://en.wikipedia.org/wiki/Park%E2%80%93Miller_random_number_generator
struct LCG(u64);
impl LCG {
    fn new() -> Self {
        LCG { 0: 48271 }
    }
    fn next(&mut self) -> u64 {
        let old = self.0;
        self.0 = (self.0 * 279470273u64) % 4294967291u64;
        old
    }
    fn reset(&mut self) {
        self.0 = 48271
    }
}

fn create_serialized_example_with_generated_code(builder: &mut flatbuffers::FlatBufferBuilder) {
    let mon = {
        let fred_name = builder.create_string("Fred");
        let inventory = builder.create_vector_of_scalars::<u8>(&vec![0, 1, 2, 3, 4][..]);
        let test4 = builder.create_vector_of_structs(&vec![my_game::example::Test::new(10, 20),
                                                           my_game::example::Test::new(30, 40)][..]);
        let pos = my_game::example::Vec3::new(1.0, 2.0, 3.0, 3.0, my_game::example::Color::Green, my_game::example::Test::new(5i16, 6i8));
        let args = my_game::example::MonsterArgs{
            hp: 80,
            mana: 150,
            name: Some(builder.create_string("MyMonster")),
            pos: Some(&pos),
            test_type: my_game::example::Any::Monster,
            // TODO(rw): better offset ergonomics
            test: Some(my_game::example::Monster::create(builder, &my_game::example::MonsterArgs{
                name: Some(fred_name),
                ..Default::default()
            }).as_union_value()),
            inventory: Some(inventory),
            test4: Some(test4),
            testarrayofstring: Some(builder.create_vector_of_strings(&["test1", "test2"])),
            ..Default::default()
        };
        my_game::example::Monster::create(builder, &args)
    };
    my_game::example::finish_monster_buffer(builder, mon);
}
fn create_serialized_example_with_library_code(builder: &mut flatbuffers::FlatBufferBuilder) {
    let nested_union_mon = {
        let name = builder.create_string("Fred");
        let table_start = builder.start_table(34);
        builder.push_slot_offset_relative(my_game::example::Monster::VT_NAME, name);
        builder.end_table(table_start)
    };
    let pos = my_game::example::Vec3::new(1.0, 2.0, 3.0, 3.0, my_game::example::Color::Green, my_game::example::Test::new(5i16, 6i8));
    let inv = builder.create_vector_of_scalars::<u8>(&vec![0, 1, 2, 3, 4]);

    let test4 = builder.create_vector_of_structs(&vec![my_game::example::Test::new(10, 20),
                                                       my_game::example::Test::new(30, 40)][..]);

    let name = builder.create_string("MyMonster");
    let testarrayofstring = builder.create_vector_of_strings(&["test1", "test2"][..]);

    // begin building

    let table_start = builder.start_table(34);
    builder.push_slot_scalar::<i16>(my_game::example::Monster::VT_HP, 80, 100);
    builder.push_slot_offset_relative::<&str>(my_game::example::Monster::VT_NAME, name);
    builder.push_slot_struct(my_game::example::Monster::VT_POS, &pos);
    builder.push_slot_scalar::<u8>(my_game::example::Monster::VT_TEST_TYPE, my_game::example::Any::Monster as u8, 0);
    builder.push_slot_offset_relative(my_game::example::Monster::VT_TEST, nested_union_mon);
    builder.push_slot_offset_relative(my_game::example::Monster::VT_INVENTORY, inv);
    builder.push_slot_offset_relative(my_game::example::Monster::VT_TEST4, test4);
    builder.push_slot_offset_relative(my_game::example::Monster::VT_TESTARRAYOFSTRING, testarrayofstring);
    let root = builder.end_table(table_start);
    builder.finish(root, Some(my_game::example::MONSTER_IDENTIFIER));
}

fn serialized_example_is_accessible_and_correct(bytes: &[u8], identifier_required: bool, size_prefixed: bool) -> Result<(), &'static str> {
    if identifier_required {
        let correct = if size_prefixed {
            my_game::example::monster_size_prefixed_buffer_has_identifier(bytes)
        } else {
            my_game::example::monster_buffer_has_identifier(bytes)
        };
        if !correct {
            return Err("incorrect buffer identifier");
        }
    }
    let monster1 = if size_prefixed {
        my_game::example::get_size_prefixed_root_as_monster(bytes)
    } else {
        my_game::example::get_root_as_monster(bytes)
    };
    for m in vec![monster1] {
        if m.hp() != 80 { assert_eq!(80, m.hp()); return Err("bad m.hp"); }
        if m.mana() != 150 { return Err("bad m.mana"); }
        match m.name() {
            Some("MyMonster") => { }
            _ => { return Err("bad m.name"); }
        }
        let pos = match m.pos() {
            None => { return Err("bad m.pos"); }
            Some(x) => { x }
        };
        if pos as *const my_game::example::Vec3 as usize % 16 != 0 {
            return Err("bad Vec3 alignment");
        }
        if pos.x() != 1.0f32 { return Err("bad pos.x"); }
        if pos.y() != 2.0f32 { return Err("bad pos.y"); }
        if pos.z() != 3.0f32 { return Err("bad pos.z"); }
        if pos.test1() != 3.0f64 { return Err("bad pos.test1"); }
        if pos.test2() != my_game::example::Color::Green { return Err("bad pos.test2"); }

        let pos_test3 = pos.test3();
        if pos_test3.a() != 5i16 { return Err("bad pos_test3.a"); }
        if pos_test3.b() != 6i8 { return Err("bad pos_test3.b"); }

        match m.enemy() {
            None => {
                println!("missing m.enemy, most language ports do not generate this yet");
            }
            Some(e) => {
                match e.name() {
                    Some("Fred") => { /* woot */ }
                    _ => { println!("missing m.enemy.name, most language ports do not generate this yet") }
                }
            }
        }

        if m.test_type() != my_game::example::Any::Monster { return Err("bad m.test_type"); }

        let table2 = match m.test() {
            None => { return Err("bad m.test"); }
            Some(x) => { x }
        };

        let monster2 = my_game::example::Monster::init_from_table(table2);

        match monster2.name() {
            Some("Fred") => { }
            _ => { return Err("bad monster2.name"); }
        }

        let inv: &[u8] = match m.inventory() {
            None => { return Err("bad m.inventory"); }
            Some(x) => { x }
        };
        if inv.len() != 5 {  return Err("bad m.inventory len"); }
        let invsum: u8 = inv.iter().sum();
        if invsum != 10 { return Err("bad m.inventory sum"); }

        {
            let test4 = match m.test4() {
                None => { return Err("bad m.test4"); }
                Some(x) => { x }
            };
            if test4.len() != 2 { return Err("bad m.test4 len"); }

            let x = test4[0];
            let y = test4[1];
            let xy_sum = x.a() as i32 + x.b() as i32 + y.a() as i32 + y.b() as i32;
            if xy_sum != 100 { return Err("bad m.test4 item sum"); }
        }

        {
            match m.testarrayoftables() {
                None => { println!("not all monster examples have testarrayoftables, skipping"); }
                Some(x) => {
                    println!("foo: {:?}", x.get(0).name());
                    if x.get(0).name() != Some("Barney") { return Err("bad testarrayoftables.get(0).name()") }
                    if x.get(1).name() != Some("Frodo") { return Err("bad testarrayoftables.get(1).name()") }
                    if x.get(2).name() != Some("Wilma") { return Err("bad testarrayoftables.get(2).name()") }
                }
            }
        }

        let testarrayofstring = match m.testarrayofstring() {
            None => { return Err("bad m.testarrayofstring"); }
            Some(x) => { x }
        };
        if testarrayofstring.len() != 2 { return Err("bad monster.testarrayofstring len"); }
        if testarrayofstring.get(0) != "test1" { return Err("bad monster.testarrayofstring.get(0)"); }
        if testarrayofstring.get(1) != "test2" { return Err("bad monster.testarrayofstring.get(1)"); }
    }
    Ok(())
}

#[cfg(test)]
mod generated_constants {
    extern crate flatbuffers;
    use super::my_game;

    #[test]
    fn monster_identifier() {
        assert_eq!("MONS", my_game::example::MONSTER_IDENTIFIER);
    }

    #[test]
    fn monster_file_extension() {
        assert_eq!("mon", my_game::example::MONSTER_EXTENSION);
    }
}

#[cfg(test)]
mod roundtrips_with_generated_code {
    extern crate flatbuffers;

    extern crate rust_usage_test;
    use rust_usage_test::monster_test_generated::my_game;

    fn build_mon<'a, 'b>(builder: &'a mut flatbuffers::FlatBufferBuilder, args: &'b my_game::example::MonsterArgs) -> my_game::example::Monster<'a> {
        let mon = my_game::example::Monster::create(builder, &args);
        my_game::example::finish_monster_buffer(builder, mon);
        my_game::example::get_root_as_monster(builder.finished_bytes())
    }

    #[test]
    fn scalar_store() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let m = build_mon(&mut b, &my_game::example::MonsterArgs{hp: 123, ..Default::default()});
        assert_eq!(m.hp(), 123);
    }
    #[test]
    fn scalar_default() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let m = build_mon(&mut b, &my_game::example::MonsterArgs{..Default::default()});
        assert_eq!(m.hp(), 100);
    }
    #[test]
    fn string_store() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let name = b.create_string("foobar");
        let m = build_mon(&mut b, &my_game::example::MonsterArgs{name: Some(name), ..Default::default()});
        assert_eq!(m.name(), Some("foobar"));
    }
    #[test]
    fn struct_store() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let name = b.create_string("foo");
        let m = build_mon(&mut b, &my_game::example::MonsterArgs{
            name: Some(name),
            pos: Some(&my_game::example::Vec3::new(1.0, 2.0, 3.0, 4.0,
                                                   my_game::example::Color::Green,
                                                   my_game::example::Test::new(98, 99))),
            ..Default::default()
        });
        assert_eq!(m.pos(), Some(&my_game::example::Vec3::new(1.0, 2.0, 3.0, 4.0,
                                                              my_game::example::Color::Green,
                                                              my_game::example::Test::new(98, 99))));
    }
    #[test]
    fn struct_default() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let name = b.create_string("foo");
        let m = build_mon(&mut b, &my_game::example::MonsterArgs{name: Some(name), ..Default::default()});
        assert_eq!(m.pos(), None);
    }
    #[test]
    fn enum_store() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let m = build_mon(&mut b, &my_game::example::MonsterArgs{color: my_game::example::Color::Red, ..Default::default()});
        assert_eq!(m.color(), my_game::example::Color::Red);
    }
    #[test]
    fn enum_default() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let m = build_mon(&mut b, &my_game::example::MonsterArgs{..Default::default()});
        assert_eq!(m.color(), my_game::example::Color::Blue);
    }
    #[test]
    fn union_store() {
        let b = &mut flatbuffers::FlatBufferBuilder::new();
        {
            let name_inner = b.create_string("foo");
            let name_outer = b.create_string("bar");

            let inner = my_game::example::Monster::create(b, &my_game::example::MonsterArgs{
                name: Some(name_inner),
                ..Default::default()
            });
            let outer = my_game::example::Monster::create(b, &my_game::example::MonsterArgs{
                name: Some(name_outer),
                test_type: my_game::example::Any::Monster,
                test: Some(inner.as_union_value()),
                ..Default::default()
            });
            my_game::example::finish_monster_buffer(b, outer);
        }

        let mon = my_game::example::get_root_as_monster(b.finished_bytes());
        assert_eq!(mon.name(), Some("bar"));
        assert_eq!(mon.test_type(), my_game::example::Any::Monster);
        assert_eq!(my_game::example::Monster::init_from_table(mon.test().unwrap()).name(),
                   Some("foo"));
    }
    #[test]
    fn union_default() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let name = b.create_string("foo");
        let m = build_mon(&mut b, &my_game::example::MonsterArgs{name: Some(name), ..Default::default()});
        assert_eq!(m.test_type(), my_game::example::Any::NONE);
        assert_eq!(m.test(), None);
    }
    #[test]
    fn table_full_namespace_store() {
        let b = &mut flatbuffers::FlatBufferBuilder::new();
        {
            let name_inner = b.create_string("foo");
            let name_outer = b.create_string("bar");

            let inner = my_game::example::Monster::create(b, &my_game::example::MonsterArgs{
                name: Some(name_inner),
                ..Default::default()
            });
            let outer = my_game::example::Monster::create(b, &my_game::example::MonsterArgs{
                name: Some(name_outer),
                enemy: Some(inner),
                ..Default::default()
            });
            my_game::example::finish_monster_buffer(b, outer);
        }

        let mon = my_game::example::get_root_as_monster(b.finished_bytes());
        assert_eq!(mon.name(), Some("bar"));
        assert_eq!(mon.enemy().unwrap().name(), Some("foo"));
    }
    #[test]
    fn table_full_namespace_default() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let name = b.create_string("foo");
        let m = build_mon(&mut b, &my_game::example::MonsterArgs{name: Some(name), ..Default::default()});
        assert_eq!(m.enemy(), None);
    }
    #[test]
    fn table_store() {
        let b = &mut flatbuffers::FlatBufferBuilder::new();
        {
            let id_inner = b.create_string("foo");
            let name_outer = b.create_string("bar");

            let inner = my_game::example::Stat::create(b, &my_game::example::StatArgs{
                id: Some(id_inner),
                ..Default::default()
            });
            let outer = my_game::example::Monster::create(b, &my_game::example::MonsterArgs{
                name: Some(name_outer),
                testempty: Some(inner),
                ..Default::default()
            });
            my_game::example::finish_monster_buffer(b, outer);
        }

        let mon = my_game::example::get_root_as_monster(b.finished_bytes());
        assert_eq!(mon.name(), Some("bar"));
        assert_eq!(mon.testempty().unwrap().id(), Some("foo"));
    }
    #[test]
    fn table_default() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let name = b.create_string("foo");
        let m = build_mon(&mut b, &my_game::example::MonsterArgs{name: Some(name), ..Default::default()});
        assert_eq!(m.testempty(), None);
    }
    #[test]
    fn nested_flatbuffer_store() {
        let b0 = {
            let mut b0 = flatbuffers::FlatBufferBuilder::new();
            let args = my_game::example::MonsterArgs{
                hp: 123,
                name: Some(b0.create_string("foobar")),
                ..Default::default()
            };
            let mon = my_game::example::Monster::create(&mut b0, &args);
            my_game::example::finish_monster_buffer(&mut b0, mon);
            b0
        };

        let b1 = {
            let mut b1 = flatbuffers::FlatBufferBuilder::new();
            let args = my_game::example::MonsterArgs{
                testnestedflatbuffer: Some(b1.create_vector_of_scalars::<u8>(b0.finished_bytes())),
                ..Default::default()
            };
            let mon = my_game::example::Monster::create(&mut b1, &args);
            my_game::example::finish_monster_buffer(&mut b1, mon);
            b1
        };

        let m = my_game::example::get_root_as_monster(b1.finished_bytes());

        assert!(m.testnestedflatbuffer().is_some());
        assert_eq!(m.testnestedflatbuffer().unwrap(), b0.finished_bytes());

        let m2_a = my_game::example::get_root_as_monster(m.testnestedflatbuffer().unwrap());
        assert_eq!(m2_a.hp(), 123);
        assert_eq!(m2_a.name(), Some("foobar"));

        assert!(m.testnestedflatbuffer_nested_flatbuffer().is_some());
        let m2_b = m.testnestedflatbuffer_nested_flatbuffer().unwrap();

        assert_eq!(m2_b.hp(), 123);
        assert_eq!(m2_b.name(), Some("foobar"));
    }
    #[test]
    fn nested_flatbuffer_default() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let name = b.create_string("foo");
        let m = build_mon(&mut b, &my_game::example::MonsterArgs{name: Some(name), ..Default::default()});
        assert_eq!(m.testnestedflatbuffer(), None);
    }
    #[test]
    fn vector_of_string_store_auto() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let v = b.create_vector_of_strings(&["foobar", "baz"]);
        let m = build_mon(&mut b, &my_game::example::MonsterArgs{testarrayofstring: Some(v), ..Default::default()});
        assert_eq!(m.testarrayofstring().unwrap().len(), 2);
        assert_eq!(m.testarrayofstring().unwrap().get(0), "foobar");
        assert_eq!(m.testarrayofstring().unwrap().get(1), "baz");
    }
    #[test]
    fn vector_of_string_store_manual_a() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let s0 = b.create_string("foobar");
        let s1 = b.create_string("baz");
        let v = b.create_vector_of_reverse_offsets(&[s0, s1]);
        let m = build_mon(&mut b, &my_game::example::MonsterArgs{testarrayofstring: Some(v), ..Default::default()});
        assert_eq!(m.testarrayofstring().unwrap().len(), 2);
        assert_eq!(m.testarrayofstring().unwrap().get(0), "foobar");
        assert_eq!(m.testarrayofstring().unwrap().get(1), "baz");
    }
    #[test]
    fn vector_of_ubyte_store() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let v = b.create_vector_of_scalars::<u8>(&[123, 234][..]);
        let m = build_mon(&mut b, &my_game::example::MonsterArgs{inventory: Some(v), ..Default::default()});
        assert_eq!(m.inventory().unwrap(), &[123, 234][..]);
    }
    #[test]
    fn vector_of_bool_store() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let v = b.create_vector_of_scalars::<bool>(&[false, true, false, true][..]);
        let m = build_mon(&mut b, &my_game::example::MonsterArgs{testarrayofbools: Some(v), ..Default::default()});
        assert_eq!(m.testarrayofbools().unwrap(), &[false, true, false, true][..]);
    }
    #[test]
    fn vector_of_f64_store() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let v = b.create_vector_of_scalars::<f64>(&[3.14159265359][..]);
        let m = build_mon(&mut b, &my_game::example::MonsterArgs{vector_of_doubles: Some(v), ..Default::default()});
        assert_eq!(m.vector_of_doubles().unwrap(), &[3.14159265359][..]);
    }
    #[test]
    fn vector_of_struct_store() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let v = b.create_vector_of_structs::<my_game::example::Test>(&[my_game::example::Test::new(127, -128), my_game::example::Test::new(3, 123)][..]);
        let m = build_mon(&mut b, &my_game::example::MonsterArgs{test4: Some(v), ..Default::default()});
        assert_eq!(m.test4().unwrap(), &[my_game::example::Test::new(127, -128), my_game::example::Test::new(3, 123)][..]);
    }
    #[test]
    fn vector_of_table_store() {
        let b = &mut flatbuffers::FlatBufferBuilder::new();
        let t0 = {
            let name = b.create_string("foo");
            let args = my_game::example::MonsterArgs{hp: 55, name: Some(name), ..Default::default()};
            my_game::example::Monster::create(b, &args)
        };
        let t1 = {
            let name = b.create_string("bar");
            let args = my_game::example::MonsterArgs{name: Some(name), ..Default::default()};
            my_game::example::Monster::create(b, &args)
        };
        let v = b.create_vector_of_reverse_offsets::<my_game::example::Monster>(&[t0, t1][..]);
        let m = build_mon(b, &my_game::example::MonsterArgs{testarrayoftables: Some(v), ..Default::default()});
        assert_eq!(m.testarrayoftables().unwrap().len(), 2);
        assert_eq!(m.testarrayoftables().unwrap().get(0).hp(), 55);
        assert_eq!(m.testarrayoftables().unwrap().get(0).name(), Some("foo"));
        assert_eq!(m.testarrayoftables().unwrap().get(1).hp(), 100);
        assert_eq!(m.testarrayoftables().unwrap().get(1).name(), Some("bar"));
    }
}

#[cfg(test)]
mod alignment_and_padding {
    use super::my_game;
    #[test]
    fn vec3_is_padded_to_mod_16() {
        assert_eq!(::std::mem::size_of::<my_game::example::Vec3>() % 16, 0);
    }
}

#[cfg(test)]
mod vector_read_scalar_tests {
    extern crate quickcheck;
    extern crate flatbuffers;

    fn prop<T: PartialEq + ::std::fmt::Debug + Copy + flatbuffers::ElementScalar>(xs: Vec<T>) {
        use flatbuffers::Follow;

        let mut b = flatbuffers::FlatBufferBuilder::new();
        b.start_vector(xs.len(), ::std::mem::size_of::<T>());
        for i in (0..xs.len()).rev() {
            b.push_element_scalar::<T>(xs[i]);
        }
        let vecend = b.end_vector::<T>(xs.len());
        b.finish_minimal(vecend);

        let buf = b.finished_bytes();

        let got = <flatbuffers::ForwardsU32Offset<&[T]>>::follow(buf, 0);
        assert_eq!(got, &xs[..]);
    }

    #[test]
    fn easy() {
        prop::<u8>(vec![]);
        prop::<u8>(vec![1u8]);
        prop::<u8>(vec![1u8, 2u8]);
        prop::<u8>(vec![1u8, 2u8, 3u8]);
        prop::<u8>(vec![1u8, 2u8, 3u8, 4u8]);
    }

    #[test]
    fn fuzz() {
        let n = 20;
        quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop::<u8> as fn(Vec<_>));
        quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop::<i8> as fn(Vec<_>));
        quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop::<u16> as fn(Vec<_>));
        quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop::<i16> as fn(Vec<_>));
        quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop::<u32> as fn(Vec<_>));
        quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop::<i32> as fn(Vec<_>));
        quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop::<u64> as fn(Vec<_>));
        quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop::<i64> as fn(Vec<_>));
        quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop::<f32> as fn(Vec<_>));
        quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop::<f64> as fn(Vec<_>));
        quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop::<bool> as fn(Vec<_>));
    }
}

#[cfg(test)]
mod vector_read_obj_tests {
    extern crate quickcheck;
    extern crate flatbuffers;

    fn prop_strings(xs: Vec<String>) {
        use flatbuffers::Follow;

        let mut b = flatbuffers::FlatBufferBuilder::new();
        let mut offsets = Vec::new();
        for s in xs.iter().rev() {
            offsets.push(b.create_string(s.as_str()));
        }

        b.start_vector(flatbuffers::SIZE_UOFFSET, xs.len());
        for &i in offsets.iter() {
            b.push_element_scalar_indirect_uoffset(i.value());
        }
        let vecend = b.end_vector::<flatbuffers::Offset<&str>>(xs.len());

        b.finish_minimal(vecend);

        let buf = b.finished_bytes();
        let got = <flatbuffers::ForwardsU32Offset<flatbuffers::Vector<flatbuffers::ForwardsU32Offset<&str>>>>::follow(buf, 0);

        assert_eq!(got.len(), xs.len());
        for i in 0..xs.len() {
            assert_eq!(got.get(i), &xs[i][..]);
        }
    }

    #[test]
    fn fuzz() {
        let n = 20;
        quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop_strings as fn(Vec<_>));
    }
}

// Prefix a FlatBuffer with a size field.
#[test]
fn test_size_prefixed_buffer() {
    // Create size prefixed buffer.
    let mut b = flatbuffers::FlatBufferBuilder::new();
    let args = &my_game::example::MonsterArgs{
        mana: 200,
        hp: 300,
        name: Some(b.create_string("bob")),
        ..Default::default()
    };
    let mon = my_game::example::Monster::create(&mut b, &args);
    b.finish_size_prefixed(mon, None);

    // Access it.
    let buf = b.finished_bytes();
    let m = flatbuffers::get_size_prefixed_root::<my_game::example::Monster>(buf);
    assert_eq!(m.mana(), 200);
    assert_eq!(m.hp(), 300);
    assert_eq!(m.name(), Some("bob"));
}

#[test]
fn fuzz_scalar_table_serialization() {
    // Values we're testing against: chosen to ensure no bits get chopped
    // off anywhere, and also be different from eachother.
    let bool_val: bool = true;
    let char_val: i8 = -127;  // 0x81
    let uchar_val: u8 = 0xFF;
    let short_val: i16 = -32222;  // 0x8222;
    let ushort_val: u16 = 0xFEEE;
    let int_val: i32 = unsafe { std::mem::transmute(0x83333333u32) };
    let uint_val: u32 = 0xFDDDDDDD;
    let long_val: i64 = unsafe { std::mem::transmute(0x8444444444444444u64) }; // TODO: byte literal?
    let ulong_val: u64 = 0xFCCCCCCCCCCCCCCCu64;
    let float_val: f32 = 3.14159;
    let double_val: f64 = 3.14159265359;

    let test_value_types_max: isize = 11;
    let max_fields_per_object: flatbuffers::VOffsetT = 100;
    let num_fuzz_objects: isize = 1000;  // The higher, the more thorough :)

    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let mut lcg = LCG::new();

    let mut objects: Vec<flatbuffers::UOffsetT> = vec![0; num_fuzz_objects as usize];

    // Generate num_fuzz_objects random objects each consisting of
    // fields_per_object fields, each of a random type.
    for i in 0..(num_fuzz_objects as usize) {
        let fields_per_object = (lcg.next() % (max_fields_per_object as u64)) as flatbuffers::VOffsetT;
        let start = builder.start_table(fields_per_object);

        for j in 0..fields_per_object {
            let choice = lcg.next() % (test_value_types_max as u64);

            let f = flatbuffers::field_index_to_field_offset(j);

            match choice {
                0 => {builder.push_slot_scalar::<bool>(f, bool_val, false);}
                1 => {builder.push_slot_scalar::<i8>(f, char_val, 0);}
                2 => {builder.push_slot_scalar::<u8>(f, uchar_val, 0);}
                3 => {builder.push_slot_scalar::<i16>(f, short_val, 0);}
                4 => {builder.push_slot_scalar::<u16>(f, ushort_val, 0);}
                5 => {builder.push_slot_scalar::<i32>(f, int_val, 0);}
                6 => {builder.push_slot_scalar::<u32>(f, uint_val, 0);}
                7 => {builder.push_slot_scalar::<i64>(f, long_val, 0);}
                8 => {builder.push_slot_scalar::<u64>(f, ulong_val, 0);}
                9 => {builder.push_slot_scalar::<f32>(f, float_val, 0.0);}
                10 => {builder.push_slot_scalar::<f64>(f, double_val, 0.0);}
                _ => { panic!("unknown choice: {}", choice); }
            }
        }
        objects[i] = builder.end_table(start).value();
    }

    // Do some bookkeeping to generate stats on fuzzes:
    let mut stats: HashMap<u64, u64> = HashMap::new();
    let mut values_generated: u64 = 0;

    // Embrace PRNG determinism:
    lcg.reset();

    // Test that all objects we generated are readable and return the
    // expected values. We generate random objects in the same order
    // so this is deterministic:
    for i in 0..(num_fuzz_objects as usize) {
        let table = {
            let buf = builder.get_active_buf_slice();
            let loc = buf.len() as flatbuffers::UOffsetT - objects[i];
            flatbuffers::Table::new(buf, loc as usize)
        };

        let fields_per_object = (lcg.next() % (max_fields_per_object as u64)) as flatbuffers::VOffsetT;
        for j in 0..fields_per_object {
            let choice = lcg.next() % (test_value_types_max as u64);

            *stats.entry(choice).or_insert(0) += 1;
            values_generated += 1;

            let f = flatbuffers::field_index_to_field_offset(j);

            match choice {
                0 => { assert_eq!(bool_val, table.get::<bool>(f, Some(false)).unwrap()); }
                1 => { assert_eq!(char_val, table.get::<i8>(f, Some(0)).unwrap()); }
                2 => { assert_eq!(uchar_val, table.get::<u8>(f, Some(0)).unwrap()); }
                3 => { assert_eq!(short_val, table.get::<i16>(f, Some(0)).unwrap()); }
                4 => { assert_eq!(ushort_val, table.get::<u16>(f, Some(0)).unwrap()); }
                5 => { assert_eq!(int_val, table.get::<i32>(f, Some(0)).unwrap()); }
                6 => { assert_eq!(uint_val, table.get::<u32>(f, Some(0)).unwrap()); }
                7 => { assert_eq!(long_val, table.get::<i64>(f, Some(0)).unwrap()); }
                8 => { assert_eq!(ulong_val, table.get::<u64>(f, Some(0)).unwrap()); }
                9 => { assert_eq!(float_val, table.get::<f32>(f, Some(0.0)).unwrap()); }
                10 => { assert_eq!(double_val, table.get::<f64>(f, Some(0.0)).unwrap()); }
                _ => { panic!("unknown choice: {}", choice); }
            }
        }
    }

    // Assert that we tested all the fuzz cases enough:
    let min_tests_per_choice = 1000;
    assert!(values_generated > 0);
    assert!(min_tests_per_choice > 0);
    for i in 0..test_value_types_max as u64 {
        assert!(stats[&i] >= min_tests_per_choice,
                format!("inadequately-tested fuzz case: {}", i));
    }
}

//void EndianSwapTest() {
//  TEST_EQ(flatbuffers::EndianSwap(static_cast<int16_t>(0x1234)), 0x3412);
//  TEST_EQ(flatbuffers::EndianSwap(static_cast<int32_t>(0x12345678)),
//          0x78563412);
//  TEST_EQ(flatbuffers::EndianSwap(static_cast<int64_t>(0x1234567890ABCDEF)),
//          0xEFCDAB9078563412);
//  TEST_EQ(flatbuffers::EndianSwap(flatbuffers::EndianSwap(3.14f)), 3.14f);
//}

#[test]
fn test_emplace_and_read_scalar_fuzz() {
    // TODO(rw): random generate values, probably with a macro
    // because num traits are annoying.
        for n in u8::min_value()..=u8::max_value() {
            let mut buf = vec![0u8; 1];
            flatbuffers::emplace_scalar(&mut buf[..], n);
            let m = flatbuffers::read_scalar(&buf[..]);
            assert_eq!(n, m);
        }
        for n in i8::min_value()..=i8::max_value() {
            let mut buf = vec![0u8; 1];
            flatbuffers::emplace_scalar(&mut buf[..], n);
            let m = flatbuffers::read_scalar(&buf[..]);
            assert_eq!(n, m);
        }
        for n in u16::min_value()..=u16::max_value() {
            let mut buf = vec![0u8; 2];
            flatbuffers::emplace_scalar(&mut buf[..], n);
            let m = flatbuffers::read_scalar(&buf[..]);
            assert_eq!(n, m);
        }
        for n in i16::min_value()..=i16::max_value() {
            let mut buf = vec![0u8; 2];
            flatbuffers::emplace_scalar(&mut buf[..], n);
            let m = flatbuffers::read_scalar(&buf[..]);
            assert_eq!(n, m);
        }

    //fn doit<T: flatbuffers::ElementScalar>() {
    //    let mut lcg = LCG::new();
    //    //let mut rng = rand::thread_rng();

    //    for i in 0..1000 {
    //        let sz = std::mem::size_of::<T>();
    //        let n = T::From(i);
    //        //let x = lcg.next();
    //        //let mut xx = vec![0u8; sz];
    //        //for i in 0..sz {
    //        //    xx[i] = x as u8;
    //        //    x = x >> 8;
    //        //}
    //        //let n: T = unsafe {
    //        //    std::mem::transmute(xx.as_ptr())
    //        //};
    //        //let n = (lcg.next() % std::mem::size_of::<T>()) as T;
    //        //let n = rng.gen::<T>();
    //        let mut buf = vec![0u8; sz];
    //        flatbuffers::emplace_scalar(&mut buf[..], n);
    //        let m: T = flatbuffers::read_scalar(&buf[..]);
    //        assert!(n == m);
    //    }
    //}
    //doit::<u8>();
    //doit::<i8>();
    //doit::<u16>();
    //doit::<i16>();
    //doit::<u32>();
    //doit::<i32>();
    //doit::<u64>();
    //doit::<i64>();
}

#[cfg(test)]
mod write_and_read_examples {
    extern crate flatbuffers;

    use super::create_serialized_example_with_library_code;
    use super::create_serialized_example_with_generated_code;
    use super::serialized_example_is_accessible_and_correct;

    #[test]
    fn generated_code_creates_correct_example() {
        let b = &mut flatbuffers::FlatBufferBuilder::new();
        create_serialized_example_with_generated_code(b);
        let buf = b.finished_bytes();
        serialized_example_is_accessible_and_correct(&buf[..], true, false).unwrap();
    }

    #[test]
    fn library_code_creates_correct_example() {
        let b = &mut flatbuffers::FlatBufferBuilder::new();
        create_serialized_example_with_library_code(b);
        let buf = b.finished_bytes();
        serialized_example_is_accessible_and_correct(&buf[..], true, false).unwrap();
    }
}

#[cfg(test)]
mod read_examples_from_other_language_ports {
    extern crate flatbuffers;

    use super::load_file;
    use super::serialized_example_is_accessible_and_correct;

    #[test]
    fn gold_cpp_example_data_is_accessible_and_correct() {
        let buf = load_file("../monsterdata_test.mon");
        serialized_example_is_accessible_and_correct(&buf[..], true, false).unwrap();
    }
    #[test]
    fn java_wire_example_data_is_accessible_and_correct() {
        let buf = load_file("../monsterdata_java_wire.mon");
        serialized_example_is_accessible_and_correct(&buf[..], true, false).unwrap();
    }
    #[test]
    fn java_wire_size_prefixed_example_data_is_accessible_and_correct() {
        let buf = load_file("../monsterdata_java_wire_sp.mon");
        serialized_example_is_accessible_and_correct(&buf[..], true, true).unwrap();
    }
}


#[cfg(test)]
mod should_panic {
    extern crate flatbuffers;

    #[test]
    #[should_panic]
    fn end_table_should_panic_when_not_in_table() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        b.end_table(flatbuffers::Offset::new(0));
    }

    #[test]
    #[should_panic]
    fn create_string_should_panic_when_in_table() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        b.start_table(0);
        b.create_string("foo");
    }

    #[test]
    #[should_panic]
    fn create_byte_string_should_panic_when_in_table() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        b.start_table(0);
        b.create_byte_string(b"foo");
    }

    #[test]
    #[should_panic]
    fn push_struct_slot_should_panic_when_not_in_table() {
        #[derive(Copy, Clone, Debug, PartialEq)]
        #[repr(C, packed)]
        struct foo { }
        impl flatbuffers::GeneratedStruct for foo {}
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let x = foo{};
        b.push_slot_struct(0, &x);
    }

    #[test]
    #[should_panic]
    fn finished_bytes_should_panic_when_table_is_not_finished() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        b.start_table(0);
        b.finished_bytes();
    }
}

#[test]
fn create_byte_vector_fuzz() {
    fn prop(vec: Vec<u8>) {
        let xs = &vec[..];

        let mut b1 = flatbuffers::FlatBufferBuilder::new();
        b1.start_vector(flatbuffers::SIZE_U8, xs.len());

        for i in (0..xs.len()).rev() {
            b1.push_element_scalar(xs[i]);
        }
        b1.end_vector::<&u8>(xs.len());

        let mut b2 = flatbuffers::FlatBufferBuilder::new();
        b2.create_byte_vector(xs);
        assert_eq!(&b1.owned_buf[..], &b2.owned_buf[..]);
    }
    let n = 20;
    quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop as fn(Vec<_>));
}

#[test]
fn table_of_strings_fuzz() {
    fn prop(vec: Vec<String>) {
        use flatbuffers::field_index_to_field_offset as fi2fo;
        use flatbuffers::Follow;

        let xs = &vec[..];

        // build
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let str_offsets: Vec<flatbuffers::Offset<_>> = xs.iter().map(|s| b.create_string(&s[..])).collect();
        let table_start = b.start_table(xs.len() as flatbuffers::VOffsetT);

        for i in 0..xs.len() {
            b.push_slot_offset_relative(fi2fo(i as flatbuffers::VOffsetT), str_offsets[i]);
        }
        let root = b.end_table(table_start);
        b.finish_minimal(root);

        // use
        let buf = b.finished_bytes();
        let tab = <flatbuffers::ForwardsU32Offset<flatbuffers::Table>>::follow(buf, 0);

        for i in 0..xs.len() {
            let v = tab.get::<flatbuffers::ForwardsU32Offset<&str>>(fi2fo(i as flatbuffers::VOffsetT), None);
            assert_eq!(v, Some(&xs[i][..]));
        }
    }
    let n = 20;
    quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop as fn(Vec<String>));
}

#[test]
fn table_of_byte_strings_fuzz() {
    fn prop(vec: Vec<Vec<u8>>) {
        use flatbuffers::field_index_to_field_offset as fi2fo;
        use flatbuffers::Follow;

        let xs = &vec[..];

        // build
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let str_offsets: Vec<flatbuffers::Offset<_>> = xs.iter().map(|s| b.create_byte_string(&s[..])).collect();
        let table_start = b.start_table(xs.len() as flatbuffers::VOffsetT);

        for i in 0..xs.len() {
            b.push_slot_offset_relative(fi2fo(i as flatbuffers::VOffsetT), str_offsets[i]);
        }
        let root = b.end_table(table_start);
        b.finish_minimal(root);

        // use
        let buf = b.finished_bytes();
        let tab = <flatbuffers::ForwardsU32Offset<flatbuffers::Table>>::follow(buf, 0);

        for i in 0..xs.len() {
            let v = tab.get::<flatbuffers::ForwardsU32Offset<&[u8]>>(fi2fo(i as flatbuffers::VOffsetT), None);
            assert_eq!(v, Some(&xs[i][..]));
        }
    }
    prop(vec![vec![1,2,3]]);

    let n = 20;
    quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop as fn(Vec<_>));
}

#[test]
fn build_and_use_table_with_vector_of_scalars_fuzz() {
    fn prop<'a, T: flatbuffers::Follow<'a> + 'a + flatbuffers::ElementScalar + ::std::fmt::Debug>(vecs: Vec<Vec<T>>) {
        use flatbuffers::field_index_to_field_offset as fi2fo;
        use flatbuffers::Follow;

        // build
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let mut offs = vec![];
        for vec in &vecs {
            b.start_vector(vec.len(), ::std::mem::size_of::<T>());

            let xs = &vec[..];
            for i in (0..xs.len()).rev() {
                b.push_element_scalar::<T>(xs[i]);
            }
            let vecend = b.end_vector::<T>(xs.len());
            offs.push(vecend);
        }

        let table_start = b.start_table(vecs.len() as flatbuffers::VOffsetT);

        for i in 0..vecs.len() {
            b.push_slot_offset_relative(fi2fo(i as flatbuffers::VOffsetT), offs[i]);
        }
        let root = b.end_table(table_start);
        b.finish_minimal(root);

        // use
        let buf = b.finished_bytes();
        let tab = <flatbuffers::ForwardsU32Offset<flatbuffers::Table>>::follow(buf, 0);

        for i in 0..vecs.len() {
            let got = tab.get::<flatbuffers::ForwardsU32Offset<&[T]>>(fi2fo(i as flatbuffers::VOffsetT), None);
            assert!(got.is_some());
            let got2 = got.unwrap();
            assert_eq!(&vecs[i][..], got2);
        }
    }
    let n = 10;

    quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop as fn(Vec<Vec<bool>>));

    quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop as fn(Vec<Vec<u8>>));
    quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop as fn(Vec<Vec<u16>>));
    quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop as fn(Vec<Vec<u32>>));
    quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop as fn(Vec<Vec<u64>>));

    quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop as fn(Vec<Vec<u8>>));
    quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop as fn(Vec<Vec<u16>>));
    quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop as fn(Vec<Vec<u32>>));
    quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop as fn(Vec<Vec<u64>>));

    quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop as fn(Vec<Vec<f32>>));
    quickcheck::QuickCheck::new().max_tests(n).quickcheck(prop as fn(Vec<Vec<f64>>));
}

#[cfg(test)]
mod follow_impls {
    extern crate flatbuffers;
    use flatbuffers::Follow;
    use flatbuffers::field_index_to_field_offset as fi2fo;

    #[test]
    fn offset_to_ref_u8() {
        let vec: Vec<u8> = vec![255, 3];
        let fs: flatbuffers::FollowStart<&u8> = flatbuffers::FollowStart::new();
        assert_eq!(*fs.self_follow(&vec[..], 1), 3);
    }

    #[test]
    fn offset_to_u8() {
        let vec: Vec<u8> = vec![255, 3];
        let fs: flatbuffers::FollowStart<u8> = flatbuffers::FollowStart::new();
        assert_eq!(fs.self_follow(&vec[..], 1), 3);
    }

    #[test]
    fn offset_to_ref_u16() {
        let vec: Vec<u8> = vec![255, 255, 3, 4];
        let fs: flatbuffers::FollowStart<&u16> = flatbuffers::FollowStart::new();
        assert_eq!(*fs.self_follow(&vec[..], 2), 1027);
    }

    #[test]
    fn offset_to_u16() {
        let vec: Vec<u8> = vec![255, 255, 3, 4];
        let fs: flatbuffers::FollowStart<u16> = flatbuffers::FollowStart::new();
        assert_eq!(fs.self_follow(&vec[..], 2), 1027);
    }

    #[test]
    fn offset_to_f32() {
        let vec: Vec<u8> = vec![255, 255, 255, 255, /* start of value */ 208, 15, 73, 64];
        let fs: flatbuffers::FollowStart<&f32> = flatbuffers::FollowStart::new();
        assert_eq!(fs.self_follow(&vec[..], 4), &3.14159);
    }

    #[test]
    fn offset_to_string() {
        let vec: Vec<u8> = vec![255,255,255,255, 3, 0, 0, 0, 'f' as u8, 'o' as u8, 'o' as u8, 0];
        let off: flatbuffers::FollowStart<&str> = flatbuffers::FollowStart::new();
        assert_eq!(off.self_follow(&vec[..], 4), "foo");
    }

    #[test]
    fn offset_to_byte_string() {
        let vec: Vec<u8> = vec![255, 255, 255, 255, 3, 0, 0, 0, 1, 2, 3, 0];
        let off: flatbuffers::FollowStart<&[u8]> = flatbuffers::FollowStart::new();
        assert_eq!(off.self_follow(&vec[..], 4), &vec![1, 2, 3][..]);
    }

    #[test]
    fn offset_to_slice_of_u16() {
        let vec: Vec<u8> = vec![255, 255, 255, 255, 2, 0, 0, 0, 1, 2, 3, 4];
        let off: flatbuffers::FollowStart<&[u16]> = flatbuffers::FollowStart::new();
        assert_eq!(off.self_follow(&vec[..], 4), &vec![513, 1027][..]);
    }

    #[test]
    fn offset_to_vector_of_u16() {
        let vec: Vec<u8> = vec![255, 255, 255, 255, 2, 0, 0, 0, 1, 2, 3, 4];
        let off: flatbuffers::FollowStart<flatbuffers::Vector<u16>> = flatbuffers::FollowStart::new();
        assert_eq!(off.self_follow(&vec[..], 4).len(), 2);
        assert_eq!(off.self_follow(&vec[..], 4).get(0), 513);
        assert_eq!(off.self_follow(&vec[..], 4).get(1), 1027);
    }

    #[test]
    fn offset_to_struct() {
        #[derive(Copy, Clone, Debug, PartialEq)]
        #[repr(C, packed)]
        struct FooStruct {
            a: i8,
            b: u8,
            c: i16,
        }

        let vec: Vec<u8> = vec![255, 255, 255, 255, 1, 2, 3, 4];
        let off: flatbuffers::FollowStart<&FooStruct> = flatbuffers::FollowStart::new();
        assert_eq!(*off.self_follow(&vec[..], 4), FooStruct{a: 1, b: 2, c: 1027});
    }

    #[test]
    fn vector_of_offset_to_string_elements() {
        let buf: Vec<u8> = vec![/* vec len */ 1, 0, 0, 0, /* offset to string */ 4, 0, 0, 0, /* str length */ 3, 0, 0, 0, 'f' as u8, 'o' as u8, 'o' as u8, 0];
        let s: flatbuffers::FollowStart<flatbuffers::Vector<flatbuffers::ForwardsU32Offset<&str>>> = flatbuffers::FollowStart::new();
        assert_eq!(s.self_follow(&buf[..], 0).len(), 1);
        assert_eq!(s.self_follow(&buf[..], 0).get(0), "foo");
    }

    #[test]
    fn slice_of_struct_elements() {
        #[derive(Copy, Clone, Debug, PartialEq)]
        #[repr(C, packed)]
        struct FooStruct {
            a: i8,
            b: u8,
            c: i16,
        }

        let buf: Vec<u8> = vec![1, 0, 0, 0, /* struct data */ 1, 2, 3, 4];
        let fs: flatbuffers::FollowStart<&[FooStruct]> = flatbuffers::FollowStart::new();
        assert_eq!(fs.self_follow(&buf[..], 0).len(), 1);
        assert_eq!(fs.self_follow(&buf[..], 0).get(0), Some(&FooStruct{a: 1, b: 2, c: 1027}));
        assert_eq!(fs.self_follow(&buf[..], 0), &vec![FooStruct{a: 1, b: 2, c: 1027}][..]);
    }

    #[test]
    fn vector_of_struct_elements() {
        #[derive(Copy, Clone, Debug, PartialEq)]
        #[repr(C, packed)]
        struct FooStruct {
            a: i8,
            b: u8,
            c: i16,
        }

        let buf: Vec<u8> = vec![1, 0, 0, 0, /* struct data */ 1, 2, 3, 4];
        let fs: flatbuffers::FollowStart<flatbuffers::Vector<&FooStruct>> = flatbuffers::FollowStart::new();
        assert_eq!(fs.self_follow(&buf[..], 0).len(), 1);
        assert_eq!(fs.self_follow(&buf[..], 0).get(0), &FooStruct{a: 1, b: 2, c: 1027});
    }

    #[test]
    fn root_to_empty_table() {
	let buf: Vec<u8> = vec![
	    12, 0, 0, 0, // offset to root table
	    // enter vtable
	    4, 0, // vtable len
	    0, 0, // inline size
	    255, 255, 255, 255, // canary
	    // enter table
	    8, 0, 0, 0, // vtable location
	];
        let fs: flatbuffers::FollowStart<flatbuffers::ForwardsU32Offset<flatbuffers::Table>> = flatbuffers::FollowStart::new();
        assert_eq!(fs.self_follow(&buf[..], 0), flatbuffers::Table::new(&buf[..], 12));
    }

    #[test]
    fn table_get_slot_scalar_u8() {
	let buf: Vec<u8> = vec![
	    14, 0, 0, 0, // offset to root table
	    // enter vtable
	    6, 0, // vtable len
	    2, 0, // inline size
	    5, 0, // value loc
	    255, 255, 255, 255, // canary
	    // enter table
	    10, 0, 0, 0, // vtable location
	    0, 99 // value (with padding)
	];
        let fs: flatbuffers::FollowStart<flatbuffers::ForwardsU32Offset<flatbuffers::Table>> = flatbuffers::FollowStart::new();
        let tab = fs.self_follow(&buf[..], 0);
        assert_eq!(tab.get::<u8>(fi2fo(0), Some(123)), Some(99));
    }

    #[test]
    fn table_get_slot_scalar_u8_default_via_vtable_len() {
	let buf: Vec<u8> = vec![
	    12, 0, 0, 0, // offset to root table
	    // enter vtable
	    4, 0, // vtable len
	    2, 0, // inline size
	    255, 255, 255, 255, // canary
	    // enter table
	    8, 0, 0, 0, // vtable location
	];
        let fs: flatbuffers::FollowStart<flatbuffers::ForwardsU32Offset<flatbuffers::Table>> = flatbuffers::FollowStart::new();
        let tab = fs.self_follow(&buf[..], 0);
        assert_eq!(tab.get::<u8>(fi2fo(0), Some(123)), Some(123));
    }

    #[test]
    fn table_get_slot_scalar_u8_default_via_vtable_zero() {
	let buf: Vec<u8> = vec![
	    14, 0, 0, 0, // offset to root table
	    // enter vtable
	    6, 0, // vtable len
	    2, 0, // inline size
	    0, 0, // zero means use the default value
	    255, 255, 255, 255, // canary
	    // enter table
	    10, 0, 0, 0, // vtable location
	];
        let fs: flatbuffers::FollowStart<flatbuffers::ForwardsU32Offset<flatbuffers::Table>> = flatbuffers::FollowStart::new();
        let tab = fs.self_follow(&buf[..], 0);
        assert_eq!(tab.get::<u8>(fi2fo(0), Some(123)), Some(123));
    }

    #[test]
    fn table_get_slot_string_multiple_types() {
	let buf: Vec<u8> = vec![
	    14, 0, 0, 0, // offset to root table
	    // enter vtable
	    6, 0, // vtable len
	    2, 0, // inline size
	    4, 0, // value loc
	    255, 255, 255, 255, // canary
	    // enter table
	    10, 0, 0, 0, // vtable location
	    8, 0, 0, 0, // offset to string
	    // leave table
	    255, 255, 255, 255, // canary
	    // enter string
	    3, 0, 0, 0, 109, 111, 111, 0 // string length and contents
	];
        let tab = <flatbuffers::ForwardsU32Offset<flatbuffers::Table>>::follow(&buf[..], 0);
        assert_eq!(tab.get::<flatbuffers::ForwardsU32Offset<&str>>(fi2fo(0), None), Some("moo"));
        assert_eq!(tab.get::<flatbuffers::ForwardsU32Offset<&[u8]>>(fi2fo(0), None), Some(&vec![109, 111, 111][..]));
        let v = tab.get::<flatbuffers::ForwardsU32Offset<flatbuffers::Vector<u8>>>(fi2fo(0), None);
        assert_eq!(v.map(|x| x.into_slice_unfollowed()), Some(&vec![109, 111, 111][..]));
    }

    #[test]
    fn table_get_slot_string_multiple_types_default_via_vtable_len() {
	let buf: Vec<u8> = vec![
	    12, 0, 0, 0, // offset to root table
	    // enter vtable
	    4, 0, // vtable len
	    4, 0, // table inline len
	    255, 255, 255, 255, // canary
	    // enter table
	    8, 0, 0, 0, // vtable location
	];
        let tab = <flatbuffers::ForwardsU32Offset<flatbuffers::Table>>::follow(&buf[..], 0);
        assert_eq!(tab.get::<flatbuffers::ForwardsU32Offset<&str>>(fi2fo(0), Some("abc")), Some("abc"));
        assert_eq!(tab.get::<flatbuffers::ForwardsU32Offset<&[u8]>>(fi2fo(0), Some(&vec![70, 71, 72][..])), Some(&vec![70, 71, 72][..]));

        let default_vec_buf: Vec<u8> = vec![3, 0, 0, 0, 70, 71, 72, 0];
        let default_vec = flatbuffers::Vector::new(&default_vec_buf[..], 0);
        let v = tab.get::<flatbuffers::ForwardsU32Offset<flatbuffers::Vector<u8>>>(fi2fo(0), Some(default_vec));
        assert_eq!(v.map(|x| x.into_slice_unfollowed()), Some(&vec![70, 71, 72][..]));
    }

    #[test]
    fn table_get_slot_string_multiple_types_default_via_vtable_zero() {
	let buf: Vec<u8> = vec![
	    14, 0, 0, 0, // offset to root table
	    // enter vtable
	    6, 0, // vtable len
	    2, 0, // inline size
	    0, 0, // value loc
	    255, 255, 255, 255, // canary
	    // enter table
	    10, 0, 0, 0, // vtable location
	];
        let tab = <flatbuffers::ForwardsU32Offset<flatbuffers::Table>>::follow(&buf[..], 0);
        assert_eq!(tab.get::<flatbuffers::ForwardsU32Offset<&str>>(fi2fo(0), Some("abc")), Some("abc"));
        assert_eq!(tab.get::<flatbuffers::ForwardsU32Offset<&[u8]>>(fi2fo(0), Some(&vec![70, 71, 72][..])), Some(&vec![70, 71, 72][..]));

        let default_vec_buf: Vec<u8> = vec![3, 0, 0, 0, 70, 71, 72, 0];
        let default_vec = flatbuffers::Vector::new(&default_vec_buf[..], 0);
        let v = tab.get::<flatbuffers::ForwardsU32Offset<flatbuffers::Vector<u8>>>(fi2fo(0), Some(default_vec));
        assert!(v.is_some());
        assert_eq!(v.unwrap().as_slice_unfollowed(), &vec![70, 71, 72][..]);
    }

}

#[cfg(test)]
mod byte_layouts {
    extern crate flatbuffers;
    use flatbuffers::field_index_to_field_offset as fi2fo;

    fn check<'a>(b: &'a flatbuffers::FlatBufferBuilder, want: &'a [u8]) {
        let got = b.get_active_buf_slice();
        assert_eq!(want, got);
        //let message = format!("case %d: want\n%v\nbut got\n%v\n", i, want, got);
        //let message = format!("foo: {}", case_message);
        //assert_eq!(1, 1);//, message);
    }

    //fn run<f: Fn(&mut flatbuffers::FlatBufferBuilder, &)(label: &'static str, f: F

    #[test]
    fn layout_01_basic_numbers() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        b.push_element_scalar(true);
        check(&b, &[1]);
        b.push_element_scalar(-127i8);
        check(&b, &[129, 1]);
        b.push_element_scalar(255u8);
        check(&b, &[255, 129, 1]);
        b.push_element_scalar(-32222i16);
        check(&b, &[0x22, 0x82, 0, 255, 129, 1]); // first pad
        b.push_element_scalar(0xFEEEu16);
        check(&b, &[0xEE, 0xFE, 0x22, 0x82, 0, 255, 129, 1]); // no pad this time
        b.push_element_scalar(-53687092i32);
        check(&b, &[204, 204, 204, 252, 0xEE, 0xFE, 0x22, 0x82, 0, 255, 129, 1]);
        b.push_element_scalar(0x98765432u32);
        check(&b, &[0x32, 0x54, 0x76, 0x98, 204, 204, 204, 252, 0xEE, 0xFE, 0x22, 0x82, 0, 255, 129, 1]);
    }

    #[test]
    fn layout_01b_bigger_numbers() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        b.push_element_scalar(0x1122334455667788u64);
        check(&b, &[0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]);
    }

    #[test]
    fn layout_02_1xbyte_vector() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        check(&b, &[]);
        b.start_vector(flatbuffers::SIZE_U8, 1);
        check(&b, &[0, 0, 0]); // align to 4bytes
        b.push_element_scalar(1u8);
        check(&b, &[1, 0, 0, 0]);
        b.end_vector::<&u8>(1);
        check(&b, &[1, 0, 0, 0, 1, 0, 0, 0]); // padding
    }

    #[test]
    fn layout_03_2xbyte_vector() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        b.start_vector(flatbuffers::SIZE_U8, 2);
        check(&b, &[0, 0]); // align to 4bytes
        b.push_element_scalar(1u8);
        check(&b, &[1, 0, 0]);
        b.push_element_scalar(2u8);
        check(&b, &[2, 1, 0, 0]);
        b.end_vector::<&u8>(2);
        check(&b, &[2, 0, 0, 0, 2, 1, 0, 0]); // padding
    }

    #[test]
    fn layout_03b_11xbyte_vector_matches_builder_size() {
        let mut b = flatbuffers::FlatBufferBuilder::new_with_capacity(12);
        b.start_vector(flatbuffers::SIZE_U8, 8);

        let mut gold = vec![0u8; 0];
        check(&b, &gold[..]);

        for i in 1u8..=8 {
            b.push_element_scalar(i);
            gold.insert(0, i);
            check(&b, &gold[..]);
        }
        b.end_vector::<&u8>(8);
        let want = vec![8u8, 0, 0, 0,  8, 7, 6, 5, 4, 3, 2, 1];
        check(&b, &want[..]);
    }
    #[test]
    fn layout_04_1xuint16_vector() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        b.start_vector(flatbuffers::SIZE_U16, 1);
        check(&b, &[0, 0]); // align to 4bytes
        b.push_element_scalar(1u16);
        check(&b, &[1, 0, 0, 0]);
        b.end_vector::<&u16>(1);
        check(&b, &[1, 0, 0, 0, 1, 0, 0, 0]); // padding
    }

    #[test]
    fn layout_05_2xuint16_vector() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let _off = b.start_vector(flatbuffers::SIZE_U16, 2);
        check(&b, &[]); // align to 4bytes
        b.push_element_scalar(0xABCDu16);
        check(&b, &[0xCD, 0xAB]);
        b.push_element_scalar(0xDCBAu16);
        check(&b, &[0xBA, 0xDC, 0xCD, 0xAB]);
        b.end_vector::<&u16>(2);
        check(&b, &[2, 0, 0, 0, 0xBA, 0xDC, 0xCD, 0xAB]);
    }

    #[test]
    fn layout_06_create_string() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let off0 = b.create_string("foo");
        assert_eq!(8, off0.value());
        check(&b, b"\x03\x00\x00\x00foo\x00"); // 0-terminated, no pad
        let off1 = b.create_string("moop");
        assert_eq!(20, off1.value());
        check(&b, b"\x04\x00\x00\x00moop\x00\x00\x00\x00\
                    \x03\x00\x00\x00foo\x00"); // 0-terminated, 3-byte pad
    }

    #[test]
    fn layout_06b_create_string_unicode() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        // These characters are chinese from blog.golang.org/strings
        // We use escape codes here so that editors without unicode support
        // aren't bothered:
        let uni_str = "\u{65e5}\u{672c}\u{8a9e}";
        let off0 = b.create_string(uni_str);
        assert_eq!(16, off0.value());
        check(&b, &[9, 0, 0, 0, 230, 151, 165, 230, 156, 172, 232, 170, 158, 0, //  null-terminated, 2-byte pad
                    0, 0]);
    }

    #[test]
    fn layout_06c_create_byte_string() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let off0 = b.create_byte_string(b"foo");
        assert_eq!(8, off0.value());
        check(&b, b"\x03\x00\x00\x00foo\x00"); // 0-terminated, no pad
        let off1 = b.create_byte_string(b"moop");
        assert_eq!(20, off1.value());
        check(&b, b"\x04\x00\x00\x00moop\x00\x00\x00\x00\
                    \x03\x00\x00\x00foo\x00"); // 0-terminated, 3-byte pad
    }

    #[test]
    fn layout_07_empty_vtable() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let off0 = b.start_table(0);
        check(&b, &[]);
        b.end_table(off0);
        check(&b, &[4, 0, // vtable length
                    4, 0, // length of table including vtable offset
                    4, 0, 0, 0]); // offset for start of vtable
    }

    #[test]
    fn layout_08_vtable_with_one_true_bool() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        check(&b, &[]);
        let off0 = b.start_table(1);
        assert_eq!(0, off0.value());
        check(&b, &[]);
        b.push_slot_scalar(fi2fo(0), true, false);
        check(&b, &[1]);
        let off1 = b.end_table(off0);
        assert_eq!(8, off1.value());
        check(&b, &[
              6, 0, // vtable bytes
              8, 0, // length of object including vtable offset
              7, 0, // start of bool value
              6, 0, 0, 0, // offset for start of vtable (int32)
              0, 0, 0, // padded to 4 bytes
              1, // bool value
        ]);
    }

    #[test]
    fn layout_09_vtable_with_one_default_bool() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        check(&b, &[]);
        let off = b.start_table(1);
        check(&b, &[]);
        b.push_slot_scalar(fi2fo(0), false, false);
        b.end_table(off);
        check(&b, &[
             4, 0, // vtable bytes
             4, 0, // end of object from here
             // entry 1 is zero and not stored.
             4, 0, 0, 0, // offset for start of vtable (int32)
        ]);
    }

    #[test]
    fn layout_10_vtable_with_one_int16() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        check(&b, &[]);
        let off = b.start_table(1);
        b.push_slot_scalar(fi2fo(0), 0x789Ai16, 0);
        b.end_table(off);
        check(&b, &[
              6, 0, // vtable bytes
              8, 0, // end of object from here
              6, 0, // offset to value
              6, 0, 0, 0, // offset for start of vtable (int32)
              0, 0, // padding to 4 bytes
              0x9A, 0x78,
        ]);
    }

    #[test]
    fn layout_11_vtable_with_two_int16() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let off = b.start_table(2);
        b.push_slot_scalar(fi2fo(0), 0x3456i16, 0);
        b.push_slot_scalar(fi2fo(1), 0x789Ai16, 0);
        b.end_table(off);
        check(&b, &[
              8, 0, // vtable bytes
              8, 0, // end of object from here
              6, 0, // offset to value 0
              4, 0, // offset to value 1
              8, 0, 0, 0, // offset for start of vtable (int32)
              0x9A, 0x78, // value 1
              0x56, 0x34, // value 0
        ]);
    }

    #[test]
    fn layout_12_vtable_with_int16_and_bool() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let off = b.start_table(2);
        b.push_slot_scalar(fi2fo(0), 0x3456i16, 0);
        b.push_slot_scalar(fi2fo(1), true, false);
        b.end_table(off);
        check(&b, &[
            8, 0, // vtable bytes
            8, 0, // end of object from here
            6, 0, // offset to value 0
            5, 0, // offset to value 1
            8, 0, 0, 0, // offset for start of vtable (int32)
            0,          // padding
            1,          // value 1
            0x56, 0x34, // value 0
        ]);
    }

    #[test]
    fn layout_12b_vtable_with_empty_vector() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        b.start_vector(flatbuffers::SIZE_U8, 0);
        let vecend = b.end_vector::<&u8>(0);
        let off = b.start_table(1);
        b.push_slot_offset_relative(fi2fo(0), vecend);
        b.end_table(off);
        check(&b, &[
              6, 0, // vtable bytes
              8, 0,
              4, 0, // offset to vector offset
              6, 0, 0, 0, // offset for start of vtable (int32)
              4, 0, 0, 0,
              0, 0, 0, 0, // length of vector (not in struct)
        ]);
    }

    #[test]
    fn layout_12c_vtable_with_empty_vector_of_byte_and_some_scalars() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        b.start_vector(flatbuffers::SIZE_U8, 0);
        let vecend = b.end_vector::<&u8>(0);
        let off = b.start_table(2);
        b.push_slot_scalar::<i16>(fi2fo(0), 55i16, 0);
        b.push_slot_scalar_indirect_uoffset(fi2fo(1), vecend.value(), 0);
        b.end_table(off);
        check(&b, &[
              8, 0, // vtable bytes
              12, 0,
              10, 0, // offset to value 0
              4, 0, // offset to vector offset
              8, 0, 0, 0, // vtable loc
              8, 0, 0, 0, // value 1
              0, 0, 55, 0, // value 0

              0, 0, 0, 0, // length of vector (not in struct)
        ]);
    }
    #[test]
    fn layout_13_vtable_with_1_int16_and_2_vector_of_i16() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        b.start_vector(flatbuffers::SIZE_I16, 2);
        b.push_element_scalar(0x1234i16);
        b.push_element_scalar(0x5678i16);
        let vecend = b.end_vector::<&i16>(2);
        let off = b.start_table(2);
        b.push_slot_scalar_indirect_uoffset(fi2fo(1), vecend.value(), 0);
        b.push_slot_scalar(fi2fo(0), 55i16, 0);
        b.end_table(off);
        check(&b, &[
              8, 0, // vtable bytes
              12, 0, // length of object
              6, 0, // start of value 0 from end of vtable
              8, 0, // start of value 1 from end of buffer
              8, 0, 0, 0, // offset for start of vtable (int32)
              0, 0, // padding
              55, 0, // value 0
              4, 0, 0, 0, // vector position from here
              2, 0, 0, 0, // length of vector (uint32)
              0x78, 0x56, // vector value 1
              0x34, 0x12, // vector value 0
        ]);
    }
    #[test]
    fn layout_14_vtable_with_1_struct_of_int8_and_int16_and_int32() {
        #[derive(Copy, Clone, Debug, PartialEq)]
        #[repr(C, packed)]
        struct foo {
            a: i32,
            _pad0: [u8; 2],
            b: i16,
            _pad1: [u8; 3],
            c: i8,
        }
        impl flatbuffers::GeneratedStruct for foo {}

        let mut b = flatbuffers::FlatBufferBuilder::new();
        let off = b.start_table(1);
        //b.prep(::std::mem::size_of::<foo>(), 0);
        //b.prep(4+4+4, 0);
        //b.push_element_scalar(55i8);
        //b.pad(3);
        //b.push_element_scalar(0x1234i16);
        //b.pad(2);
        //b.push_element_scalar(0x12345678i32);
        //let struct_start = b.rev_cur_idx();
        let x = foo{a: 0x12345678i32.to_le(), _pad0: [0,0], b: 0x1234i16.to_le(), _pad1: [0, 0, 0], c: 55i8.to_le()};
        b.push_slot_struct(fi2fo(0), &x);
        b.end_table(off);
        check(&b, &[
              6, 0, // vtable bytes
              16, 0, // end of object from here
              4, 0, // start of struct from here
              6, 0, 0, 0, // offset for start of vtable (int32)
              0x78, 0x56, 0x34, 0x12, // value 2
              0, 0, // padding
              0x34, 0x12, // value 1
              0, 0, 0, // padding
              55, // value 0
        ]);
    }
  	// test 15: vtable with 1 vector of 2 struct of 2 int8
    #[test]
    fn layout_15_vtable_with_1_vector_of_2_struct_2_int8() {
        #[allow(dead_code)]
        struct FooStruct {
            a: i8,
            b: i8,
        }
        //impl<'a> flatbuffers::Follow<'a> for FooStruct {
        //    type Inner = &'a FooStruct;
        //    fn follow(&'a self, _buf: &'a [u8], loc: usize) -> Self::Inner {
        //        self
        //    }
        //}
        let mut b = flatbuffers::FlatBufferBuilder::new();
        b.start_vector(::std::mem::size_of::<FooStruct>(), 2);
        b.push_element_scalar(33i8);
        b.push_element_scalar(44i8);
        b.push_element_scalar(55i8);
        b.push_element_scalar(66i8);
        let vecend = b.end_vector::<&FooStruct>(2);
        let off = b.start_table(1);
        b.push_slot_scalar_indirect_uoffset(fi2fo(0), vecend.value(), 0);
        b.end_table(off);
        check(&b, &[
              6, 0, // vtable bytes
              8, 0,
              4, 0, // offset of vector offset
              6, 0, 0, 0, // offset for start of vtable (int32)
              4, 0, 0, 0, // vector start offset

              2, 0, 0, 0, // vector length
              66, // vector value 1,1
              55, // vector value 1,0
              44, // vector value 0,1
              33, // vector value 0,0
        ]);
    }

    #[test]
    fn layout_16_table_with_some_elements() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let off = b.start_table(2);
        b.push_slot_scalar(fi2fo(0), 33i8, 0);
        b.push_slot_scalar(fi2fo(1), 66i16, 0);
        let off2 = b.end_table(off);
        b.finish_minimal(off2);

        check(&b, &[
              12, 0, 0, 0, // root of table: points to vtable offset

              8, 0, // vtable bytes
              8, 0, // end of object from here
              7, 0, // start of value 0
              4, 0, // start of value 1

              8, 0, 0, 0, // offset for start of vtable (int32)

              66, 0, // value 1
              0,  // padding
              33, // value 0
        ]);
    }

    #[test]
    fn layout_17_one_unfinished_table_and_one_finished_table() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        {
            let off = b.start_table(2);
            b.push_slot_scalar(fi2fo(0), 33i8, 0);
            b.push_slot_scalar(fi2fo(1), 44i8, 0);
            b.end_table(off);
        }

        {
            let off = b.start_table(3);
            b.push_slot_scalar(fi2fo(0), 55i8, 0);
            b.push_slot_scalar(fi2fo(1), 66i8, 0);
            b.push_slot_scalar(fi2fo(2), 77i8, 0);
            let off2 = b.end_table(off);
            b.finish_minimal(off2);
        }

        check(&b, &[
              16, 0, 0, 0, // root of table: points to object
              0, 0, // padding

              10, 0, // vtable bytes
              8, 0, // size of object
              7, 0, // start of value 0
              6, 0, // start of value 1
              5, 0, // start of value 2
              10, 0, 0, 0, // offset for start of vtable (int32)
              0,  // padding
              77, // value 2
              66, // value 1
              55, // value 0

              //12, 0, 0, 0, // root of table: points to object

              8, 0, // vtable bytes
              8, 0, // size of object
              7, 0, // start of value 0
              6, 0, // start of value 1
              8, 0, 0, 0, // offset for start of vtable (int32)
              0, 0, // padding
              44, // value 1
              33, // value 0
              ]);
    }

    #[test]
    fn layout_18_a_bunch_of_bools() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let off = b.start_table(8);
        b.push_slot_scalar(fi2fo(0), true, false);
        b.push_slot_scalar(fi2fo(1), true, false);
        b.push_slot_scalar(fi2fo(2), true, false);
        b.push_slot_scalar(fi2fo(3), true, false);
        b.push_slot_scalar(fi2fo(4), true, false);
        b.push_slot_scalar(fi2fo(5), true, false);
        b.push_slot_scalar(fi2fo(6), true, false);
        b.push_slot_scalar(fi2fo(7), true, false);
        let off2 = b.end_table(off);
        b.finish_minimal(off2);

        check(&b, &[
              24, 0, 0, 0, // root of table: points to vtable offset

              20, 0, // vtable bytes
              12, 0, // size of object
              11, 0, // start of value 0
              10, 0, // start of value 1
              9, 0, // start of value 2
              8, 0, // start of value 3
              7, 0, // start of value 4
              6, 0, // start of value 5
              5, 0, // start of value 6
              4, 0, // start of value 7
              20, 0, 0, 0, // vtable offset

              1, // value 7
              1, // value 6
              1, // value 5
              1, // value 4
              1, // value 3
              1, // value 2
              1, // value 1
              1, // value 0
              ]);
    }

    #[test]
    fn layout_19_three_bools() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let off = b.start_table(3);
        b.push_slot_scalar(fi2fo(0), true, false);
        b.push_slot_scalar(fi2fo(1), true, false);
        b.push_slot_scalar(fi2fo(2), true, false);
        let off2 = b.end_table(off);
        b.finish_minimal(off2);

        check(&b, &[
              16, 0, 0, 0, // root of table: points to vtable offset

              0, 0, // padding

              10, 0, // vtable bytes
              8, 0, // size of object
              7, 0, // start of value 0
              6, 0, // start of value 1
              5, 0, // start of value 2
              10, 0, 0, 0, // vtable offset from here

              0, // padding
              1, // value 2
              1, // value 1
              1, // value 0
        ]);
    }

    #[test]
    fn layout_20_some_floats() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let off = b.start_table(1);
        b.push_slot_scalar(fi2fo(0), 1.0f32, 0.0);
        b.end_table(off);

        check(&b, &[
              6, 0, // vtable bytes
              8, 0, // size of object
              4, 0, // start of value 0
              6, 0, 0, 0, // vtable offset

              0, 0, 128, 63, // value 0
        ]);
    }

    #[test]
    fn layout_21_vtable_defaults() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let off = b.start_table(3);
        b.push_slot_scalar::<i8>(fi2fo(0), 1, 1);
        b.push_slot_scalar::<i8>(fi2fo(1), 3, 2);
        b.push_slot_scalar::<i8>(fi2fo(2), 3, 3);
        b.end_table(off);
        check(&b, &[
              8, 0, // vtable size in bytes
              8, 0, // object inline data in bytes
              0, 0, // entry 1/3: 0 => default
              7, 0, // entry 2/3: 7 => table start + 7 bytes
              // entry 3/3: not present => default
              8, 0, 0, 0,
              0, 0, 0,
              3,
        ]);
    }

    #[test]
    fn layout_22_root() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let off = b.start_table(3);
        // skipped: b.push_slot_scalar::<i16>(0, 1, 1);
        b.push_slot_scalar::<i16>(fi2fo(1), 3, 2);
        b.push_slot_scalar::<i16>(fi2fo(2), 3, 3);
        let table_end = b.end_table(off);
        b.finish_minimal(table_end);
        check(&b, &[
              12, 0, 0, 0, // root

              8, 0, // vtable size in bytes
              8, 0, // object inline data in bytes
              0, 0, // entry 1/3: 0 => default
              6, 0, // entry 2/3: 6 => table start + 6 bytes
              // entry 3/3: not present => default
              8, 0, 0, 0, // size of table data in bytes
              0, 0, // padding
              3, 0, // value 2/3
        ]);
    }
    #[test]
    fn layout_23_varied_slots_and_root() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let off = b.start_table(3);
        b.push_slot_scalar::<i16>(fi2fo(0), 1, 0);
        b.push_slot_scalar::<u8>(fi2fo(1), 2, 0);
        b.push_slot_scalar::<f32>(fi2fo(2), 3.0, 0.0);
        let table_end = b.end_table(off);
        b.finish_minimal(table_end);
        check(&b, &[
              16, 0, 0, 0, // root
              0, 0, // padding
              10, 0, // vtable bytes
              12, 0, // object inline data size
              10, 0, // offset to value #1 (i16)
              9, 0, // offset to value #2 (u8)
              4, 0, // offset to value #3 (f32)
              10, 0, // size of table data in bytes
              0, 0, // padding
              0, 0, 64, 64, // value #3 => 3.0 (float32)
              0, 2, // value #1 => 2 (u8)
              1, 0, // value #0 => 1 (int16)
        ]);
    }
}

fn load_file(filename: &str) -> Vec<u8> {
    use std::io::Read;
    let mut f = std::fs::File::open(filename).expect("file does not exist");
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).expect("file reading failed");
    buf
}
