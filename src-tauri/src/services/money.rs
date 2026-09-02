// Money, written the way an Indian document writes it.
//
// One implementation, because the invoice and the legal notice must not group
// the same figure differently. A GST invoice reading "1,55,760.00" beside a
// demand notice reading "155,760.00" is two documents from one firm that do not
// agree with each other, and it is the sort of thing opposing counsel notices
// before anyone here does.

/// Format an amount with Indian digit grouping: 600000.0 → "6,00,000.00".
///
/// Three digits, then twos — the lakh/crore convention, not the Western
/// thousands one. `amount_in_words` already speaks in lakhs, so a figure
/// grouped as "600,000.00" beside the words "Six Lakh" reads as a mistake on a
/// GST invoice.
pub fn format_inr(amount: f64) -> String {
    let negative = amount < 0.0;
    let text = format!("{:.2}", amount.abs());
    let (whole, fraction) = text.split_once('.').unwrap_or((text.as_str(), "00"));

    let grouped = if whole.len() <= 3 {
        whole.to_owned()
    } else {
        let (lead, last_three) = whole.split_at(whole.len() - 3);
        // The leading part is grouped in twos, read from the right.
        let mut pairs: Vec<String> = lead
            .as_bytes()
            .rchunks(2)
            .map(|c| String::from_utf8_lossy(c).into_owned())
            .collect();
        pairs.reverse();
        format!("{},{last_three}", pairs.join(","))
    };

    format!("{}{grouped}.{fraction}", if negative { "-" } else { "" })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Indian grouping is three digits then twos, not Western thousands.
    /// The invoice prints these next to `amount_in_words`, which already says
    /// "Lakh" — grouped the Western way the two read as contradicting each other.
    #[test]
    fn amounts_use_indian_digit_grouping() {
        assert_eq!(format_inr(0.0),         "0.00");
        assert_eq!(format_inr(999.5),       "999.50");
        assert_eq!(format_inr(1_000.0),     "1,000.00");
        assert_eq!(format_inr(23_600.0),    "23,600.00");
        assert_eq!(format_inr(100_000.0),   "1,00,000.00");
        assert_eq!(format_inr(600_000.0),   "6,00,000.00");
        assert_eq!(format_inr(1_234_567.0), "12,34,567.00");
        // One crore.
        assert_eq!(format_inr(10_000_000.0), "1,00,00,000.00");
    }
    #[test]
    fn a_credit_keeps_its_sign() {
        assert_eq!(format_inr(-5_00_000.0), "-5,00,000.00");
    }
    #[test]
    fn paise_are_always_shown() {
        // A GST invoice states paise even when they are zero.
        assert_eq!(format_inr(8_000.0),  "8,000.00");
        assert_eq!(format_inr(8_000.05), "8,000.05");
        assert_eq!(format_inr(8_000.5),  "8,000.50");
    }
    /// Grouping is presentation only. It must never disagree with the figure
    /// the invoice was calculated from.
    #[test]
    fn grouping_does_not_change_the_value() {
        for amount in [0.0_f64, 1.0, 999.99, 1_00_000.0, 82_600.01, 1_23_45_678.9] {
            let stripped: String = format_inr(amount).chars().filter(|c| *c != ',').collect();
            assert_eq!(
                stripped,
                format!("{amount:.2}"),
                "grouping altered the amount {amount}"
            );
        }
    }
}
