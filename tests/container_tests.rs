use rundmc::container::parse_signal;
use nix::sys::signal::Signal;

#[test]
fn test_parse_signal_by_name() {
    assert_eq!(parse_signal("TERM").unwrap(), Signal::SIGTERM);
    assert_eq!(parse_signal("SIGTERM").unwrap(), Signal::SIGTERM);
    assert_eq!(parse_signal("KILL").unwrap(), Signal::SIGKILL);
    assert_eq!(parse_signal("SIGKILL").unwrap(), Signal::SIGKILL);
}

#[test]
fn test_parse_signal_by_number() {
    assert_eq!(parse_signal("15").unwrap(), Signal::SIGTERM);
    assert_eq!(parse_signal("9").unwrap(), Signal::SIGKILL);
    assert_eq!(parse_signal("2").unwrap(), Signal::SIGINT);
}

#[test]
fn test_parse_invalid_signal() {
    assert!(parse_signal("INVALID").is_err());
    assert!(parse_signal("999").is_err());
}
