fn main() {
    println!("⚙️ DeX Trading Agent Engine initialized!");
    
    // Call the function from the shared workspace crate
    let message = shared::init_message();
    println!("{}", message);
}
