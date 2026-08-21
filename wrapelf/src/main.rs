// Wraps a raw guest ELF with the standard v1compat kernel into a RISC Zero
// ProgramBinary — the same container `cargo risczero build` emits as `.bin`.
fn main() {
    let mut args = std::env::args().skip(1);
    let input = args.next().expect("usage: wrapelf <in.elf> <out.bin>");
    let output = args.next().expect("usage: wrapelf <in.elf> <out.bin>");
    let elf = std::fs::read(&input).expect("read elf");
    let binary = risc0_binfmt::ProgramBinary::new(&elf, risc0_zkos_v1compat::V1COMPAT_ELF);
    let bytes = binary.encode();
    let image_id = binary.compute_image_id().expect("image id");
    std::fs::write(&output, &bytes).expect("write bin");
    println!("{output}: {} bytes, image id {image_id}", bytes.len());
}
