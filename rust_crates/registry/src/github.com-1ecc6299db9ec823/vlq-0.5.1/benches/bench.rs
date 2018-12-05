#![feature(test)]

extern crate test;
extern crate vlq;

#[bench]
fn bench_decode_single_digit(b: &mut test::Bencher) {
    let mut input = vec![];

    for x in -15..16 {
        vlq::encode(x, &mut input).unwrap();
    }

    {
        let mut input = input.iter().cloned();
        for x in -15..16 {
            assert_eq!(x, vlq::decode(&mut input).unwrap());
        }
    }

    b.iter(|| {
        let mut input = input.iter().cloned();
        for _ in -15..16 {
            let _ = test::black_box(vlq::decode(&mut input));
        }
    })
}

#[bench]
fn bench_decode_many_digits(b: &mut test::Bencher) {
    let mut input = vec![];

    for x in -150..160 {
        vlq::encode(x, &mut input).unwrap();
    }

    {
        let mut input = input.iter().cloned();
        for x in -150..160 {
            assert_eq!(x, vlq::decode(&mut input).unwrap());
        }
    }

    b.iter(|| {
        let mut input = input.iter().cloned();
        for _ in -150..160 {
            let _ = test::black_box(vlq::decode(&mut input));
        }
    })
}
