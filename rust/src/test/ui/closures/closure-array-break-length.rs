fn main() {
    |_: [_; continue]| {}; //~ ERROR: `continue` outside of loop

    while |_: [_; continue]| {} {} //~ ERROR: `continue` outside of loop

    while |_: [_; break]| {} {} //~ ERROR: `break` outside of loop
}
