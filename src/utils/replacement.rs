use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    // Regular expressions
    static ref RE_REMOVE_NUMBER_BEFORE_HYPHEN: Regex = Regex::new(r"\b(\d+)(\s-\s\w+)\b").unwrap();
    static ref RE_SPACE: Regex = Regex::new(r"\s+").unwrap();
    static ref RE_DOUBLE_SPACE: Regex = Regex::new(r"\s{2,}").unwrap();
    static ref RE_NUMBER: Regex = Regex::new(r"\d+\.\d+\.\d+").unwrap();
    static ref RE_ELLIPSIS: Regex = Regex::new(r"\.{3,}").unwrap();
    static ref RE_COMMA: Regex = Regex::new(r",\s*,").unwrap();
    static ref RE_REMOVE_COMMA_AT_END: Regex = Regex::new(r",$").unwrap();
    static ref RE_CHAPTER_NUMBER: Regex = Regex::new(r"\d+\.\d+").unwrap();
    static ref RE_EMPTY_PARENTHESES: Regex = Regex::new(r"\(\s*\)").unwrap();
    static ref RE_BRACKETS: Regex = Regex::new(r"[()]").unwrap();
    static ref RE_NON_ALPHABETIC: Regex = Regex::new(r"^[\p{P}\p{Z}\p{N}]+$").unwrap();
    static ref RE_REPLACE_O: Regex = Regex::new(r"^o ").unwrap();
    static ref RE_REPLACE_SECTION: Regex = Regex::new(r"^§ ").unwrap();
    static ref RE_REPLACE_DASH: Regex = Regex::new(r"^- ?").unwrap();
    static ref RE_REPLACE_BULLET_POINT: Regex = Regex::new(r"^· ").unwrap();
    static ref RE_REMOVE_BRACKET_AT_END: Regex = Regex::new(r"\[$").unwrap();
    static ref RE_ZERO_TO_TEN_AT_START: Regex = Regex::new(r"^(0[0-9]|10)\b").unwrap();
    static ref RE_COLON_SPACE_AT_START: Regex = Regex::new(r"^: ").unwrap();
    static ref RE_DOT_AT_END: Regex = Regex::new(r"\.$").unwrap();

    // Extra words
    static ref RE_CHAPTER: Regex = Regex::new(r"Kap\.\s*\d+").unwrap();
    static ref RE_DAY: Regex = Regex::new(r"Tag \d+").unwrap();
    static ref RE_CSS_FOLIEN: Regex = Regex::new(r"\b\d+\sCSS_Teil\s\d+\sFolien\s\d+\sbis\s\d+\b").unwrap();
    static ref RE_HTML_FOLIEN: Regex = Regex::new(r"\bHTML_Teil\s\d+\sFolien\s\d+\sbis\s\d+\b").unwrap();
}

pub async fn apply_replacements(description: &str) -> String {
    let mut description = String::from(description);

    let replacements = vec![
        ("1 & 2", " "),
        (", 2, 3", " "),
        ("_Aufg_9_", " "),
        ("1, Kap.", " "),
        ("Kap.", " "),
        ("-Auf_6-1_Lernsituation-6…", " "),
        ("_Aufg_7-Kreuzworträtsel relationales Datenmodell…", " "),
        ("-591_Lehrbuch_Auf_4_", " "),
        ("-590_Lehrbuch_Auf_2_", " "),
        ("siehe Übungsdatei 5.1.2_....", " "),
        ("Einführung, Kap.", " "),
        ("Einzel-/Gruppenarbeit", " "),
        ("Gruppenarbeit und Besprechung", " "),
        ("Inhalte:", "  "),
        ("Tagesinhalte:", " "),
        ("Übungen:", " "),
        ("Prüfungen", " "),
        ("Prüfungen:", " "),
        ("Handlungsaufgabe:", " "),
        ("Grammar:", " "),
        ("IT-Milestone:", " "),
        ("Your English skills:", " "),
        ("Communication:", " "),
        ("Wiederholung des Vortags", " "),
        ("alles Folienpräsentation", " "),
        ("Informationen beschaffen und verwerten", " "),
        ("Lehrgespräche in den folgenden Themen:", " "),
        ("(Lehrgespräche)", " "),
        ("EXCEL", " "),
        ("Lernmethode: Workshop in", " "),
        ("Praktische Übung:", " "),
        ("Lernmethoden:", " "),
        ("Krank", "Selbstlernphase"),
        ("hemen und Lernziele", "Themen und Lernziele")
    ];

    let special_replacements = vec![
        ("\t", " "),
        ("\n", " "),
        (" , ",  ", "),
        (" +", " "),
        ("--", "-"),
        ("- -", "-"),
        (" :", ": "),
        (":-", ": -"),
    ];

    // Apply other replacements synchronously
    for (from, to) in replacements {
        description = description.replace(from, to);
    }

    // Apply special replacements synchronously
    for (from, to) in special_replacements {
        description = description.replace(from, to);
    }

    // Remove numbers before hyphen
    description = RE_REMOVE_NUMBER_BEFORE_HYPHEN.replace_all(&description, "$2").to_string();
    // Replace all spaces with a single space
    description = RE_SPACE.replace_all(&description, " ").to_string();
    // Replace multiple consecutive spaces
    description = RE_DOUBLE_SPACE.replace_all(&description, " ").to_string();
    // Remove (X.X.X) and (X.X)
    description = RE_NUMBER.replace_all(&description, "").to_string();
    // Replace ... with nothing
    description = RE_ELLIPSIS.replace_all(&description, "").to_string();
    // Remove empty parentheses
    description = RE_EMPTY_PARENTHESES.replace_all(&description, "").to_string();
    // Replace multiple consecutive commas
    description = RE_COMMA.replace_all(&description, ", ").to_string();
    // Remove comma at the end of the string
    description = RE_REMOVE_COMMA_AT_END.replace_all(&description, "").to_string();
    // Remove chapter numbers
    description = RE_CHAPTER_NUMBER.replace_all(&description, "").to_string();
    // Remove all brackets
    description = RE_BRACKETS.replace_all(&description, "").to_string();
    // Remove all non-alphabetic characters
    description = RE_NON_ALPHABETIC.replace_all(&description, "").to_string();
    // Replace "o " at the beginning of the string
    description = RE_REPLACE_O.replace_all(&description, "").to_string();
    // Replace "§ " at the beginning of the string
    description = RE_REPLACE_SECTION.replace_all(&description, "").to_string();
    // Replace "- " or "-" at the beginning of the string
    description = RE_REPLACE_DASH.replace_all(&description, "").to_string();
    // Replace "· " at the beginning of the string
    description = RE_REPLACE_BULLET_POINT.replace_all(&description, "").to_string();
    // Remove "[" at the end of the string
    description = RE_REMOVE_BRACKET_AT_END.replace_all(&description, "").to_string();
    // Remove all numbers from 00 to 10 at the start of the string
    description = RE_ZERO_TO_TEN_AT_START.replace_all(&description, "").to_string();
    // Remove all ": " at the start of the string
    description = RE_COLON_SPACE_AT_START.replace_all(&description, "").to_string();
    // Remove "." at the end of the string
    description = RE_DOT_AT_END.replace_all(&description, "").to_string();

    // Remove all chapters
    description = RE_CHAPTER.replace_all(&description, "").to_string();
    // Remove all "Tag X" occurrences
    description = RE_DAY.replace_all(&description, "").to_string();
    // Remove all "6 CSS_Teil 2 Folien 1 bis 50" like occurrences
    description = RE_CSS_FOLIEN.replace_all(&description, "").to_string();
    // Remove all "6 CSS_Teil 2 Folien 1 bis 50" like occurrences
    description = RE_CSS_FOLIEN.replace_all(&description, "").to_string();
    // Remove all "HTML_Teil 2 Folien 1 bis 33" like occurrences
    description = RE_HTML_FOLIEN.replace_all(&description, "").to_string();

    for i in 1..=7 {
        let day = format!("Tag {:02}", i);
        description = description.replace(&day, "");
    }

    return description;
}
