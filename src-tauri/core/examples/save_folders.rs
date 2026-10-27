//! Lists the Rocksmith save folders GuitarStar finds (read-only).
//! Usage: cargo run -p guitarstar_core --example save_folders

fn main() {
    for folder in guitarstar_core::saves::find_save_folders() {
        println!("{folder:?}");
    }
}
