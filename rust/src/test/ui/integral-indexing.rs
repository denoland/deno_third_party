pub fn main() {
    let v: Vec<isize> = vec![0, 1, 2, 3, 4, 5];
    let s: String = "abcdef".to_string();
    v[3_usize];
    v[3];
    v[3u8];  //~ERROR : std::slice::SliceIndex<[isize]>` is not satisfied
    v[3i8];  //~ERROR : std::slice::SliceIndex<[isize]>` is not satisfied
    v[3u32]; //~ERROR : std::slice::SliceIndex<[isize]>` is not satisfied
    v[3i32]; //~ERROR : std::slice::SliceIndex<[isize]>` is not satisfied
    s.as_bytes()[3_usize];
    s.as_bytes()[3];
    s.as_bytes()[3u8];  //~ERROR : std::slice::SliceIndex<[u8]>` is not satisfied
    s.as_bytes()[3i8];  //~ERROR : std::slice::SliceIndex<[u8]>` is not satisfied
    s.as_bytes()[3u32]; //~ERROR : std::slice::SliceIndex<[u8]>` is not satisfied
    s.as_bytes()[3i32]; //~ERROR : std::slice::SliceIndex<[u8]>` is not satisfied
}
