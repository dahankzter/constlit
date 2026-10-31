use constlit::{CronSpec, cron};

#[test]
fn cron_in_const() {
    const EVERY_5: CronSpec = cron!("*/5 * * * *");
    assert_eq!(EVERY_5.as_str(), "*/5 * * * *");
    assert_eq!(EVERY_5.to_string(), "*/5 * * * *");
    assert!(EVERY_5.matches(55, 23, 31, 12, 0));
    assert!(!EVERY_5.matches(56, 23, 31, 12, 0));
    assert_eq!(EVERY_5.minutes(), 0x0084_2108_4210_8421);
    assert_ne!(cron!("@daily"), cron!("0 0 * * *"));
    assert_eq!(cron!("@daily").hours(), cron!("0 0 * * *").hours());
}
