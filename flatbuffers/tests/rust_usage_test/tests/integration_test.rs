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
use rust_usage_test::monster_test_generated::MyGame;
//use rust_usage_test::namespace_test::NamespaceA;

//pub use MyGame::Example;

//mod MyGame;
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

//std::string test_data_path = "tests/";

fn create_serialized_example_with_generated_code(builder: &mut flatbuffers::FlatBufferBuilder) {
    let mon = {
        let fred_name = builder.create_string("Fred");
        let inventory = builder.create_vector_of_scalars::<u8>(&vec![0, 1, 2, 3, 4][..]);
        let test4 = builder.create_vector_of_structs(&vec![MyGame::Example::Test::new(10, 20),
                                                           MyGame::Example::Test::new(30, 40)][..]);
        let pos = MyGame::Example::Vec3::new(1.0, 2.0, 3.0, 3.0, MyGame::Example::Color::Green, MyGame::Example::Test::new(5i16, 6i8));
        let args = MyGame::Example::MonsterArgs{
            hp: 80,
            mana: 150,
            name: Some(builder.create_string("MyMonster")),
            pos: Some(&pos),
            test_type: MyGame::Example::Any::Monster,
            // TODO(rw): better offset ergonomics
            test: Some(flatbuffers::Offset::new(MyGame::Example::CreateMonster(builder, &MyGame::Example::MonsterArgs{
                name: Some(fred_name),
                ..Default::default()
            }).value())),
            inventory: Some(inventory),
            test4: Some(test4),
            //testarrayofstring: Some(builder.create_vector_of_strings(&["bob", "fred", "bob", "fred"])),
            testarrayofstring: Some(builder.create_vector_of_strings(&["test1", "test2"])),
            ..Default::default()
        };
        MyGame::Example::CreateMonster(builder, &args)
    };
    MyGame::Example::FinishMonsterBuffer(builder, mon);
}
fn create_serialized_example_with_library_code<'a>(builder: &'a mut flatbuffers::FlatBufferBuilder<'a>) {
    let nested_union_mon = {
        let name = builder.create_string("Fred");
        let table_start = builder.start_table(34);
        builder.push_slot_offset_relative(MyGame::Example::Monster::VT_NAME, name);
        builder.end_table(table_start)
    };
    let pos = MyGame::Example::Vec3::new(1.0, 2.0, 3.0, 3.0, MyGame::Example::Color::Green, MyGame::Example::Test::new(5i16, 6i8));
    let inv = builder.create_vector_of_scalars::<u8>(&vec![0, 1, 2, 3, 4]);

    let test4 = builder.create_vector_of_structs(&vec![MyGame::Example::Test::new(10, 20),
                                                       MyGame::Example::Test::new(30, 40)][..]);

    let name = builder.create_string("MyMonster");
    let testarrayofstring = builder.create_vector_of_strings(&["test1", "test2"][..]);

    // begin building

    let table_start = builder.start_table(34);
    builder.push_slot_scalar::<i16>(MyGame::Example::Monster::VT_HP, 80, 100);
//    builder.push_slot_scalar::<i16>(MyGame::Example::Monster::VT_MANA, 150, 150);
    builder.push_slot_offset_relative::<&str>(MyGame::Example::Monster::VT_NAME, name);
    builder.push_slot_struct(MyGame::Example::Monster::VT_POS, &pos);
    builder.push_slot_scalar::<u8>(MyGame::Example::Monster::VT_TEST_TYPE, MyGame::Example::Any::Monster as u8, 0);
    builder.push_slot_offset_relative(MyGame::Example::Monster::VT_TEST, nested_union_mon);
    builder.push_slot_offset_relative(MyGame::Example::Monster::VT_INVENTORY, inv);
    builder.push_slot_offset_relative(MyGame::Example::Monster::VT_TEST4, test4);
    builder.push_slot_offset_relative(MyGame::Example::Monster::VT_TESTARRAYOFSTRING, testarrayofstring);
    let root = builder.end_table(table_start);
    builder.finish(root, Some(MyGame::Example::MonsterIdentifier()));

}

fn create_serialized_example_with_generated_code_more_fields(builder: &mut flatbuffers::FlatBufferBuilder) {
////  let x = MyGame::Example::Test::new(10, 20);
////  let _vec = MyGame::Example::Vec3::new(1.0,2.0,3.0,0.0, MyGame::Example::Color::Red, x);
////  let _name = builder.create_string("MyMonster");
////  let inv_data = vec![0, 1, 2, 3, 4];//, 5, 6, 7, 8, 9];
////  let inventory = builder.create_vector(&inv_data);
////
////  // Alternatively, create the vector first, and fill in data later:
////  // unsigned char *inv_buf = nullptr;
////  // auto inventory = builder.CreateUninitializedVector<unsigned char>(
////  //                                                              10, &inv_buf);
////  // memcpy(inv_buf, inv_data, 10);
////
////  let tests = vec![MyGame::Example::Test::new(10, 20), MyGame::Example::Test::new(30, 40)];
////
////  // Create a vector of structures from a lambda.
////  let testv = builder.create_vector_of_structs_from_fn(2, |i, s| *s = tests[i]);
////
////  // create monster with very few fields set:
////  // (same functionality as CreateMonster below, but sets fields manually)
////  let mut mlocs: [flatbuffers::Offset<MyGame::Example::Monster<'_>>; 3] = [flatbuffers::Offset::<_>::new(0); 3];
////  let fred = builder.create_string("Fred");
////  let barney = builder.create_string("Barney");
////  let wilma = builder.create_string("Wilma");
////
////  {
////      let mut mb1 = MyGame::Example::MonsterBuilder::new(builder);
////      mb1.add_name(fred);
////      mlocs[0] = mb1.finish();
////  }
////
////  {
////      let mut mb2 = MyGame::Example::MonsterBuilder::new(builder);
////      mb2.add_name(barney);
////      mb2.add_hp(1000);
////      mlocs[1] = mb2.finish();
////  }
////
////  {
////      let mut mb3 = MyGame::Example::MonsterBuilder::new(builder);
////      mb3.add_name(wilma);
////      mlocs[2] = mb3.finish();
////  }
////
////  // Create an array of strings. Also test string pooling, and lambdas.
////  let names: [&'static str; 4] = ["bob", "fred", "bob", "fred"];
////  let vecofstrings = builder.create_vector_of_strings(&names);
////  //let vecofstrings = builder.create_vector_from_fn::<_, _>(
////  //    4,
////  //    |i, b| -> flatbuffers::Offset<flatbuffers::StringOffset> {
////  //        b.create_shared_string(names[i])
////  //    });
////
////  // Creating vectors of strings in one convenient call.
////  let names2 = vec!["jane", "mary"];
////  let vecofstrings2 = builder.create_vector_of_strings(&names2);
////
////  // Create an array of sorted tables, can be used with binary search when read:
////  let vecoftables = builder.create_vector_of_sorted_tables(&mut mlocs);
////
////  // Create an array of sorted structs,
////  // can be used with binary search when read:
////  let mut abilities = vec![];
////  abilities.push(MyGame::Example::Ability::new(4, 40));
////  abilities.push(MyGame::Example::Ability::new(3, 30));
////  abilities.push(MyGame::Example::Ability::new(2, 20));
////  abilities.push(MyGame::Example::Ability::new(1, 10));
////  let vecofstructs = builder.create_vector_of_sorted_structs(&mut abilities);
////
////  // Create a nested FlatBuffer.
////  // Nested FlatBuffers are stored in a ubyte vector, which can be convenient
////  // since they can be memcpy'd around much easier than other FlatBuffer
////  // values. They have little overhead compared to storing the table directly.
////  // As a test, create a mostly empty Monster buffer:
////  let mut nested_builder = flatbuffers::FlatBufferBuilder::new();
////  let args = MyGame::Example::MonsterArgs{
////      mana: 0,
////      hp: 0,
////      name: Some(nested_builder.create_string("NestedMonster")),
////      ..Default::default()
////  };
////  let nmloc = MyGame::Example::CreateMonster(&mut nested_builder, &args);
////  MyGame::Example::FinishMonsterBuffer(&mut nested_builder, nmloc);
////
////  // Now we can store the buffer in the parent. Note that by default, vectors
////  // are only aligned to their elements or size field, so in this case if the
////  // buffer contains 64-bit elements, they may not be correctly aligned. We fix
////  // that with:
////  //builder.ForceVectorAlignment(nested_builder.get_size(), size_of(uint8_t),
////  //                             nested_builder.get_buffer_min_alignment());
////  // If for whatever reason you don't have the nested_builder available, you
////  // can substitute flatbuffers::largest_scalar_t (64-bit) for the alignment, or
////  // the largest force_align value in your schema if you're using it.
////  // TODO
////  let nested_flatbuffer_vector = builder.create_vector(&vec![0, 0][..]);
////  //    nested_builder.get_buffer_pointer(), nested_builder.get_size());
////
//////  // Test a nested FlexBuffer:
//////  flexbuffers::Builder flexbuild;
//////  flexbuild.Int(1234);
//////  flexbuild.Finish();
//////  auto flex = builder.CreateVector(flexbuild.GetBuffer());
//////
////    // shortcut for creating monster with all fields set:
////    let mloc = MyGame::Example::CreateMonster(builder, &MyGame::Example::MonsterArgs{
////        pos: Some(&_vec),
////        mana: 150,
////        hp: 80,
////        name: Some(_name),
////        inventory: Some(inventory),
////        color: MyGame::Example::Color::Blue,
////        test_type: MyGame::Example::Any::Monster,
////        test: None,//Some(mlocs[1].union()),  // Store a union.
////        test4: Some(testv),
////        testarrayofstring: Some(vecofstrings),
////        //testarrayoftables: vecoftables, // TODO
////        enemy: Some(flatbuffers::Offset::new(0)),
////        testnestedflatbuffer: Some(nested_flatbuffer_vector),
////        testempty: Some(flatbuffers::Offset::new(0)),
////        testbool: false,
////        testhashs32_fnv1: 0,
////        testhashu32_fnv1: 0,
////        testhashs64_fnv1: 0,
////        testhashu64_fnv1: 0,
////        testhashs32_fnv1a: 0,
////        testhashu32_fnv1a: 0,
////        testhashs64_fnv1a: 0,
////        testhashu64_fnv1a: 0,
////        testarrayofbools: Some(flatbuffers::Offset::new(0)),
////        testf: 3.14159f32,
////        testf2: 3.0f32,
////        testf3: 0.0f32,
////        //testarrayofstring2: vecofstrings2, // TODO
////        testarrayofsortedstruct: Some(vecofstructs),
////        flex: Some(flatbuffers::Offset::new(0)),
////        test5: Some(flatbuffers::Offset::new(0)),
////        vector_of_longs: Some(flatbuffers::Offset::new(0)),
////        vector_of_doubles: Some(flatbuffers::Offset::new(0)),
////        //parent_namespace_test: flatbuffers::Offset::new(0),
////
////        ..Default::default() // for phantom
////    });
////
////
////    let mloc = MyGame::Example::CreateMonster(builder, &args);
////    //builder.finish(mloc.value());
////    MyGame::Example::FinishMonsterBuffer(builder, mloc);
//////
//////  // clang-format off
//////  #ifdef FLATBUFFERS_TEST_VERBOSE
//////  // print byte data for debugging:
//////  auto p = builder.GetBufferPointer();
//////  for (flatbuffers::uoffset_t i = 0; i < builder.GetSize(); i++)
//////    printf("%d ", p[i]);
//////  #endif
//////  // clang-format on
//////
//////  // return the buffer for the caller to use.
//////  auto bufferpointer =
//////      reinterpret_cast<const char *>(builder.GetBufferPointer());
//////  buffer.assign(bufferpointer, bufferpointer + builder.GetSize());
//////
////  //return builder.get_active_buf_slice();
////  //return builder.release_buffer_pointer();
////
////  //return flatbuffers::DetachedBuffer{};
}
#[test]
fn test_generated_monster_identifier() {
    assert_eq!("MONS", MyGame::Example::MonsterIdentifier());
}
fn serialized_example_is_accessible_and_correct(bytes: &[u8], identifier_required: bool, size_prefixed: bool) -> Result<(), &'static str> {
    if identifier_required {
        let correct = if size_prefixed {
            MyGame::Example::MonsterSizePrefixedBufferHasIdentifier(bytes)
        } else {
            MyGame::Example::MonsterBufferHasIdentifier(bytes)
        };
        if !correct {
            return Err("incorrect buffer identifier");
        }
    }
    let monster1 = if size_prefixed {
        MyGame::Example::GetSizePrefixedRootAsMonster(bytes)
    } else {
        MyGame::Example::GetRootAsMonster(bytes)
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
        if pos.x() != 1.0f32 { return Err("bad pos.x"); }
        if pos.y() != 2.0f32 { return Err("bad pos.y"); }
        if pos.z() != 3.0f32 { return Err("bad pos.z"); }
        if pos.test1() != 3.0f64 { return Err("bad pos.test1"); }
        if pos.test2() != MyGame::Example::Color::Green { return Err("bad pos.test2"); }

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
        //assert_eq!(m.vector_of_longs(), Some(&[1, 100, 10000, 1000000, 100000000][..]));
        //assert_eq!(m.vector_of_longs(), Some(&[1, 100, 10000, 1000000, 100000000][..]));

        if m.test_type() != MyGame::Example::Any::Monster { return Err("bad m.test_type"); }

        let table2 = match m.test() {
            None => { return Err("bad m.test"); }
            Some(x) => { x }
        };

        let monster2 = MyGame::Example::Monster::init_from_table(table2);

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


        //{
        //    let test5 = match m.test5() {
        //        None => { return Err("bad m.test5"); }
        //        Some(x) => { x }
        //    };
        //    if test5.len() != 2 { return Err("bad test5.len") }
        //    if test5[0].a() != 10 { return Err("bad test5[0].a") }
        //    if test5[0].b() != 20 { return Err("bad test5[0].b") }
        //    if test5[1].a() != 10 { return Err("bad test5[1].a") }
        //    if test5[1].b() != 20 { return Err("bad test5[1].b") }
        //}

        //if m.testarrayofbools() != Some(&[true, false, true][..]) {
        //    return Err("bad m.testarrayofbools");
        //}

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
mod roundtrips_with_generated_code {
    extern crate flatbuffers;

    extern crate rust_usage_test;
    use rust_usage_test::monster_test_generated::MyGame;

    fn build_mon<'a, 'b>(builder: &'a mut flatbuffers::FlatBufferBuilder, args: &'b MyGame::Example::MonsterArgs) -> MyGame::Example::Monster<'a> {
        let mon = MyGame::Example::CreateMonster(builder, &args);
        MyGame::Example::FinishMonsterBuffer(builder, mon);
        MyGame::Example::GetRootAsMonster(builder.get_active_buf_slice())
    }

    #[test]
    fn scalar_store() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let m = build_mon(&mut b, &MyGame::Example::MonsterArgs{hp: 123, ..Default::default()});
        assert_eq!(m.hp(), 123);
    }
    #[test]
    fn scalar_default() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let m = build_mon(&mut b, &MyGame::Example::MonsterArgs{..Default::default()});
        assert_eq!(m.hp(), 100);
    }
    #[test]
    fn string_store() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let name = b.create_string("foobar");
        let m = build_mon(&mut b, &MyGame::Example::MonsterArgs{name: Some(name), ..Default::default()});
        assert_eq!(m.name(), Some("foobar"));
    }
    #[test]
    fn enum_store() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let m = build_mon(&mut b, &MyGame::Example::MonsterArgs{color: MyGame::Example::Color::Red, ..Default::default()});
        assert_eq!(m.color(), MyGame::Example::Color::Red);
    }
    #[test]
    fn enum_default() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let m = build_mon(&mut b, &MyGame::Example::MonsterArgs{..Default::default()});
        assert_eq!(m.color(), MyGame::Example::Color::Blue);
    }
    #[test]
    fn vector_of_string_store() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let v = b.create_vector_of_strings(&["foobar", "baz"]);
        let m = build_mon(&mut b, &MyGame::Example::MonsterArgs{testarrayofstring: Some(v), ..Default::default()});
        assert_eq!(m.testarrayofstring().unwrap().len(), 2);
        assert_eq!(m.testarrayofstring().unwrap().get(0), "foobar");
        assert_eq!(m.testarrayofstring().unwrap().get(1), "baz");
    }
    #[test]
    fn vector_of_ubyte_store() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let v = b.create_vector_of_scalars::<u8>(&[123, 234][..]);
        let m = build_mon(&mut b, &MyGame::Example::MonsterArgs{inventory: Some(v), ..Default::default()});
        assert_eq!(m.inventory().unwrap(), &[123, 234][..]);
    }
    #[test]
    fn vector_of_bool_store() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let v = b.create_vector_of_scalars::<bool>(&[false, true, false, true][..]);
        let m = build_mon(&mut b, &MyGame::Example::MonsterArgs{testarrayofbools: Some(v), ..Default::default()});
        assert_eq!(m.testarrayofbools().unwrap(), &[false, true, false, true][..]);
    }
    #[test]
    fn vector_of_f64_store() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let v = b.create_vector_of_scalars::<f64>(&[3.14159265359][..]);
        let m = build_mon(&mut b, &MyGame::Example::MonsterArgs{vector_of_doubles: Some(v), ..Default::default()});
        assert_eq!(m.vector_of_doubles().unwrap(), &[3.14159265359][..]);
    }
    #[test]
    fn vector_of_struct_store() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let v = b.create_vector_of_structs::<MyGame::Example::Test>(&[MyGame::Example::Test::new(127, -128), MyGame::Example::Test::new(3, 123)][..]);
        let m = build_mon(&mut b, &MyGame::Example::MonsterArgs{test4: Some(v), ..Default::default()});
        assert_eq!(m.test4().unwrap(), &[MyGame::Example::Test::new(127, -128), MyGame::Example::Test::new(3, 123)][..]);
    }
    #[ignore]
    #[test]
    fn vector_of_table_store() {
        let b = &mut flatbuffers::FlatBufferBuilder::new();
        let t0 = {
            let name = b.create_string("foo");
            let args = MyGame::Example::MonsterArgs{hp: 55, name: Some(name), ..Default::default()};
            MyGame::Example::CreateMonster(b, &args)
        };
        let t1 = {
            let name = b.create_string("bar");
            let args = MyGame::Example::MonsterArgs{name: Some(name), ..Default::default()};
            MyGame::Example::CreateMonster(b, &args)
        };
        assert!(false, "needs better ergonomics around writing tables re: offsets");
        let v = b.create_vector_of_reverse_offsets::<MyGame::Example::Monster>(&[t0, t1][..]);
        let m = build_mon(b, &MyGame::Example::MonsterArgs{testarrayoftables: Some(v), ..Default::default()});
        assert_eq!(m.testarrayoftables().unwrap().len(), 2);
        assert_eq!(m.testarrayoftables().unwrap().get(1).hp(), 55);
        assert_eq!(m.testarrayoftables().unwrap().get(0).name(), Some("foo"));
        assert_eq!(m.testarrayoftables().unwrap().get(1).hp(), 100);
        assert_eq!(m.testarrayoftables().unwrap().get(1).name(), Some("bar"));
    }
}

#[test]
fn force_align() {
    assert_eq!(std::mem::size_of::<MyGame::Example::Vec3>() % 16, 0);
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
        let root = b.finish_minimal(vecend);

        let buf = b.get_active_buf_slice();

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

        let buf = b.get_active_buf_slice();
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


////  example of accessing a buffer loaded in memory:
//void AccessFlatBufferTest(const uint8_t *flatbuf, size_t length,
//                          bool pooled = true) {
//  // First, verify the buffers integrity (optional)
//  flatbuffers::Verifier verifier(flatbuf, length);
//  TEST_EQ(VerifyMonsterBuffer(verifier), true);
//
//  std::vector<uint8_t> test_buff;
//  test_buff.resize(length * 2);
//  std::memcpy(&test_buff[0], flatbuf, length);
//  std::memcpy(&test_buff[length], flatbuf, length);
//
//  flatbuffers::Verifier verifier1(&test_buff[0], length);
//  TEST_EQ(VerifyMonsterBuffer(verifier1), true);
//  TEST_EQ(verifier1.GetComputedSize(), length);
//
//  flatbuffers::Verifier verifier2(&test_buff[length], length);
//  TEST_EQ(VerifyMonsterBuffer(verifier2), true);
//  TEST_EQ(verifier2.GetComputedSize(), length);
//
//  TEST_EQ(strcmp(MonsterIdentifier(), "MONS"), 0);
//  TEST_EQ(MonsterBufferHasIdentifier(flatbuf), true);
//  TEST_EQ(strcmp(MonsterExtension(), "mon"), 0);
//
//  // Access the buffer from the root.
//  auto monster = GetMonster(flatbuf);
//
//  TEST_EQ(monster->hp(), 80);
//  TEST_EQ(monster->mana(), 150);  // default
//  TEST_EQ_STR(monster->name()->c_str(), "MyMonster");
//  // Can't access the following field, it is deprecated in the schema,
//  // which means accessors are not generated:
//  // monster.friendly()
//
//  auto pos = monster->pos();
//  TEST_NOTNULL(pos);
//  TEST_EQ(pos->z(), 3);
//  TEST_EQ(pos->test3().a(), 10);
//  TEST_EQ(pos->test3().b(), 20);
//
//  auto inventory = monster->inventory();
//  TEST_EQ(VectorLength(inventory), 10UL);  // Works even if inventory is null.
//  TEST_NOTNULL(inventory);
//  unsigned char inv_data[] = { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9 };
//  for (auto it = inventory->begin(); it != inventory->end(); ++it)
//    TEST_EQ(*it, inv_data[it - inventory->begin()]);
//
//  TEST_EQ(monster->color(), Color_Blue);
//
//  // Example of accessing a union:
//  TEST_EQ(monster->test_type(), Any_Monster);  // First make sure which it is.
//  auto monster2 = reinterpret_cast<const Monster *>(monster->test());
//  TEST_NOTNULL(monster2);
//  TEST_EQ_STR(monster2->name()->c_str(), "Fred");
//
//  // Example of accessing a vector of strings:
//  auto vecofstrings = monster->testarrayofstring();
//  TEST_EQ(vecofstrings->Length(), 4U);
//  TEST_EQ_STR(vecofstrings->Get(0)->c_str(), "bob");
//  TEST_EQ_STR(vecofstrings->Get(1)->c_str(), "fred");
//  if (pooled) {
//    // These should have pointer equality because of string pooling.
//    TEST_EQ(vecofstrings->Get(0)->c_str(), vecofstrings->Get(2)->c_str());
//    TEST_EQ(vecofstrings->Get(1)->c_str(), vecofstrings->Get(3)->c_str());
//  }
//
//  auto vecofstrings2 = monster->testarrayofstring2();
//  if (vecofstrings2) {
//    TEST_EQ(vecofstrings2->Length(), 2U);
//    TEST_EQ_STR(vecofstrings2->Get(0)->c_str(), "jane");
//    TEST_EQ_STR(vecofstrings2->Get(1)->c_str(), "mary");
//  }
//
//  // Example of accessing a vector of tables:
//  auto vecoftables = monster->testarrayoftables();
//  TEST_EQ(vecoftables->Length(), 3U);
//  for (auto it = vecoftables->begin(); it != vecoftables->end(); ++it)
//    TEST_EQ(strlen(it->name()->c_str()) >= 4, true);
//  TEST_EQ_STR(vecoftables->Get(0)->name()->c_str(), "Barney");
//  TEST_EQ(vecoftables->Get(0)->hp(), 1000);
//  TEST_EQ_STR(vecoftables->Get(1)->name()->c_str(), "Fred");
//  TEST_EQ_STR(vecoftables->Get(2)->name()->c_str(), "Wilma");
//  TEST_NOTNULL(vecoftables->LookupByKey("Barney"));
//  TEST_NOTNULL(vecoftables->LookupByKey("Fred"));
//  TEST_NOTNULL(vecoftables->LookupByKey("Wilma"));
//
//  // Test accessing a vector of sorted structs
//  auto vecofstructs = monster->testarrayofsortedstruct();
//  if (vecofstructs) {  // not filled in monster_test.bfbs
//    for (flatbuffers::uoffset_t i = 0; i < vecofstructs->size() - 1; i++) {
//      auto left = vecofstructs->Get(i);
//      auto right = vecofstructs->Get(i + 1);
//      TEST_EQ(true, (left->KeyCompareLessThan(right)));
//    }
//    TEST_NOTNULL(vecofstructs->LookupByKey(3));
//    TEST_EQ(static_cast<const Ability *>(nullptr),
//            vecofstructs->LookupByKey(5));
//  }
//
//  // Test nested FlatBuffers if available:
//  auto nested_buffer = monster->testnestedflatbuffer();
//  if (nested_buffer) {
//    // nested_buffer is a vector of bytes you can memcpy. However, if you
//    // actually want to access the nested data, this is a convenient
//    // accessor that directly gives you the root table:
//    auto nested_monster = monster->testnestedflatbuffer_nested_root();
//    TEST_EQ_STR(nested_monster->name()->c_str(), "NestedMonster");
//  }
//
//  // Test flexbuffer if available:
//  auto flex = monster->flex();
//  // flex is a vector of bytes you can memcpy etc.
//  TEST_EQ(flex->size(), 4);  // Encoded FlexBuffer bytes.
//  // However, if you actually want to access the nested data, this is a
//  // convenient accessor that directly gives you the root value:
//  TEST_EQ(monster->flex_flexbuffer_root().AsInt16(), 1234);
//
//  // Since Flatbuffers uses explicit mechanisms to override the default
//  // compiler alignment, double check that the compiler indeed obeys them:
//  // (Test consists of a short and byte):
//  TEST_EQ(flatbuffers::AlignOf<Test>(), 2UL);
//  TEST_EQ(sizeof(Test), 4UL);
//
//  const flatbuffers::Vector<const Test *> *tests_array[] = {
//    monster->test4(),
//    monster->test5(),
//  };
//  for (size_t i = 0; i < sizeof(tests_array) / sizeof(tests_array[0]); ++i) {
//    auto tests = tests_array[i];
//    TEST_NOTNULL(tests);
//    auto test_0 = tests->Get(0);
//    auto test_1 = tests->Get(1);
//    TEST_EQ(test_0->a(), 10);
//    TEST_EQ(test_0->b(), 20);
//    TEST_EQ(test_1->a(), 30);
//    TEST_EQ(test_1->b(), 40);
//    for (auto it = tests->begin(); it != tests->end(); ++it) {
//      TEST_EQ(it->a() == 10 || it->a() == 30, true);  // Just testing iterators.
//    }
//  }
//
//  // Checking for presence of fields:
//  TEST_EQ(flatbuffers::IsFieldPresent(monster, Monster::VT_HP), true);
//  TEST_EQ(flatbuffers::IsFieldPresent(monster, Monster::VT_MANA), false);
//
//  // Obtaining a buffer from a root:
//  TEST_EQ(GetBufferStartFromRootPointer(monster), flatbuf);
//}
//
//// Change a FlatBuffer in-place, after it has been constructed.
//void MutateFlatBuffersTest(uint8_t *flatbuf, std::size_t length) {
//  // Get non-const pointer to root.
//  auto monster = GetMutableMonster(flatbuf);
//
//  // Each of these tests mutates, then tests, then set back to the original,
//  // so we can test that the buffer in the end still passes our original test.
//  auto hp_ok = monster->mutate_hp(10);
//  TEST_EQ(hp_ok, true);  // Field was present.
//  TEST_EQ(monster->hp(), 10);
//  // Mutate to default value
//  auto hp_ok_default = monster->mutate_hp(100);
//  TEST_EQ(hp_ok_default, true);  // Field was present.
//  TEST_EQ(monster->hp(), 100);
//  // Test that mutate to default above keeps field valid for further mutations
//  auto hp_ok_2 = monster->mutate_hp(20);
//  TEST_EQ(hp_ok_2, true);
//  TEST_EQ(monster->hp(), 20);
//  monster->mutate_hp(80);
//
//  // Monster originally at 150 mana (default value)
//  auto mana_default_ok = monster->mutate_mana(150);  // Mutate to default value.
//  TEST_EQ(mana_default_ok,
//          true);  // Mutation should succeed, because default value.
//  TEST_EQ(monster->mana(), 150);
//  auto mana_ok = monster->mutate_mana(10);
//  TEST_EQ(mana_ok, false);  // Field was NOT present, because default value.
//  TEST_EQ(monster->mana(), 150);
//
//  // Mutate structs.
//  auto pos = monster->mutable_pos();
//  auto test3 = pos->mutable_test3();  // Struct inside a struct.
//  test3.mutate_a(50);                 // Struct fields never fail.
//  TEST_EQ(test3.a(), 50);
//  test3.mutate_a(10);
//
//  // Mutate vectors.
//  auto inventory = monster->mutable_inventory();
//  inventory->Mutate(9, 100);
//  TEST_EQ(inventory->Get(9), 100);
//  inventory->Mutate(9, 9);
//
//  auto tables = monster->mutable_testarrayoftables();
//  auto first = tables->GetMutableObject(0);
//  TEST_EQ(first->hp(), 1000);
//  first->mutate_hp(0);
//  TEST_EQ(first->hp(), 0);
//  first->mutate_hp(1000);
//
//  // Run the verifier and the regular test to make sure we didn't trample on
//  // anything.
//  AccessFlatBufferTest(flatbuf, length);
//}
//fn check_read_buffer(buf: &[u8]) {
//	let monster1 = MyGame::Example::GetRootAsMonster(buf);
//	//let monster2 = {
//    //    let mut x = MyGame::Example::Monster::(..Default::default());
//    //};
//}

//// Unpack a FlatBuffer into objects.
//void ObjectFlatBuffersTest(uint8_t *flatbuf) {
//  // Optional: we can specify resolver and rehasher functions to turn hashed
//  // strings into object pointers and back, to implement remote references
//  // and such.
//  auto resolver = flatbuffers::resolver_function_t(
//      [](void **pointer_adr, flatbuffers::hash_value_t hash) {
//        (void)pointer_adr;
//        (void)hash;
//        // Don't actually do anything, leave variable null.
//      });
//  auto rehasher = flatbuffers::rehasher_function_t(
//      [](void *pointer) -> flatbuffers::hash_value_t {
//        (void)pointer;
//        return 0;
//      });
//
//  // Turn a buffer into C++ objects.
//  auto monster1 = UnPackMonster(flatbuf, &resolver);
//
//  // Re-serialize the data.
//  flatbuffers::FlatBufferBuilder fbb1;
//  fbb1.Finish(CreateMonster(fbb1, monster1.get(), &rehasher),
//              MonsterIdentifier());
//
//  // Unpack again, and re-serialize again.
//  auto monster2 = UnPackMonster(fbb1.GetBufferPointer(), &resolver);
//  flatbuffers::FlatBufferBuilder fbb2;
//  fbb2.Finish(CreateMonster(fbb2, monster2.get(), &rehasher),
//              MonsterIdentifier());
//
//  // Now we've gone full round-trip, the two buffers should match.
//  auto len1 = fbb1.GetSize();
//  auto len2 = fbb2.GetSize();
//  TEST_EQ(len1, len2);
//  TEST_EQ(memcmp(fbb1.GetBufferPointer(), fbb2.GetBufferPointer(), len1), 0);
//
//  // Test it with the original buffer test to make sure all data survived.
//  AccessFlatBufferTest(fbb2.GetBufferPointer(), len2, false);
//
//  // Test accessing fields, similar to AccessFlatBufferTest above.
//  TEST_EQ(monster2->hp, 80);
//  TEST_EQ(monster2->mana, 150);  // default
//  TEST_EQ_STR(monster2->name.c_str(), "MyMonster");
//
//  auto &pos = monster2->pos;
//  TEST_NOTNULL(pos);
//  TEST_EQ(pos->z(), 3);
//  TEST_EQ(pos->test3().a(), 10);
//  TEST_EQ(pos->test3().b(), 20);
//
//  auto &inventory = monster2->inventory;
//  TEST_EQ(inventory.size(), 10UL);
//  unsigned char inv_data[] = { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9 };
//  for (auto it = inventory.begin(); it != inventory.end(); ++it)
//    TEST_EQ(*it, inv_data[it - inventory.begin()]);
//
//  TEST_EQ(monster2->color, Color_Blue);
//
//  auto monster3 = monster2->test.AsMonster();
//  TEST_NOTNULL(monster3);
//  TEST_EQ_STR(monster3->name.c_str(), "Fred");
//
//  auto &vecofstrings = monster2->testarrayofstring;
//  TEST_EQ(vecofstrings.size(), 4U);
//  TEST_EQ_STR(vecofstrings[0].c_str(), "bob");
//  TEST_EQ_STR(vecofstrings[1].c_str(), "fred");
//
//  auto &vecofstrings2 = monster2->testarrayofstring2;
//  TEST_EQ(vecofstrings2.size(), 2U);
//  TEST_EQ_STR(vecofstrings2[0].c_str(), "jane");
//  TEST_EQ_STR(vecofstrings2[1].c_str(), "mary");
//
//  auto &vecoftables = monster2->testarrayoftables;
//  TEST_EQ(vecoftables.size(), 3U);
//  TEST_EQ_STR(vecoftables[0]->name.c_str(), "Barney");
//  TEST_EQ(vecoftables[0]->hp, 1000);
//  TEST_EQ_STR(vecoftables[1]->name.c_str(), "Fred");
//  TEST_EQ_STR(vecoftables[2]->name.c_str(), "Wilma");
//
//  auto &tests = monster2->test4;
//  TEST_EQ(tests[0].a(), 10);
//  TEST_EQ(tests[0].b(), 20);
//  TEST_EQ(tests[1].a(), 30);
//  TEST_EQ(tests[1].b(), 40);
//}
//
// Prefix a FlatBuffer with a size field.
#[test]
fn test_size_prefixed_buffer() {
    // Create size prefixed buffer.
    let mut b = flatbuffers::FlatBufferBuilder::new();
    let args = &MyGame::Example::MonsterArgs{
        mana: 200,
        hp: 300,
        name: Some(b.create_string("bob")),
        ..Default::default()
    };
    let mon = MyGame::Example::CreateMonster(&mut b, &args);
    b.finish_size_prefixed(mon, None);

    // Access it.
    let buf = b.get_active_buf_slice();
    let m = flatbuffers::get_size_prefixed_root::<MyGame::Example::Monster>(buf);
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

    let test_values_max: isize = 11;
    let max_fields_per_object: flatbuffers::VOffsetT = 20;
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
            let choice = lcg.next() % (test_values_max as u64);

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

    // Embrace RNG determinism:
    lcg.reset();

    // Test that all objects we generated are readable and return the
    // expected values. We generate random objects in the same order
    // so this is deterministic:
    for i in 0..(num_fuzz_objects as usize) {
        let table = {
            let buf = builder.get_buf_slice();
            let loc = buf.len() as flatbuffers::UOffsetT - objects[i];
            flatbuffers::Table::new(buf, loc as usize)
        };

        let fields_per_object = (lcg.next() % (max_fields_per_object as u64)) as flatbuffers::VOffsetT;
        for j in 0..fields_per_object {
            let choice = lcg.next() % (test_values_max as u64);

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

    // Assert that we tested all the fuzz cases, at least 5% each:
    let min_tests_per_choice = values_generated / 20;
    assert!(values_generated > 0);
    assert!(min_tests_per_choice > 0);
    for i in 0..test_values_max as u64 {
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

#[test]
fn generated_code_creates_example_data_that_is_accessible_and_correct() {
    let b = &mut flatbuffers::FlatBufferBuilder::new();
    create_serialized_example_with_generated_code(b);
    let buf = b.get_active_buf_slice();
    serialized_example_is_accessible_and_correct(&buf[..], true, false).unwrap();
}

#[test]
fn library_code_creates_example_data_that_is_accessible_and_correct() {
    let b = &mut flatbuffers::FlatBufferBuilder::new();
    create_serialized_example_with_generated_code(b);
    let buf = b.get_active_buf_slice();
    serialized_example_is_accessible_and_correct(&buf[..], true, false).unwrap();
}

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
#[test]
fn go_wire_example_data_is_accessible_and_correct() {
    let filename = "../monsterdata_go_wire.mon";
    let mut f = match std::fs::File::open(filename) {
        Ok(f) => { f }
        Err(_) => {
            println!("missing go file, deal with this later");
            return;
        }
    };
    let buf = load_file(filename);
    serialized_example_is_accessible_and_correct(&buf[..], true, false).unwrap();
}
#[test]
fn python_wire_example_data_is_accessible_and_correct() {
    let buf = load_file("../monsterdata_python_wire.mon");
    serialized_example_is_accessible_and_correct(&buf[..], false, false).unwrap();
}

#[test]
fn test_creation_and_reading_of_nested_flatbuffer_using_generated_code() {
    let b0 = {
        let mut b0 = flatbuffers::FlatBufferBuilder::new();
        let args = MyGame::Example::MonsterArgs{
            hp: 123,
            name: Some(b0.create_string("foobar")),
            ..Default::default()
        };
        let mon = MyGame::Example::CreateMonster(&mut b0, &args);
        MyGame::Example::FinishMonsterBuffer(&mut b0, mon);
        b0
    };

    let b1 = {
        let mut b1 = flatbuffers::FlatBufferBuilder::new();
        let args = MyGame::Example::MonsterArgs{
            testnestedflatbuffer: Some(b1.create_vector_of_scalars::<u8>(b0.get_active_buf_slice())),
            ..Default::default()
        };
        let mon = MyGame::Example::CreateMonster(&mut b1, &args);
        MyGame::Example::FinishMonsterBuffer(&mut b1, mon);
        b1
    };


    let m = MyGame::Example::GetRootAsMonster(b1.get_active_buf_slice());

    assert!(m.testnestedflatbuffer().is_some());
    assert_eq!(m.testnestedflatbuffer().unwrap(), b0.get_active_buf_slice());

    println!("nested buf: {:?}", m.testnestedflatbuffer().unwrap());

    let m2_a = MyGame::Example::GetRootAsMonster(m.testnestedflatbuffer().unwrap());
    assert_eq!(m2_a.hp(), 123);
    assert_eq!(m2_a.name(), Some("foobar"));


    assert!(m.testnestedflatbuffer_nested_flatbuffer().is_some());
    let m2_b = m.testnestedflatbuffer_nested_flatbuffer().unwrap();

    assert_eq!(m2_b.hp(), 123);
    assert_eq!(m2_b.name(), Some("foobar"));
}

#[test]
#[ignore] // we don't have a gold example of testnestedflatbuffer
fn test_reading_of_gold_nested_flatbuffer_using_generated_code() {
    let data = load_file("../monsterdata_test.mon");
    let m = MyGame::Example::GetRootAsMonster(&data[..]);

    assert!(m.testnestedflatbuffer().is_some());

    let m2_a = MyGame::Example::GetRootAsMonster(m.testnestedflatbuffer().unwrap());
    assert_eq!(m2_a.name(), Some("NestedMonster"));

    assert!(m.testnestedflatbuffer_nested_flatbuffer().is_some());
    let m2_b = m.testnestedflatbuffer_nested_flatbuffer().unwrap();

    assert_eq!(m2_b.name(), Some("NestedMonster"));
}

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
        let buf = b.get_active_buf_slice();
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
        let buf = b.get_active_buf_slice();
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
        let buf = b.get_active_buf_slice();
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
mod test_follow_impls {
    extern crate flatbuffers;
    use flatbuffers::Follow;
    use flatbuffers::field_index_to_field_offset as fi2fo;

    #[test]
    fn test_offset_to_ref_u8() {
        let vec: Vec<u8> = vec![255, 3];
        let fs: flatbuffers::FollowStart<&u8> = flatbuffers::FollowStart::new();
        assert_eq!(*fs.self_follow(&vec[..], 1), 3);
    }

    #[test]
    fn test_offset_to_u8() {
        let vec: Vec<u8> = vec![255, 3];
        let fs: flatbuffers::FollowStart<u8> = flatbuffers::FollowStart::new();
        assert_eq!(fs.self_follow(&vec[..], 1), 3);
    }

    #[test]
    fn test_offset_to_ref_u16() {
        let vec: Vec<u8> = vec![255, 255, 3, 4];
        let fs: flatbuffers::FollowStart<&u16> = flatbuffers::FollowStart::new();
        assert_eq!(*fs.self_follow(&vec[..], 2), 1027);
    }

    #[test]
    fn test_offset_to_u16() {
        let vec: Vec<u8> = vec![255, 255, 3, 4];
        let fs: flatbuffers::FollowStart<u16> = flatbuffers::FollowStart::new();
        assert_eq!(fs.self_follow(&vec[..], 2), 1027);
    }

    #[test]
    fn test_offset_to_f32() {
        let vec: Vec<u8> = vec![255, 255, 255, 255, /* start of value */ 208, 15, 73, 64];
        let fs: flatbuffers::FollowStart<&f32> = flatbuffers::FollowStart::new();
        assert_eq!(fs.self_follow(&vec[..], 4), &3.14159);
    }

    #[test]
    fn test_offset_to_string() {
        let vec: Vec<u8> = vec![255,255,255,255, 3, 0, 0, 0, 'f' as u8, 'o' as u8, 'o' as u8, 0];
        let off: flatbuffers::FollowStart<&str> = flatbuffers::FollowStart::new();
        assert_eq!(off.self_follow(&vec[..], 4), "foo");
    }

    #[test]
    fn test_offset_to_byte_string() {
        let vec: Vec<u8> = vec![255, 255, 255, 255, 3, 0, 0, 0, 1, 2, 3, 0];
        let off: flatbuffers::FollowStart<&[u8]> = flatbuffers::FollowStart::new();
        assert_eq!(off.self_follow(&vec[..], 4), &vec![1, 2, 3][..]);
    }

    #[test]
    fn test_offset_to_slice_of_u16() {
        let vec: Vec<u8> = vec![255, 255, 255, 255, 2, 0, 0, 0, 1, 2, 3, 4];
        let off: flatbuffers::FollowStart<&[u16]> = flatbuffers::FollowStart::new();
        assert_eq!(off.self_follow(&vec[..], 4), &vec![513, 1027][..]);
    }

    #[test]
    fn test_offset_to_vector_of_u16() {
        let vec: Vec<u8> = vec![255, 255, 255, 255, 2, 0, 0, 0, 1, 2, 3, 4];
        let off: flatbuffers::FollowStart<flatbuffers::Vector<u16>> = flatbuffers::FollowStart::new();
        assert_eq!(off.self_follow(&vec[..], 4).len(), 2);
        assert_eq!(off.self_follow(&vec[..], 4).get(0), 513);
        assert_eq!(off.self_follow(&vec[..], 4).get(1), 1027);
    }

    #[test]
    fn test_offset_to_struct() {
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
    fn test_vector_of_offset_to_string_elements() {
        let buf: Vec<u8> = vec![/* vec len */ 1, 0, 0, 0, /* offset to string */ 4, 0, 0, 0, /* str length */ 3, 0, 0, 0, 'f' as u8, 'o' as u8, 'o' as u8, 0];
        let s: flatbuffers::FollowStart<flatbuffers::Vector<flatbuffers::ForwardsU32Offset<&str>>> = flatbuffers::FollowStart::new();
        assert_eq!(s.self_follow(&buf[..], 0).len(), 1);
        assert_eq!(s.self_follow(&buf[..], 0).get(0), "foo");
    }

    #[test]
    fn test_slice_of_struct_elements() {
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
    fn test_vector_of_struct_elements() {
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
    fn test_root_to_empty_table() {
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
    fn test_table_get_slot_scalar_u8() {
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
    fn test_table_get_slot_scalar_u8_default_via_vtable_len() {
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
    fn test_table_get_slot_scalar_u8_default_via_vtable_zero() {
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
    fn test_table_get_slot_string_multiple_types() {
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
    fn test_table_get_slot_string_multiple_types_default_via_vtable_len() {
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
    fn test_table_get_slot_string_multiple_types_default_via_vtable_zero() {
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
    fn test_1_basic_numbers() {
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
    fn test_1b_bigger_numbers() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        b.push_element_scalar(0x1122334455667788u64);
        check(&b, &[0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]);
    }

    #[test]
    fn test_2_1xbyte_vector() {
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
    fn test_3_2xbyte_vector() {
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
    fn test_3b_11xbyte_vector_matches_builder_size() {
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
    fn test_4_1xuint16_vector() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        b.start_vector(flatbuffers::SIZE_U16, 1);
        check(&b, &[0, 0]); // align to 4bytes
        b.push_element_scalar(1u16);
        check(&b, &[1, 0, 0, 0]);
        b.end_vector::<&u16>(1);
        check(&b, &[1, 0, 0, 0, 1, 0, 0, 0]); // padding
    }

    #[test]
    fn test_5_2xuint16_vector() {
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
    fn test_6_create_string() {
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
    fn test_6b_create_string_unicode() {
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
    fn test_6c_create_byte_string() {
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
    fn test_7_empty_vtable() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let off0 = b.start_table(0);
        //assert_eq!(4, off0.value());
        check(&b, &[]);
        let off1 = b.end_table(off0);
        //assert_eq!(4, off1.value());
        check(&b, &[4, 0, // vtable length
                    4, 0, // length of table including vtable offset
                    4, 0, 0, 0]); // offset for start of vtable
    }

    #[test]
    fn test_8_vtable_with_one_true_bool() {
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
    fn test_9_vtable_with_one_default_bool() {
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
    fn test_10_vtable_with_one_int16() {
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
    fn test_11_vtable_with_two_int16() {
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
    fn test_12_vtable_with_int16_and_bool() {
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
    fn test_12b_vtable_with_empty_vector() {
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
    fn test_12c_vtable_with_empty_vector_of_byte_and_some_scalars() {
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
    fn test_13_vtable_with_1_int16_and_2_vector_of_i16() {
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
    fn test_14_vtable_with_1_struct_of_int8_and_int16_and_int32() {
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
    fn test_15_vtable_with_1_vector_of_2_struct_2_int8() {
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
    fn test_16_table_with_some_elements() {
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
    fn test_17_one_unfinished_table_and_one_finished_table() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        {
            let off = b.start_table(2);
            b.push_slot_scalar(fi2fo(0), 33i8, 0);
            b.push_slot_scalar(fi2fo(1), 44i8, 0);
            let off2 = b.end_table(off);
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
    fn test_18_a_bunch_of_bools() {
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
    fn test_19_three_bools() {
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
    fn test_20_some_floats() {
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
    fn test_21_vtable_defaults() {
        let mut b = flatbuffers::FlatBufferBuilder::new();
        let off = b.start_table(3);
        b.push_slot_scalar::<i8>(fi2fo(0), 1, 1);
        b.push_slot_scalar::<i8>(fi2fo(1), 3, 2);
        b.push_slot_scalar::<i8>(fi2fo(2), 3, 3);
        let table_end = b.end_table(off);
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
    fn test_22_root() {
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
    fn test_23_varied_slots_and_root() {
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
