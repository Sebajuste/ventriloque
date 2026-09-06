// Une date lisible, sans dependance de plus : le temps systeme suffit pour dire quel jour.
//
// Sert a horodater l'installation et la fabrication d'un paquet. Rien ici ne demande l'heure,
// le fuseau ni la duree : ajouter `chrono` pour cela aurait coute une caisse pour un clou.

/// Aujourd'hui, en `AAAA-MM-JJ` (UTC).
pub fn today() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    from_unix_days((seconds / 86_400) as i64)
}

fn leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn from_unix_days(days: i64) -> String {
    let (mut year, mut rest) = (1970i64, days);
    loop {
        let length = if leap(year) { 366 } else { 365 };
        if rest < length {
            break;
        }
        rest -= length;
        year += 1;
    }
    let months = [31, if leap(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 0usize;
    while month < 12 && rest >= months[month] {
        rest -= months[month];
        month += 1;
    }
    format!("{year:04}-{:02}-{:02}", month + 1, rest + 1)
}

#[cfg(test)]
mod tests {
    use super::from_unix_days;

    #[test]
    fn le_premier_jour_de_l_epoque_est_le_premier_janvier_1970() {
        assert_eq!(from_unix_days(0), "1970-01-01");
    }

    // Le 29 fevrier est le seul jour que se trompe un calendrier qui ignore les bissextiles.
    #[test]
    fn les_annees_bissextiles_sont_comptees() {
        assert_eq!(from_unix_days(59), "1970-03-01");
        assert_eq!(from_unix_days(789), "1972-02-29");
        assert_eq!(from_unix_days(790), "1972-03-01");
    }

    // 2000 est bissextile (divisible par 400), 1900 ne l'est pas -- la regle que les
    // implementations naives ratent.
    #[test]
    fn l_an_2000_est_bissextile() {
        assert_eq!(from_unix_days(11_016), "2000-02-29");
        assert_eq!(from_unix_days(20_337), "2025-09-06");
    }
}
