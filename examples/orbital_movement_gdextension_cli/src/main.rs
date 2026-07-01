use orbital_movement_gdextension::greet;

fn main() {
    match greet("native demo") {
        Ok(message) => println!("{message}"),
        Err(error) => eprintln!("error: {error}"),
    }
}
