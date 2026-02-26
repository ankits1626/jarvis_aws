use std::fs;

// This function MIGHT fail — notice the Result return type
fn load_gems() -> Result<String, std::io::Error> {
    let contents = fs::read_to_string("gems.txt")?;  // ? = pass error up
    Ok(contents)
}

fn main() {
    match load_gems() {
        Ok(data) => println!("Loaded: {}", data),
        Err(e) => println!("Failed to load: {}", e),
    }
}