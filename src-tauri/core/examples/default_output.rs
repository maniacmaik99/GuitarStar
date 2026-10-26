//! Prints the Windows default playback device as GuitarStar sees it.
//! Usage: cargo run -p guitarstar_core --example default_output

fn main() {
    println!("{:?}", guitarstar_core::audio::default_output());
}
