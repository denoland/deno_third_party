fn test(cond: bool) {
    let v;
    while cond {
        v = 3;
        break;
    }
    println!("{}", v); //~ ERROR use of possibly uninitialized variable: `v`
}

fn main() {
    test(true);
}
