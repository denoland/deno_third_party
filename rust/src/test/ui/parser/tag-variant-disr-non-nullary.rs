// compile-flags: -Z parse-only

//error-pattern: discriminator values can only be used with a field-less enum

enum color {
    red = 0xff0000,
    green = 0x00ff00,
    blue = 0x0000ff,
    black = 0x000000,
    white = 0xffffff,
    other (str),
}
