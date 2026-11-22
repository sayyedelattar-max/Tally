//! Text normalisation used for both indexing and querying.
//!
//! Goal: a viewer typing `المرفا` must find `المرفأ`, and `٢٠١` must find channel 201.
//! Both sides go through [`normalize`], so the index never needs to know about
//! spelling variants.

/// Lower-case, strip Arabic diacritics and tatweel, unify letter variants,
/// map Arabic-Indic digits to ASCII, and collapse punctuation to single spaces.
pub fn normalize(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut pending_space = false;

    for ch in input.chars() {
        let mapped = match ch {
            // tashkeel, Quranic marks, superscript alef, tatweel: drop entirely
            '\u{064B}'..='\u{065F}' | '\u{0670}' | '\u{0640}' | '\u{06D6}'..='\u{06ED}' => continue,
            // alef variants -> bare alef
            'أ' | 'إ' | 'آ' | 'ٱ' => 'ا',
            // alef maqsura -> ya, ta marbuta -> ha
            'ى' => 'ي',
            'ة' => 'ه',
            // hamza on carriers -> carrier
            'ؤ' => 'و',
            'ئ' => 'ي',
            // Arabic-Indic and Persian digits
            '٠'..='٩' => char::from(b'0' + (ch as u32 - '٠' as u32) as u8),
            '۰'..='۹' => char::from(b'0' + (ch as u32 - '۰' as u32) as u8),
            c if c.is_alphanumeric() => c,
            _ => {
                pending_space = true;
                continue;
            }
        };
        if pending_space && !out.is_empty() {
            out.push(' ');
        }
        pending_space = false;
        out.extend(mapped.to_lowercase());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn latin_is_lowercased_and_punctuation_collapsed() {
        assert_eq!(normalize("  Oasis Evenings · Ep 14 "), "oasis evenings ep 14");
    }

    #[test]
    fn arabic_letter_variants_unify() {
        assert_eq!(normalize("المرفأ"), normalize("المرفا"));
        assert_eq!(normalize("إعلانات"), normalize("اعلانات"));
        assert_eq!(normalize("مدرسة"), normalize("مدرسه"));
    }

    #[test]
    fn diacritics_and_tatweel_are_dropped() {
        assert_eq!(normalize("مَرْحَبًا"), "مرحبا");
        assert_eq!(normalize("الـــمرفأ"), "المرفا");
    }

    #[test]
    fn arabic_indic_digits_become_ascii() {
        assert_eq!(normalize("ميريديان سبورت ٢٠١"), "ميريديان سبورت 201");
        assert_eq!(normalize("CH ۱۰۱"), "ch 101");
    }
}
