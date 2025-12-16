mod doc;

use doc::Document;
use std::fs::File;
use std::io;

fn main() -> io::Result<()> {

    println!("Main isn't really doing anything right now, run the server and client instead");


    // let mut d = Document::new();
    //
    // d.insert(0, "Hello")?;
    // d.insert(5, " world")?;
    // d.delete(6, 5)?;
    // d.insert(6, "Rust")?;
    //
    // println!("Current text: {}", d.text());
    // println!("Ops so far: {:?}", d.ops());
    //
    // let f = File::create("doc.bin")?;
    // d.save(f)?;
    // println!("Saved to doc.bin");
    //
    // let f2 = File::open("doc.bin")?;
    // let loaded = Document::load(f2)?;
    //
    // println!("Loaded text: {}", loaded.text());
    // println!("Loaded ops: {:?}", loaded.ops());

    Ok(())
}
