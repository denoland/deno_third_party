extern crate flatbuffers;

extern crate rust_usage_test;
use rust_usage_test::monster_test_generated::my_game;

#[macro_use]
extern crate bencher;

use bencher::Bencher;

fn read_canonical_buffer(bench: &mut Bencher) {
    let owned_data = {
        let mut builder = &mut flatbuffers::FlatBufferBuilder::new();
        create_serialized_example_with_generated_code(&mut builder);
        builder.finished_bytes().to_vec()
    };
    let data = &owned_data[..];
    let n = data.len() as u64;
    bench.iter(|| {
        read_serialized_example_with_generated_code(data);
    });
    bench.bytes = n;
}

fn create_canonical_buffer(bench: &mut Bencher) {
    let mut builder = &mut flatbuffers::FlatBufferBuilder::new();
    // warmup
    create_serialized_example_with_generated_code(&mut builder);
    let n = builder.finished_bytes().len() as u64;
    builder.reset();

    bench.iter(|| {
        create_serialized_example_with_generated_code(&mut builder);
        builder.reset();
    });

    bench.bytes = n;
}

#[inline(always)]
fn create_serialized_example_with_generated_code(builder: &mut flatbuffers::FlatBufferBuilder) {
    let t0_name = builder.create_string("Barney");
    let t1_name = builder.create_string("Fred");
    let t2_name = builder.create_string("Wilma");
    let t0 = my_game::example::Monster::create(builder, &my_game::example::MonsterArgs{
        hp: 1000,
        name: Some(t0_name),
        ..Default::default()
    });
    let t1 = my_game::example::Monster::create(builder, &my_game::example::MonsterArgs{
        name: Some(t1_name),
        ..Default::default()
    });
    let t2 = my_game::example::Monster::create(builder, &my_game::example::MonsterArgs{
        name: Some(t2_name),
        ..Default::default()
    });
    let s0 = builder.create_string("test1");
    let s1 = builder.create_string("test2");
    let mon = {
        let fred_name = builder.create_string("Fred");
        let inventory = builder.create_vector_of_scalars::<u8>(&[0, 1, 2, 3, 4]);
        let test4 = builder.create_vector_of_structs(&[my_game::example::Test::new(10, 20),
                                                       my_game::example::Test::new(30, 40)]);
        let pos = my_game::example::Vec3::new(1.0, 2.0, 3.0, 3.0, my_game::example::Color::Green, my_game::example::Test::new(5i16, 6i8));
        let args = my_game::example::MonsterArgs{
            hp: 80,
            mana: 150,
            name: Some(builder.create_string("MyMonster")),
            pos: Some(&pos),
            test_type: my_game::example::Any::Monster,
            test: Some(my_game::example::Monster::create(builder, &my_game::example::MonsterArgs{
                name: Some(fred_name),
                ..Default::default()
            }).as_union_value()),
            inventory: Some(inventory),
            test4: Some(test4),
            testarrayofstring: Some(builder.create_vector_of_reverse_offsets(&[s0, s1])),
            testarrayoftables: Some(builder.create_vector_of_reverse_offsets(&[t0, t1, t2])),
            ..Default::default()
        };
        my_game::example::Monster::create(builder, &args)
    };
    my_game::example::finish_monster_buffer(builder, mon);

    // make it do some work
    //if builder.finished_bytes().len() == 0 { panic!("bad benchmark"); }
}

#[inline(always)]
fn maybe_blackbox<T>(t: T) -> T {
    bencher::black_box(t)
    //t
}

#[inline(always)]
fn read_serialized_example_with_generated_code(bytes: &[u8]) {
    let m = my_game::example::get_root_as_monster(bytes);
    maybe_blackbox(m.hp());
    maybe_blackbox(m.mana());
    maybe_blackbox(m.name());
    let pos = m.pos().unwrap();
    maybe_blackbox(pos.x());
    maybe_blackbox(pos.y());
    maybe_blackbox(pos.z());
    maybe_blackbox(pos.test1());
    maybe_blackbox(pos.test2());
    let pos_test3 = pos.test3();
    maybe_blackbox(pos_test3.a());
    maybe_blackbox(pos_test3.b());
    maybe_blackbox(m.test_type());
    let table2 = m.test().unwrap();
    let monster2 = my_game::example::Monster::init_from_table(table2);
    maybe_blackbox(monster2.name());
    maybe_blackbox(m.inventory());
    maybe_blackbox(m.test4());
    let testarrayoftables = m.testarrayoftables().unwrap();
    maybe_blackbox(testarrayoftables.get(0).hp());
    maybe_blackbox(testarrayoftables.get(0).name());
    maybe_blackbox(testarrayoftables.get(1).name());
    maybe_blackbox(testarrayoftables.get(2).name());
    let testarrayofstring = m.testarrayofstring().unwrap();
    maybe_blackbox(testarrayofstring.get(0));
    maybe_blackbox(testarrayofstring.get(1));
}

benchmark_group!(benches, read_canonical_buffer, create_canonical_buffer);
benchmark_main!(benches);
