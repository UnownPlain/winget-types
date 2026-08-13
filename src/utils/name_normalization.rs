//! Package-name normalization compatible with winget-cli's initial normalization algorithm.

use std::sync::LazyLock;

use icu_casemap::CaseMapper;
use icu_normalizer::ComposingNormalizerBorrowed;
use regex::Regex;

const ASCII_WHITESPACE: &[char] = &[' ', '\u{c}', '\n', '\r', '\t', '\u{b}'];

const LEGAL_ENTITY_SUFFIXES: &str = "ab,ad,ag,aps,as,asa,bv,co,cv,doo,ev,ges,gesmbh,gmbh,inc,kg,ks,ps,llc,lp,ltd,ltda,mbh,nv,plc,sl,pty,pvt,sa,sarl,sc,sca,sp,spa,srl,sro,company,corp,corporation,holding,holdings,incorporated,limited,subsidiary";

const LOCALES: &str = "af-za,am-et,ar-ae,ar-bh,ar-dz,ar-eg,ar-iq,ar-jo,ar-kw,ar-lb,ar-ly,ar-ma,arn-cl,ar-om,ar-qa,ar-sa,ar-sy,ar-tn,ar-ye,as-in,ba-ru,be-by,bg-bg,bn-bd,bn-in,bo-cn,br-fr,ca-es,ca-es-valencia,co-fr,cs-cz,cy-gb,da-dk,de-at,de-ch,de-de,de-li,de-lu,dsb-de,dv-mv,el-gr,en-au,en-bz,en-ca,en-gb,en-ie,en-in,en-jm,en-my,en-nz,en-ph,en-sg,en-tt,en-us,en-za,en-zw,es-ar,es-bo,es-cl,es-co,es-cr,es-do,es-ec,es-es,es-gt,es-hn,es-mx,es-ni,es-pa,es-pe,es-pr,es-py,es-sv,es-us,es-uy,es-ve,et-ee,eu-es,fa-ir,fi-fi,fil-ph,fo-fo,fr-be,fr-ca,fr-ch,fr-fr,fr-lu,fr-mc,fy-nl,ga-ie,gd-db,gl-es,gsw-fr,gu-in,he-il,hi-in,hr-ba,hr-hr,hsb-de,hu-hu,hy-am,id-id,ig-ng,ii-cn,is-is,it-ch,it-it,ja-jp,ka-ge,kk-kz,kl-gl,km-kh,kn-in,kok-in,ko-kr,ky-kg,lb-lu,lo-la,lt-lt,lv-lv,mi-nz,mk-mk,ml-in,mn-mn,moh-ca,mr-in,ms-bn,ms-my,mt-mt,nb-no,ne-np,nl-be,nl-nl,nn-no,nso-za,oc-fr,or-in,pa-in,pl-pl,prs-af,ps-af,pt-br,pt-pt,qut-gt,quz-bo,quz-ec,quz-pe,rm-ch,ro-ro,ru-ru,rw-rw,sah-ru,sa-in,se-fi,se-no,se-se,si-lk,sk-sk,sl-si,sma-no,sma-se,smj-no,smj-se,smn-fi,sms-fi,sq-al,sv-fi,sv-se,sw-ke,syr-sy,ta-in,te-in,th-th,tk-tm,tn-za,tr-tr,tt-ru,ug-cn,uk-ua,ur-pk,vi-vn,wo-sn,xh-za,yo-ng,zh-cn,zh-hk,zh-mo,zh-sg,zh-tw,zu-za,az-cyrl-az,az-latn-az,bs-cyrl-ba,bs-latn-ba,ha-latn-ng,iu-cans-ca,iu-latn-ca,mn-mong-cn,sr-cyrl-ba,sr-cyrl-cs,sr-cyrl-me,sr-cyrl-rs,sr-latn-ba,sr-latn-cs,sr-latn-me,sr-latn-rs,tg-cyrl-tj,tzm-latn-dz,uz-cyrl-uz,uz-latn-uz";

struct NameRegexes {
    architecture: [Regex; 5],
    locale: Regex,
    sap_package: Regex,
    kb_number: Regex,
    cleanup: [Regex; 18],
    program_name_split: Regex,
    non_letters_and_digits: Regex,
}

impl NameRegexes {
    fn new() -> Self {
        Self {
            // The order is significant: combined 32/64-bit names must be handled before 64-bit,
            // and 64-bit must be handled before x86 because x86-64 contains x86.
            architecture: [
                Regex::new(r"(?i)(?:^|[^\p{L}\p{Nd}])(?P<target>(?:64[\\/]32|32[\\/]64)[\p{Pd}\p{Pc}\p{Z}]?BITS?(?:\sEDITION)?)").unwrap(),
                Regex::new(r"(?i)(?:^|[^\p{L}\p{Nd}])(?P<target>X64|AMD64|X86(?:[\p{Pd}\p{Pc}]64))(?:(?P<edition>\sEDITION)|\P{Nd}|$)").unwrap(),
                Regex::new(r"(?i)(?:^|[^\p{L}\p{Nd}])(?P<target>64[\p{Pd}\p{Pc}\p{Z}]?BITS?(?:\sEDITION)?)").unwrap(),
                Regex::new(r"(?i)(?:^|[^\p{L}\p{Nd}])(?P<target>X32|X86)(?:(?P<edition>\sEDITION)|\P{Nd}|$)").unwrap(),
                Regex::new(r"(?i)(?:^|[^\p{L}\p{Nd}])(?P<target>32[\p{Pd}\p{Pc}\p{Z}]?BITS?(?:\sEDITION)?)").unwrap(),
            ],
            locale: Regex::new(r"(?i)(?:^|[^A-Z])(?:(?P<valencia>[A-Z]{2,3}(?:-(?:CANS|CYRL|LATN|MONG))?-[A-Z]{2}-VALENCIA)|(?P<target>[A-Z]{2,3}(?:-(?:CANS|CYRL|LATN|MONG))?-[A-Z]{2})(?:[^A-Z]|$))").unwrap(),
            sap_package: Regex::new(r"(?i)^(?:[\p{Lu}\p{Nd}]+[._])+[\p{Lu}\p{Nd}]+(?:-(?:\p{Nd}+\.)+\p{Nd}+)(?:-(?:\p{Lu}{2}(?:_\p{Lu}{2})?|CORE))(?:-(?:\p{Lu}{2}|\p{Nd}{2}))$").unwrap(),
            kb_number: Regex::new(r"(?i)\((KB\d+)\)").unwrap(),
            cleanup: [
                Regex::new(r"(?i)^(?:ROBLOX\s(?:PLAYER|STUDIO))(?P<target>\sFOR\s.*)").unwrap(),
                Regex::new(r"(?i)^(?:BOMGAR\s(?:JUMP CLIENT|(?:ACCESS|REPRESENTATIVE) CONSOLE|BUTTON)|EMBEDDED CALLBACK)(?P<target>\s.*)").unwrap(),
                Regex::new(r"(?i)(?P<target>^\(.*?\))").unwrap(),
                Regex::new(r#"(?i)(?P<target>\(\s*\)|\[\s*\]|"\s*")"#).unwrap(),
                Regex::new(r"(?i)(?P<target>\(CHANGE #\d{1,2} TO [CDEF]:\\(?:.+?\\)*[^\s]*\\?\))").unwrap(),
                Regex::new(r"(?i)(?P<target>\([CDEF]:\\(?:.+?\\)*[^\s]*\\?\))").unwrap(),
                Regex::new(r#"(?i)(?P<target>"[CDEF]:\\(?:.+?\\)*[^\s]*\\?")"#).unwrap(),
                Regex::new(r"(?i)(?P<target>(?:(?:INSTALLED\sAT|IN)\s)?[CDEF]:\\(?:.+?\\)*[^\s]*\\?)").unwrap(),
                Regex::new(r"(?i)(?:^|[^\p{L}])(?P<target>(?:(?:V|VER|VERSI(?:O|Ó)N|VERSÃO|VERSIE|WERSJA|BUILD|RELEASE|RC|SP)\P{L})?\p{Lu}\p{Nd}+(?:[\p{Po}\p{Pd}\p{Pc}]\p{Nd}+)+)").unwrap(),
                Regex::new(r"(?i)(?:^|[^\p{L}])(?P<target>(?:V|VER|VERSI(?:O|Ó)N|VERSÃO|VERSIE|WERSJA|BUILD|RELEASE|RC|SP)\P{L}?\p{Nd}+(?:[\p{Po}\p{Pd}\p{Pc}]\p{Nd}?(?:RC|B|A|R|SP|K)?\p{Nd}+)+(?:[\p{Po}\p{Pd}\p{Pc}]?[\p{L}\p{Nd}]+)*)").unwrap(),
                Regex::new(r"(?i)(?P<target>\p{Nd}+(?:[\p{Po}\p{Pd}\p{Pc}]\p{Nd}?(?:RC|B|A|R|SP|K)?\p{Nd}+)+(?:[\p{Po}\p{Pd}\p{Pc}]?[\p{L}\p{Nd}]+)*)").unwrap(),
                Regex::new(r"(?i)(?P<target>FOR\s(?:P|V|R|VER|VERSI(?:O|Ó)N|VERSÃO|VERSIE|WERSJA|BUILD|RELEASE|RC|SP)(?:\P{L}|\P{L}\p{L})?(?:\p{Nd}|\.\p{Nd})+(?:RC|B|A|R|V|SP)?\p{Nd}?)").unwrap(),
                Regex::new(r"(?i)(?:^|[^\p{L}])(?P<target>(?:P|V|R|VER|VERSI(?:O|Ó)N|VERSÃO|VERSIE|WERSJA|BUILD|RELEASE|RC|SP)(?:\P{L}|\P{L}\p{L})?(?:\p{Nd}|\.\p{Nd})+(?:RC|B|A|R|V|SP)?\p{Nd}?)").unwrap(),
                Regex::new(r"(?i)(?P<target>\sEN\s*$)").unwrap(),
                Regex::new(r"(?P<target>\([^()]*\)|\[[^\[\]]*\])").unwrap(),
                Regex::new(r#"(?P<target>\p{Ps}.*\p{Pe}|".*")"#).unwrap(),
                Regex::new(r"(?i)(?:^|[^\p{L}])(?P<target>(?:HTTPS?|FTP)://)").unwrap(),
                Regex::new(r"(?P<target>^[^\p{L}\p{Nd}]+|[^\p{L}\p{Nd}]+$)").unwrap(),
            ],
            program_name_split: Regex::new(r"[^\p{L}\p{Nd}+&]").unwrap(),
            non_letters_and_digits: Regex::new(r"[^\p{L}\p{Nd}]").unwrap(),
        }
    }
}

static REGEXES: LazyLock<NameRegexes> = LazyLock::new(NameRegexes::new);

/// Normalizes a package name using [winget-cli's initial name-normalization algorithm].
///
/// This is the form winget uses when correlating an installed ARP display name with an available
/// package name. It intentionally removes version, architecture, locale, path, and other common
/// qualifiers before retaining only letters and decimal digits.
///
/// [winget-cli's initial name-normalization algorithm]: https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCommonCore/NameNormalization.cpp
#[must_use]
pub fn normalize_name(name: &str) -> String {
    let folded = CaseMapper::new().fold_string(name);
    let mut result = ComposingNormalizerBorrowed::new_nfkc()
        .normalize(&folded)
        .trim_matches(ASCII_WHITESPACE)
        .to_owned();

    if let Some(index) = find_double_at_after_three_utf16_code_units(&result) {
        result.truncate(index);
    }

    while unwrap(&mut result) {}

    if REGEXES.sap_package.is_match(&result) {
        return result;
    }

    if !remove_all(&REGEXES.architecture[0], &mut result)
        && !remove_all(&REGEXES.architecture[1], &mut result)
        && !remove_all(&REGEXES.architecture[2], &mut result)
        && !remove_all(&REGEXES.architecture[3], &mut result)
    {
        remove_all(&REGEXES.architecture[4], &mut result);
    }
    remove_locales(&mut result);

    result = REGEXES.kb_number.replace_all(&result, "$1").into_owned();

    loop {
        let mut removed = false;
        for expression in &REGEXES.cleanup {
            removed = remove_all(expression, &mut result) || removed;
        }
        if !removed {
            break;
        }
    }

    let mut normalized = String::new();
    for token in REGEXES
        .program_name_split
        .split(&result)
        .filter(|token| !token.trim_matches(ASCII_WHITESPACE).is_empty())
    {
        if !normalized.is_empty() && is_legal_entity_suffix(token) {
            continue;
        }
        normalized.push_str(token);
    }

    REGEXES
        .non_letters_and_digits
        .replace_all(&normalized, "")
        .into_owned()
}

fn unwrap(value: &mut String) -> bool {
    let wrapped = (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('(') && value.ends_with(')'));
    if wrapped && value.len() >= 2 {
        value.remove(0);
        value.pop();
        true
    } else {
        false
    }
}

fn find_double_at_after_three_utf16_code_units(value: &str) -> Option<usize> {
    let mut utf16_offset = 0;
    for (byte_offset, character) in value.char_indices() {
        if utf16_offset >= 3 && value[byte_offset..].starts_with("@@") {
            return Some(byte_offset);
        }
        utf16_offset += character.len_utf16();
    }
    None
}

fn remove_all(expression: &Regex, input: &mut String) -> bool {
    let mut removed = false;
    while let Some(range) = expression.captures(input).and_then(|captures| {
        let target = captures.name("target")?;
        Some(target.start()..captures.name("edition").unwrap_or(target).end())
    }) {
        input.replace_range(range, "");
        removed = true;
    }
    removed
}

fn remove_locales(input: &mut String) {
    let mut offset = 0;
    while let Some(captures) = REGEXES.locale.captures_at(input, offset) {
        let locale = captures
            .name("valencia")
            .or_else(|| captures.name("target"))
            .unwrap();
        if is_locale(locale.as_str()) {
            input.replace_range(locale.range(), "");
            offset = 0;
        } else {
            offset = locale.end();
        }
    }
}

fn is_locale(value: &str) -> bool {
    LOCALES.split(',').any(|locale| locale == value)
}

fn is_legal_entity_suffix(value: &str) -> bool {
    LEGAL_ENTITY_SUFFIXES
        .split(',')
        .any(|suffix| suffix == value)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::normalize_name;

    #[rstest]
    #[case("Name", "name")]
    #[case("Name x86_64", "name")]
    #[case("Name (64 bit)", "name")]
    #[case("Name 32/64 bit", "name")]
    #[case("Name en-US", "name")]
    #[case("Name (es-mx)", "name")]
    #[case("Names-mx", "namesmx")]
    #[case("Name xx-YY-en-US", "namexxyy")]
    #[case("Name ca-ES-valenciaX", "namex")]
    #[case("Name en-US-valenciaX", "nameenusvalenciax")]
    #[case("Fix for (KB42)", "fixforkb42")]
    #[case("Example Product version 2.1.7", "exampleproduct")]
    #[case("Name x64 edition", "name")]
    #[case("Name x64 edition2", "name2")]
    #[case("Name x641", "namex641")]
    #[case("Name 64-bitterness", "nameterness")]
    #[case("Name x64 x86", "namex86")]
    #[case("Name 32/64-bit x64", "namex64")]
    #[case("Name XFOR V1", "namex")]
    #[case("éé@@suffix", "éésuffix")]
    #[case("ééé@@suffix", "ééé")]
    fn matches_winget_cli(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(normalize_name(input), expected);
    }
}
