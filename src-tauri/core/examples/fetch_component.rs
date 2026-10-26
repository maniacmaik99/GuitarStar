//! Downloads the newest release of a component into a scratch folder, to check
//! that the release layout still matches what GuitarStar expects.
//! Usage: cargo run -p guitarstar_core --example fetch_component -- rs_asio <dir>

use guitarstar_core::components;

fn main() {
    let mut args = std::env::args().skip(1);
    let id = args.next().unwrap_or_else(|| "rs_asio".into());
    let dir = std::path::PathBuf::from(args.next().expect("target folder required"));
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        let def = components::find(&id).unwrap();
        let http = components::http_client();
        let releases = components::releases(&http, def).await.unwrap();
        let newest = releases.first().expect("no release found");
        println!("{} releases, newest {} ({:?})", releases.len(), newest.version, newest.digest);
        let path = components::download(&http, &dir, def, newest).await.unwrap();
        for e in std::fs::read_dir(&path).unwrap() {
            let e = e.unwrap();
            println!("  {} ({} bytes)", e.file_name().to_string_lossy(), e.metadata().unwrap().len());
        }
    });
}
