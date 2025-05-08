
#[test]
fn test_runner() {
    let config = "./spec";
    let output = std::process::Command::new("cargo").args(["run", "--", "run", config]).output().expect("Failed to run test");
    println!("{:?}", output.status);
}
