//! Package-name and publisher normalization compatible with winget-cli's initial normalization algorithm.

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

/// Normalizes a publisher for winget-cli's case-insensitive ARP matching.
///
/// Applies Unicode case folding followed by NFKC, then the publisher-specific cleanup rules
/// from winget-cli's initial normalization algorithm. Unlike package names, publishers stop
/// at the first legal entity suffix after the first token (for example, `Example LLC Tools`
/// becomes `example`). This corresponds to `FoldCase` followed by `NormalizePublisher`,
/// rather than the case-preserving `NormalizePublisher` method alone.
///
/// [Upstream implementation](https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCommonCore/NameNormalization.cpp).
#[must_use]
pub fn normalize_publisher(publisher: &str) -> String {
    let folded = CaseMapper::new().fold_string(publisher);
    let normalized = ComposingNormalizerBorrowed::new_nfkc().normalize(&folded);
    let mut prepared = normalized.trim_matches(ASCII_WHITESPACE);
    if let Some(index) = find_double_at_after_three_utf16_code_units(prepared) {
        prepared = &prepared[..index];
    }
    // Strip wrappers on a borrowed slice so deeply nested wrappers need no repeated shifts.
    while prepared.len() >= 2
        && ((prepared.starts_with('"') && prepared.ends_with('"'))
            || (prepared.starts_with('(') && prepared.ends_with(')')))
    {
        prepared = &prepared[1..prepared.len() - 1];
    }
    let mut result = prepared.to_owned();

    // Compile independently of the much larger package-name regex set. Reuse a scratch
    // buffer across cleanup passes, and inspect lookaround boundaries without backtracking.
    let regexes = &*PUBLISHER_REGEXES;
    let mut scratch = String::with_capacity(result.len());
    loop {
        let mut removed = false;
        for rule in &regexes.cleanup {
            removed |= remove_publisher_matches(rule, &mut result, &mut scratch);
        }
        if !removed {
            break;
        }
    }

    scratch.clear();
    for token in regexes
        .split
        .split(&result)
        .filter(|token| !token.is_empty())
    {
        // NFKC can introduce uppercase letters after the initial fold (e.g. ᴸ -> L).
        // Upstream folds each token again when checking legal entity suffixes.
        if !scratch.is_empty() && is_legal_entity_suffix(&CaseMapper::new().fold_string(token)) {
            break;
        }
        scratch.push_str(token);
    }
    scratch
}

struct PublisherRegexes {
    cleanup: [PublisherRule; 8],
    letter: Regex,
    split: Regex,
}

struct PublisherRule {
    expression: Regex,
    boundary: PublisherBoundary,
}

#[derive(Clone, Copy)]
enum PublisherBoundary {
    Unrestricted,
    VersionDelimited,
    Version,
    Protocol,
    Acronym,
}

static PUBLISHER_REGEXES: LazyLock<PublisherRegexes> = LazyLock::new(|| {
    use PublisherBoundary::*;

    PublisherRegexes {
        cleanup: [
            (r"(?i)(?:(?P<prefix>V|VER|VERSI(?:O|Ó)N|VERSÃO|VERSIE|WERSJA|BUILD|RELEASE|RC|SP)\P{L}?)?\p{Nd}+([\p{Po}\p{Pd}\p{Pc}]\p{Nd}?(RC|B|A|R|SP|K)?\p{Nd}+)+([\p{Po}\p{Pd}\p{Pc}]?[\p{L}\p{Nd}]+)*", VersionDelimited),
            (r"(?i)(?P<for>FOR\s)?(?:P|V|R|VER|VERSI(?:O|Ó)N|VERSÃO|VERSIE|WERSJA|BUILD|RELEASE|RC|SP)(?:\P{L}|\P{L}\p{L})?(\p{Nd}|\.\p{Nd})+(?:RC|B|A|R|V|SP)?\p{Nd}?", Version),
            (r"\([^()]*\)|\[[^\[\]]*\]", Unrestricted),
            // ICU's dot excludes all Unicode line terminators, not just LF.
            (r#"\p{Ps}[^\n\r\x{85}\x{2028}\x{2029}]*\p{Pe}|"[^\n\r\x{85}\x{2028}\x{2029}]*""#, Unrestricted),
            (r"(?i)(?:HTTPS?|FTP)://", Protocol),
            (r"(?:^|\s)(?P<target>[^\p{L}]+)(?:\s|$)", Unrestricted),
            (r"\P{L}+$", Unrestricted),
            (r"[./]", Acronym),
        ].map(|(pattern, boundary)| PublisherRule {
            expression: Regex::new(pattern).unwrap(),
            boundary,
        }),
        letter: Regex::new(r"\p{L}").unwrap(),
        split: Regex::new(r"[^\p{L}\p{Nd}]").unwrap(),
    }
});

fn publisher_letter(character: char) -> bool {
    PUBLISHER_REGEXES
        .letter
        .is_match(character.encode_utf8(&mut [0; 4]))
}

fn publisher_letter_before(value: &str, index: usize) -> bool {
    value[..index]
        .chars()
        .next_back()
        .is_some_and(publisher_letter)
}

// Each regex replaces all non-overlapping matches against the original string, as ICU
// Replace does. In particular, a deletion must not change a later match's lookbehind until
// the next cleanup pass. Only the boundary assertions missing from Rust's regex engine
// are checked here; matching itself uses its finite automata.
fn remove_publisher_matches(
    rule: &PublisherRule,
    value: &mut String,
    scratch: &mut String,
) -> bool {
    scratch.clear();
    let mut search = 0;
    let mut copied = 0;
    let expression = &rule.expression;
    let mut locations = expression.capture_locations();
    let prefix = expression
        .capture_names()
        .position(|name| name == Some("prefix"));
    let for_prefix = expression
        .capture_names()
        .position(|name| name == Some("for"));
    let target = expression
        .capture_names()
        .position(|name| name == Some("target"));
    while let Some(matched) = expression.captures_read_at(&mut locations, value, search) {
        let valid = match rule.boundary {
            PublisherBoundary::VersionDelimited => {
                prefix.and_then(|index| locations.get(index)).is_none()
                    || !publisher_letter_before(value, matched.start())
            }
            PublisherBoundary::Version => !publisher_letter_before(
                value,
                for_prefix
                    .and_then(|index| locations.get(index))
                    .map_or(matched.start(), |(_, end)| end),
            ),
            PublisherBoundary::Protocol => !publisher_letter_before(value, matched.start()),
            PublisherBoundary::Acronym => {
                let mut before = value[..matched.start()].chars().rev();
                let mut after = value[matched.end()..].chars();
                before.next().is_some_and(publisher_letter)
                    && !before.next().is_some_and(publisher_letter)
                    && after.next().is_some_and(publisher_letter)
                    && !after.next().is_some_and(publisher_letter)
            }
            PublisherBoundary::Unrestricted => true,
        };
        if valid {
            let (start, end) = target
                .and_then(|index| locations.get(index))
                .unwrap_or((matched.start(), matched.end()));
            scratch.push_str(&value[copied..start]);
            copied = end;
            search = end;
        } else {
            search = matched.start() + value[matched.start()..].chars().next().unwrap().len_utf8();
        }
    }
    if copied == 0 {
        return false;
    }
    scratch.push_str(&value[copied..]);
    core::mem::swap(value, scratch);
    true
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

    use super::{normalize_name, normalize_publisher};

    #[test]
    fn publishers_match_winget_cli_corpus() {
        // Copyright (c) Microsoft Corporation. Licensed under the MIT License.
        // Inputs: https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCLITests/TestData/InputPublishers.txt
        // Expected publishers (before the first dot): https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCLITests/TestData/NormalizationInitialIds.txt
        let cases = [
            ("Wildfire Games", "WildfireGames"),
            ("SweetScape Software", "SweetScapeSoftware"),
            ("360安全中心", "360安全中心"),
            ("360安全中心", "360安全中心"),
            ("Open Media LLC", "OpenMedia"),
            ("Open Media LLC", "OpenMedia"),
            ("Open Media LLC", "OpenMedia"),
            ("Open Media LLC", "OpenMedia"),
            ("Open Media LLC", "OpenMedia"),
            ("Open Media LLC", "OpenMedia"),
            ("Igor Pavlov", "IgorPavlov"),
            ("Igor Pavlov", "IgorPavlov"),
            ("Igor Pavlov", "IgorPavlov"),
            ("Igor Pavlov, Tino Reichardt", "IgorPavlovTinoReichardt"),
            (
                "Amazon Web Services Developer Relations",
                "AmazonWebServicesDeveloperRelations",
            ),
            ("Amazon Web Services", "AmazonWebServices"),
            ("AWS Serverless Applications", "AWSServerlessApplications"),
            ("Axis Communications AB", "AxisCommunications"),
            ("Abacus Research AG", "AbacusResearch"),
            ("Microsoft", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Adobe Systems Incorporated", "AdobeSystems"),
            ("Adobe Systems Incorporated", "AdobeSystems"),
            ("Adobe Systems Incorporated", "AdobeSystems"),
            ("AdoptOpenJDK", "AdoptOpenJDK"),
            ("AdoptOpenJDK", "AdoptOpenJDK"),
            ("AdoptOpenJDK", "AdoptOpenJDK"),
            ("AdoptOpenJDK", "AdoptOpenJDK"),
            ("AdoptOpenJDK", "AdoptOpenJDK"),
            ("AdoptOpenJDK", "AdoptOpenJDK"),
            ("AdoptOpenJDK", "AdoptOpenJDK"),
            ("Famatech", "Famatech"),
            ("Ondrej Salplachta", "OndrejSalplachta"),
            ("Famatech", "Famatech"),
            ("Pawel Psztyc", "PawelPsztyc"),
            ("Aegisub Team", "AegisubTeam"),
            ("Aegisub Team", "AegisubTeam"),
            ("a wandersick", "awandersick"),
            ("Alacritty", "Alacritty"),
            ("Alchemy Development Group", "AlchemyDevelopmentGroup"),
            ("Algoryx", "Algoryx"),
            ("Stefan Sundin", "StefanSundin"),
            ("Amazon.com, Inc.", "Amazoncom"),
            ("Amazon", "Amazon"),
            ("Amazon", "Amazon"),
            ("Amazon Web Services, Inc", "AmazonWebServices"),
            ("Anaconda, Inc.", "Anaconda"),
            ("Angry IP Scanner", "AngryIPScanner"),
            ("AntiMicro", "AntiMicro"),
            ("AppGet", "AppGet"),
            ("Appium Developers", "AppiumDevelopers"),
            ("Appium Developers", "AppiumDevelopers"),
            ("Arduino LLC", "Arduino"),
            ("Armagetron Advanced Team", "ArmagetronAdvancedTeam"),
            ("Sundaram Ramaswamy", "SundaramRamaswamy"),
            ("Rabid Viper Productions", "RabidViperProductions"),
            ("Audacity Team", "AudacityTeam"),
            ("CodeUX.design e.U.", "CodeUXdesigneU"),
            ("Armin Osaj", "ArminOsaj"),
            ("Lexikos", "Lexikos"),
            ("Lexikos", "Lexikos"),
            ("The Sleuth Kit", "TheSleuthKit"),
            ("GPL Public release.", "GPLPublicrelease"),
            ("OmicronLab", "OmicronLab"),
            ("7room", "7room"),
            ("Microsoft® Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft", "Microsoft"),
            ("Microsoft", "Microsoft"),
            ("Microsoft", "Microsoft"),
            ("Marcin Szeniak", "MarcinSzeniak"),
            ("Contoso.com", "Contosocom"),
            ("Beda Kosata", "BedaKosata"),
            (
                "Space Sciences Laboratory, U.C. Berkeley",
                "SpaceSciencesLaboratoryUCBerkeley",
            ),
            ("BPBible Development Team", "BPBibleDevelopmentTeam"),
            ("Google, Inc.", "Google"),
            ("Debauchee Open Source Group", "DebaucheeOpenSourceGroup"),
            ("Debauchee Open Source Group", "DebaucheeOpenSourceGroup"),
            ("Paul Frazee", "PaulFrazee"),
            ("Elastic", "Elastic"),
            ("Elastic", "Elastic"),
            ("Marco Mastroddi Software", "MarcoMastroddiSoftware"),
            ("beeftext.org", "beeftextorg"),
            (
                "The Betaflight open source project.",
                "TheBetaflightopensourceproject",
            ),
            ("Scooter Software, Inc.", "ScooterSoftware"),
            ("Scooter Software", "ScooterSoftware"),
            ("Scooter Software", "ScooterSoftware"),
            ("Bigly Software", "BiglySoftware"),
            ("BitPay", "BitPay"),
            ("Bitwarden Inc.", "Bitwarden"),
            ("BleachBit", "BleachBit"),
            ("Blender Foundation", "BlenderFoundation"),
            ("BlueJeans Network, Inc.", "BlueJeansNetwork"),
            ("Hamster Republic Productions", "HamsterRepublicProductions"),
            ("Apple Inc.", "Apple"),
            ("Andrew Sampson", "AndrewSampson"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("brackets.io", "bracketsio"),
            ("Brave Software Inc", "BraveSoftware"),
            ("Brave Software Inc", "BraveSoftware"),
            ("TGRMN Software", "TGRMNSoftware"),
            ("Buttercup", "Buttercup"),
            ("Buttercup", "Buttercup"),
            ("C-Dogs SDL Team", "CDogsSDLTeam"),
            ("SingularLabs", "SingularLabs"),
            ("Piriform", "Piriform"),
            ("Canneverbe Limited", "Canneverbe"),
            ("Kitware", "Kitware"),
            ("Corsair", "Corsair"),
            ("CPUID, Inc.", "CPUID"),
            ("CPUID, Inc.", "CPUID"),
            ("CPUID, Inc.", "CPUID"),
            ("CPUID, Inc.", "CPUID"),
            ("CPUID, Inc.", "CPUID"),
            ("Penguin Labs, LLC", "PenguinLabs"),
            ("SaeraSoft", "SaeraSoft"),
            ("TechSmith Corporation", "TechSmith"),
            ("TechSmith Corporation", "TechSmith"),
            ("Sindre Sorhus", "SindreSorhus"),
            ("Sindre Sorhus", "SindreSorhus"),
            ("Giel Cobben", "GielCobben"),
            ("Mathew Sachin", "MathewSachin"),
            ("RedDucks", "RedDucks"),
            ("Alexandr Subbotin", "AlexandrSubbotin"),
            ("MIT IS&T", "MITIST"),
            ("secana", "secana"),
            ("Webprofusion Pty Ltd", "Webprofusion"),
            ("a wandersick", "awandersick"),
            ("Chef Software, Inc.", "ChefSoftware"),
            ("ChemAxon", "ChemAxon"),
            ("ChemAxon", "ChemAxon"),
            ("ChemAxon", "ChemAxon"),
            ("ChemAxon", "ChemAxon"),
            ("The Chromium Authors", "TheChromiumAuthors"),
            ("Circuit Diagram", "CircuitDiagram"),
            ("Circuit Diagram", "CircuitDiagram"),
            ("Cisco Webex LLC", "CiscoWebex"),
            ("Daniel Scalzi", "DanielScalzi"),
            ("alch", "alch"),
            ("Fndroid", "Fndroid"),
            ("Fndroid", "Fndroid"),
            ("Fndroid", "Fndroid"),
            ("Fndroid", "Fndroid"),
            (
                "Beijing EEO Education Technology Co., Ltd.",
                "BeijingEEOEducationTechnology",
            ),
            ("Clementine", "Clementine"),
            ("Martin Ridgers", "MartinRidgers"),
            ("Cloudflare, Inc.", "Cloudflare"),
            ("The Code::Blocks Team", "TheCodeBlocksTeam"),
            ("Eran Ifrah", "EranIfrah"),
            ("Steven Cole", "StevenCole"),
            ("TerranovaTeam", "TerranovaTeam"),
            ("Jay Prall", "JayPrall"),
            ("Toinane", "Toinane"),
            ("ConEmu-Maximus5", "ConEmuMaximus"),
            ("Concept2 Inc.", "Concept2"),
            ("Microsoft Corporation", "Microsoft"),
            ("Contasimple S.L.", "Contasimple"),
            ("ALCPU", "ALCPU"),
            ("Couchbase Inc.", "Couchbase"),
            ("Couchbase Inc.", "Couchbase"),
            ("Cozy Cloud", "CozyCloud"),
            ("The Cppcheck team", "TheCppcheckteam"),
            ("Habib Rehman", "HabibRehman"),
            ("cryptomator.org", "cryptomatororg"),
            ("Crystal Dew World", "CrystalDewWorld"),
            ("Crystal Dew World", "CrystalDewWorld"),
            ("Crystal Dew World", "CrystalDewWorld"),
            ("Crystal Dew World", "CrystalDewWorld"),
            ("Crystal Dew World", "CrystalDewWorld"),
            ("Crystal Dew World", "CrystalDewWorld"),
            ("Crystal Dew World", "CrystalDewWorld"),
            ("Crystal Dew World", "CrystalDewWorld"),
            ("Crystal Dew World", "CrystalDewWorld"),
            ("Crystal Dew World", "CrystalDewWorld"),
            ("Crystal Dew World", "CrystalDewWorld"),
            ("Crystal Dew World", "CrystalDewWorld"),
            ("Crystal Dew World", "CrystalDewWorld"),
            ("Crystal Dew World", "CrystalDewWorld"),
            ("Crystal Dew World", "CrystalDewWorld"),
            ("Crystal Dew World", "CrystalDewWorld"),
            ("Crystal Dew World", "CrystalDewWorld"),
            ("cubicsdr.com", "cubicsdrcom"),
            ("Acro Software Inc.", "AcroSoftware"),
            ("iterate GmbH", "iterate"),
            ("DB Browser for SQLite Team", "DBBrowserforSQLiteTeam"),
            ("DBeaver Corp", "DBeaver"),
            ("DBeaver Corp", "DBeaver"),
            ("DBeaver Corp", "DBeaver"),
            ("DBeaver Corp", "DBeaver"),
            ("DBeaver Corp", "DBeaver"),
            ("DBeaver Corp", "DBeaver"),
            ("DBeaver Corp", "DBeaver"),
            ("DBeaver Corp", "DBeaver"),
            ("DBeaver Corp", "DBeaver"),
            ("DJI", "DJI"),
            ("DJI", "DJI"),
            ("DJI", "DJI"),
            ("DJI", "DJI"),
            ("DJI", "DJI"),
            ("DJI", "DJI"),
            ("DJI", "DJI"),
            ("DJI", "DJI"),
            ("TshwaneDJe", "TshwaneDJe"),
            ("Boxstar", "Boxstar"),
            ("Boxstar", "Boxstar"),
            ("Deezer", "Deezer"),
            ("Ashley Stone", "AshleyStone"),
            ("Piriform", "Piriform"),
            ("Dell, Inc.", "Dell"),
            ("Dell Inc.", "Dell"),
            ("Bloodshed Software", "BloodshedSoftware"),
            ("Thinking Man Software", "ThinkingManSoftware"),
            ("Serraniel", "Serraniel"),
            ("Serraniel", "Serraniel"),
            ("Scott Brogden", "ScottBrogden"),
            ("Dixa", "Dixa"),
            ("DjVuZone", "DjVuZone"),
            ("DockStation", "DockStation"),
            ("Docker Inc.", "Docker"),
            ("Dokany Project", "DokanyProject"),
            ("Dokany Project", "DokanyProject"),
            ("Dolphin Team", "DolphinTeam"),
            ("dengine.net", "denginenet"),
            ("Digimezzo", "Digimezzo"),
            ("Doxie & Co. LLC", "Doxie"),
            ("Dropbox, Inc.", "Dropbox"),
            ("EasternGraphics", "EasternGraphics"),
            ("EasternGraphics", "EasternGraphics"),
            ("EagleGet", "EagleGet"),
            ("EagleGet", "EagleGet"),
            ("Luke Stratman", "LukeStratman"),
            ("ES-Computing", "ESComputing"),
            ("EduMIPS64 Development Team", "EduMIPS64DevelopmentTeam"),
            ("Elastic", "Elastic"),
            ("Elgato Systems GmbH", "ElgatoSystems"),
            ("Empoche.com", "Empochecom"),
            ("Empoche.com", "Empochecom"),
            ("MacPaw, Inc.", "MacPaw"),
            ("a wandersick", "awandersick"),
            (
                "Sinew Software Systems Private Limited",
                "SinewSoftwareSystemsPrivate",
            ),
            ("The Eraser Project", "TheEraserProject"),
            ("The Eraser Project", "TheEraserProject"),
            ("The Eraser Project", "TheEraserProject"),
            ("The Eraser Project", "TheEraserProject"),
            ("Esteem", "Esteem"),
            ("Ethereum", "Ethereum"),
            ("Evernote Corp.", "Evernote"),
            ("Evernote Corp.", "Evernote"),
            ("David Carpenter", "DavidCarpenter"),
            ("David Carpenter", "DavidCarpenter"),
            ("David Carpenter", "DavidCarpenter"),
            ("David Carpenter", "DavidCarpenter"),
            ("voidtools", "voidtools"),
            ("ExpressVPN", "ExpressVPN"),
            ("Ultrapico", "Ultrapico"),
            ("The ExtremeTuxRacer team", "TheExtremeTuxRacerteam"),
            ("Eugene Roshal & Far Group", "EugeneRoshalFarGroup"),
            ("H.Shirouzu", "HShirouzu"),
            ("FastStone Soft", "FastStoneSoft"),
            ("FastStone Soft", "FastStoneSoft"),
            ("Fedora Project", "FedoraProject"),
            ("Amine Mouafik", "AmineMouafik"),
            ("Progress Software Corporation", "ProgressSoftware"),
            ("Progress Software Corporation", "ProgressSoftware"),
            ("Progress Software Corporation", "ProgressSoftware"),
            ("Progress Software Corporation", "ProgressSoftware"),
            ("Progress Software Corporation", "ProgressSoftware"),
            ("Progress Software Corporation", "ProgressSoftware"),
            ("Progress Software Corporation", "ProgressSoftware"),
            ("Adrien Allard", "AdrienAllard"),
            ("Binary Fortress Software", "BinaryFortressSoftware"),
            ("Tim Kosse", "TimKosse"),
            ("Tim Kosse", "TimKosse"),
            ("Tim Kosse", "TimKosse"),
            ("Tim Kosse", "TimKosse"),
            ("Tim Kosse", "TimKosse"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("OpenSight Software LLC", "OpenSightSoftware"),
            ("The FlightGear Team", "TheFlightGearTeam"),
            ("Dominik Levitsky Studio, LLC", "DominikLevitskyStudio"),
            ("FontForgeBuilds", "FontForgeBuilds"),
            ("Free Time", "FreeTime"),
            ("Free Time", "FreeTime"),
            ("Foxit Software Inc.", "FoxitSoftware"),
            ("Foxit Software Inc.", "FoxitSoftware"),
            ("Stefan Malzner", "StefanMalzner"),
            (
                "Marek Jasinski - www.FreeCommander.com",
                "MarekJasinskiwwwFreeCommandercom",
            ),
            ("Humanity", "Humanity"),
            ("The GIMP Team", "TheGIMPTeam"),
            ("The GIMP Team", "TheGIMPTeam"),
            ("The GIMP Team", "TheGIMPTeam"),
            ("The GIMP Team", "TheGIMPTeam"),
            ("The GIMP Team", "TheGIMPTeam"),
            ("The GIMP Team", "TheGIMPTeam"),
            ("The GIMP Team", "TheGIMPTeam"),
            ("The GIMP Team", "TheGIMPTeam"),
            ("ARM Holdings", "ARM"),
            (
                "The Free Software Foundation, Inc.",
                "TheFreeSoftwareFoundation",
            ),
            ("The GnuPG Project", "TheGnuPGProject"),
            ("GCN Development", "GCNDevelopment"),
            ("GOG.com", "GOGcom"),
            ("Artifex Software Inc.", "ArtifexSoftware"),
            ("Ghostgum Software Pty Ltd", "GhostgumSoftware"),
            ("Garmin Ltd or its subsidiaries", "Garmin"),
            ("ThoughtWorks Inc.", "ThoughtWorks"),
            ("ThoughtWorks Inc.", "ThoughtWorks"),
            ("The Geany developer team", "TheGeanydeveloperteam"),
            ("Primate Labs Inc.", "PrimateLabs"),
            ("Gephi", "Gephi"),
            ("Outertech", "Outertech"),
            ("Git Extensions Team", "GitExtensionsTeam"),
            ("Git Extensions Team", "GitExtensionsTeam"),
            ("Git Extensions Team", "GitExtensionsTeam"),
            ("GitHub, Inc.", "GitHub"),
            (
                "The Git Development Community",
                "TheGitDevelopmentCommunity",
            ),
            (
                "The Git Development Community",
                "TheGitDevelopmentCommunity",
            ),
            (
                "The Git Development Community",
                "TheGitDevelopmentCommunity",
            ),
            (
                "The Git Development Community",
                "TheGitDevelopmentCommunity",
            ),
            (
                "The Git Development Community",
                "TheGitDevelopmentCommunity",
            ),
            (
                "The Git Development Community",
                "TheGitDevelopmentCommunity",
            ),
            ("GitHub, Inc.", "GitHub"),
            ("GitHub, Inc.", "GitHub"),
            ("Stef Heyenrath", "StefHeyenrath"),
            ("Troupe Technology Limited", "TroupeTechnology"),
            ("Glimpse Project", "GlimpseProject"),
            ("Glimpse Project", "GlimpseProject"),
            ("Glimpse Project", "GlimpseProject"),
            ("GnuCash Development Team", "GnuCashDevelopmentTeam"),
            ("GnuCash Development Team", "GnuCashDevelopmentTeam"),
            ("GnuWin32", "GnuWin"),
            ("GnuWin32", "GnuWin"),
            ("GnuWin32", "GnuWin"),
            ("GnuWin32", "GnuWin"),
            ("https://golang.org", "golangorg"),
            ("https://golang.org", "golangorg"),
            ("https://golang.org", "golangorg"),
            ("https://golang.org", "golangorg"),
            ("https://golang.org", "golangorg"),
            ("https://golang.org", "golangorg"),
            ("https://golang.org", "golangorg"),
            ("https://golang.org", "golangorg"),
            ("https://golang.org", "golangorg"),
            ("https://golang.org", "golangorg"),
            ("https://golang.org", "golangorg"),
            ("https://golang.org", "golangorg"),
            ("https://golang.org", "golangorg"),
            ("https://golang.org", "golangorg"),
            ("https://golang.org", "golangorg"),
            ("https://golang.org", "golangorg"),
            ("https://golang.org", "golangorg"),
            ("GoldWave Inc.", "GoldWave"),
            ("Google LLC", "Google"),
            ("Google Inc.", "Google"),
            ("Google", "Google"),
            ("The Gpg4win Project", "TheGpg4winProject"),
            ("The Gpg4win Project", "TheGpg4winProject"),
            ("Grafana Labs", "GrafanaLabs"),
            ("Grafana Labs", "GrafanaLabs"),
            ("Grammarly", "Grammarly"),
            ("The Gramps project", "TheGrampsproject"),
            ("Graphcool", "Graphcool"),
            ("Adam Miskiewicz", "AdamMiskiewicz"),
            ("AT&T Research Labs.", "ATTResearchLabs"),
            ("Greenshot", "Greenshot"),
            ("Grid Team", "GridTeam"),
            ("Epiforge Software, LLC", "EpiforgeSoftware"),
            ("HHD Software, Ltd.", "HHDSoftware"),
            (
                "Hector Maurcio Rodriguez Segura",
                "HectorMaurcioRodriguezSegura",
            ),
            ("HP Inc.", "HP"),
            (
                "Huawei Software Technologies Co., Ltd.",
                "HuaweiSoftwareTechnologies",
            ),
            ("Martin Malik - REALiX", "MartinMalikREALiX"),
            ("Martin Malik - REALiX", "MartinMalikREALiX"),
            ("Martin Malik - REALiX", "MartinMalikREALiX"),
            ("Martin Malik - REALiX", "MartinMalikREALiX"),
            ("HandyOrg", "HandyOrg"),
            ("Vincent L", "VincentL"),
            ("Implbits Software", "ImplbitsSoftware"),
            ("Implbits Software", "ImplbitsSoftware"),
            ("JAM Software", "JAMSoftware"),
            ("Hedgewars Project", "HedgewarsProject"),
            ("Ansgar Becker", "AnsgarBecker"),
            ("Ansgar Becker", "AnsgarBecker"),
            ("Ansgar Becker", "AnsgarBecker"),
            ("Ansgar Becker", "AnsgarBecker"),
            ("Ansgar Becker", "AnsgarBecker"),
            ("Perforce Software, Inc.", "PerforceSoftware"),
            ("IBE Software", "IBESoftware"),
            ("HexChat", "HexChat"),
            ("Scott Lerch", "ScottLerch"),
            ("Caphyon", "Caphyon"),
            ("Borvid", "Borvid"),
            ("Borvid", "Borvid"),
            ("Borvid", "Borvid"),
            ("Borvid", "Borvid"),
            ("Borvid", "Borvid"),
            ("Borvid", "Borvid"),
            ("Borvid", "Borvid"),
            ("Huawei Corporation", "Huawei"),
            ("Hyne & Son Pty Ltd", "HyneSon"),
            ("Marquis Kurt", "MarquisKurt"),
            ("Google Inc", "Google"),
            ("Hex-Rays SA", "HexRays"),
            ("Tibbo Technology Inc", "TibboTechnology"),
            ("David Moore", "DavidMoore"),
            ("IRCCloud Ltd.", "IRCCloud"),
            ("Ivan Zahariev", "IvanZahariev"),
            ("Duong Dieu Phap", "DuongDieuPhap"),
            ("Inkscape", "Inkscape"),
            ("jrsoftware.org", "jrsoftwareorg"),
            ("jrsoftware.org", "jrsoftwareorg"),
            ("Crystal Rich, Ltd", "CrystalRich"),
            ("seonglae", "seonglae"),
            ("Irfan Skiljan", "IrfanSkiljan"),
            ("IronPython Team", "IronPythonTeam"),
            ("IronPython Team", "IronPythonTeam"),
            ("ISWIX LLC", "ISWIX"),
            ("Smart Projects", "SmartProjects"),
            ("ChemAxon", "ChemAxon"),
            ("JabRef", "JabRef"),
            ("Jackett", "Jackett"),
            ("Savoir-Faire Linux", "SavoirFaireLinux"),
            ("Oracle Corporation", "Oracle"),
            ("Oracle Corporation", "Oracle"),
            ("JetBrains", "JetBrains"),
            ("Jitsi Team", "JitsiTeam"),
            ("Jitsi Team", "JitsiTeam"),
            ("Laurent Cozic", "LaurentCozic"),
            ("Laurent Cozic", "LaurentCozic"),
            ("Laurent Cozic", "LaurentCozic"),
            ("Julia Language", "JuliaLanguage"),
            ("Julia Language", "JuliaLanguage"),
            ("Julia Language", "JuliaLanguage"),
            ("KLCP", "KLCP"),
            ("KLCP", "KLCP"),
            ("KKBOX Taiwan Co., Ltd.", "KKBOXTaiwan"),
            ("Chia-Lung, Chen", "ChiaLungChen"),
            ("Dominik Reichl", "DominikReichl"),
            ("Dominik Reichl", "DominikReichl"),
            ("Dominik Reichl", "DominikReichl"),
            ("KeePassXC Team", "KeePassXCTeam"),
            ("KeeWeb", "KeeWeb"),
            ("Keybase, Inc.", "Keybase"),
            ("KiCad", "KiCad"),
            ("KiCad", "KiCad"),
            ("KiCad", "KiCad"),
            ("Krisp Technologies, Inc", "KrispTechnologies"),
            ("Krita Foundation", "KritaFoundation"),
            ("Roni Lehto", "RoniLehto"),
            ("LBRY Inc.", "LBRY"),
            ("LBRY Inc.", "LBRY"),
            ("LBRY Inc.", "LBRY"),
            ("LBRY Inc.", "LBRY"),
            ("LBRY Inc.", "LBRY"),
            ("LINE Corporation", "LINE"),
            ("Joseph Albahari", "JosephAlbahari"),
            ("LLVM", "LLVM"),
            ("LMMS Developers", "LMMSDevelopers"),
            ("LMMS Developers", "LMMSDevelopers"),
            ("love2d.org", "love2dorg"),
            ("leokhoa", "leokhoa"),
            ("LastPass", "LastPass"),
            ("Lazarus Team", "LazarusTeam"),
            ("Riot Games, Inc", "RiotGames"),
            ("Lenovo", "Lenovo"),
            ("Lenovo", "Lenovo"),
            ("Lakend Labs, Inc.", "LakendLabs"),
            ("Leonflix", "Leonflix"),
            ("BellSoft", "BellSoft"),
            ("BellSoft", "BellSoft"),
            ("BellSoft", "BellSoft"),
            ("BellSoft", "BellSoft"),
            ("BellSoft", "BellSoft"),
            ("BellSoft", "BellSoft"),
            ("BellSoft", "BellSoft"),
            ("BellSoft", "BellSoft"),
            ("LibreCAD Team", "LibreCADTeam"),
            ("The Document Foundation", "TheDocumentFoundation"),
            ("The Document Foundation", "TheDocumentFoundation"),
            ("Team Lidarr", "TeamLidarr"),
            ("Alexey 'Tyrrrz' Golub", "AlexeyTyrrrzGolub"),
            ("Alexey 'Tyrrrz' Golub", "AlexeyTyrrrzGolub"),
            ("Christian Kaiser", "ChristianKaiser"),
            ("Leif Asbrink SM5BSZ", "LeifAsbrinkSM5BSZ"),
            ("Lisk Foundation", "LiskFoundation"),
            ("Listen 1", "Listen"),
            ("Listen 1", "Listen"),
            ("Flywheel", "Flywheel"),
            ("Crystal Rich Ltd", "CrystalRich"),
            ("Binary Fortress Software", "BinaryFortressSoftware"),
            ("Binary Fortress Software", "BinaryFortressSoftware"),
            ("Logitech Inc.", "Logitech"),
            ("Loom, Inc.", "Loom"),
            ("LyX Team", "LyXTeam"),
            ("LyX Team", "LyXTeam"),
            ("COTILab", "COTILab"),
            ("COTILab", "COTILab"),
            ("Moritz Bunkus", "MoritzBunkus"),
            ("Moritz Bunkus", "MoritzBunkus"),
            ("Moritz Bunkus", "MoritzBunkus"),
            ("Moritz Bunkus", "MoritzBunkus"),
            ("Moritz Bunkus", "MoritzBunkus"),
            ("MPC-HC Team", "MPCHCTeam"),
            ("MPC-HC Team", "MPCHCTeam"),
            ("MPC-HC Team", "MPCHCTeam"),
            ("Thomas Nordquist", "ThomasNordquist"),
            ("Microsoft", "Microsoft"),
            ("Maxthon International Limited", "MaxthonInternational"),
            ("MY.COM B.V.", "MYCOM"),
            ("FlyingSnow, Samantha Glocker", "FlyingSnowSamanthaGlocker"),
            ("Firetrust", "Firetrust"),
            ("Firetrust", "Firetrust"),
            ("Majsoul Plus Team", "MajsoulPlusTeam"),
            ("GuinpinSoft inc", "GuinpinSoft"),
            ("Malwarebytes", "Malwarebytes"),
            ("KDE", "KDE"),
            ("MariaDB Corporation Ab", "MariaDB"),
            ("Jocs", "Jocs"),
            ("West Wind Technologies", "WestWindTechnologies"),
            ("West Wind Technologies", "WestWindTechnologies"),
            ("West Wind Technologies", "WestWindTechnologies"),
            ("West Wind Technologies", "WestWindTechnologies"),
            ("West Wind Technologies", "WestWindTechnologies"),
            ("Markdown Outlook", "MarkdownOutlook"),
            ("Master Packager Ltd.", "MasterPackager"),
            ("Mattermost, Inc.", "Mattermost"),
            ("MediaArea.net", "MediaAreanet"),
            ("MediaArea.net", "MediaAreanet"),
            ("Ventis Media Inc.", "VentisMedia"),
            ("The Meld project", "TheMeldproject"),
            ("Janea Systems", "JaneaSystems"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Garage", "MicrosoftGarage"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft", "Microsoft"),
            ("Microsoft", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Anaconda, Inc.", "Anaconda"),
            ("Anaconda, Inc.", "Anaconda"),
            ("MongoDB Inc.", "MongoDB"),
            ("Xamarin, Inc.", "Xamarin"),
            ("The MonoGame Team", "TheMonoGameTeam"),
            (
                "Moonlight Game Streaming Project",
                "MoonlightGameStreamingProject",
            ),
            ("AGALWOOD", "AGALWOOD"),
            ("AGALWOOD", "AGALWOOD"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Mozilla", "Mozilla"),
            ("Nicholas H.Tollervey", "NicholasHTollervey"),
            ("Mullvad VPN", "MullvadVPN"),
            ("Mullvad VPN", "MullvadVPN"),
            ("Microsoft Corporation", "Microsoft"),
            ("canonical", "canonical"),
            ("The Mumble Developers", "TheMumbleDevelopers"),
            ("The Mumble Developers", "TheMumbleDevelopers"),
            ("The Mumble Developers", "TheMumbleDevelopers"),
            ("Werner Schweer and Others", "WernerSchweerandOthers"),
            ("Youta Tec", "YoutaTec"),
            ("Logitech", "Logitech"),
            (
                "Martin Renold and the MyPaint Development Team",
                "MartinRenoldandtheMyPaintDevelopmentTeam",
            ),
            ("Oracle Corporation", "Oracle"),
            ("Feodor2", "Feodor"),
            ("Justin Aquadro", "JustinAquadro"),
            ("Rico Suter", "RicoSuter"),
            ("NV Access", "NVAccess"),
            ("NVIDIA Corporation", "NVIDIA"),
            ("NVIDIA Corporation", "NVIDIA"),
            ("NZXT, Inc.", "NZXT"),
            ("NZXT, Inc.", "NZXT"),
            ("NZXT, Inc.", "NZXT"),
            ("NZXT, Inc.", "NZXT"),
            ("NZXT, Inc.", "NZXT"),
            ("NZXT, Inc.", "NZXT"),
            ("Allan CORNET", "AllanCORNET"),
            ("Neotys", "Neotys"),
            ("Lutz Roeder", "LutzRoeder"),
            ("Nitro", "Nitro"),
            ("Nmap Project", "NmapProject"),
            ("qinghai", "qinghai"),
            ("qinghai", "qinghai"),
            ("qinghai", "qinghai"),
            ("Node.js Foundation", "NodejsFoundation"),
            ("Nodist", "Nodist"),
            ("NordVPN", "NordVPN"),
            ("TEFINCOM S.A.", "TEFINCOM"),
            ("NordVPN", "NordVPN"),
            ("NordVPN", "NordVPN"),
            ("CodingRoad", "CodingRoad"),
            ("XhmikosR", "XhmikosR"),
            ("Notion Labs, Incorporated", "NotionLabs"),
            ("Notion Labs, Incorporated", "NotionLabs"),
            ("Nmap Project", "NmapProject"),
            ("NullNoname", "NullNoname"),
            ("Nullsoft and Contributors", "NullsoftandContributors"),
            ("OBS Project", "OBSProject"),
            ("Hamster Republic Productions", "HamsterRepublicProductions"),
            ("Ascensio System SIA", "AscensioSystemSIA"),
            ("GNU Octave", "GNUOctave"),
            ("WetHat Lab", "WetHatLab"),
            ("Open Shop Channel", "OpenShopChannel"),
            ("The Open-Shell Team", "TheOpenShellTeam"),
            ("namazso", "namazso"),
            ("namazso", "namazso"),
            (
                "ojdkbuild open-source project",
                "ojdkbuildopensourceproject",
            ),
            (
                "ojdkbuild open-source project",
                "ojdkbuildopensourceproject",
            ),
            (
                "ojdkbuild open-source project",
                "ojdkbuildopensourceproject",
            ),
            (
                "ojdkbuild open-source project",
                "ojdkbuildopensourceproject",
            ),
            ("OpenMPT Devs", "OpenMPTDevs"),
            ("Apache Software Foundation", "ApacheSoftwareFoundation"),
            ("OpenRA developers", "OpenRAdevelopers"),
            ("The OpenSCAD Developers", "TheOpenSCADDevelopers"),
            ("Shining Light Productions", "ShiningLightProductions"),
            ("OpenShot Studios, LLC", "OpenShotStudios"),
            ("OpenTTD", "OpenTTD"),
            ("OpenTTD", "OpenTTD"),
            ("OpenVPN Technologies, Inc.", "OpenVPNTechnologies"),
            ("SparkLabs Pty Ltd", "SparkLabs"),
            ("OpenVPN Technologies", "OpenVPNTechnologies"),
            ("Opera Software", "OperaSoftware"),
            ("Opera Software", "OperaSoftware"),
            ("Opera Software", "OperaSoftware"),
            ("Opera Software", "OperaSoftware"),
            ("Opera Software", "OperaSoftware"),
            ("Opera Software", "OperaSoftware"),
            ("Opera Software", "OperaSoftware"),
            ("Opera Software", "OperaSoftware"),
            ("Oracle Corporation", "Oracle"),
            ("Oracle Corporation", "Oracle"),
            ("Oracle Corporation", "Oracle"),
            ("Oracle Corporation", "Oracle"),
            ("OutSystems", "OutSystems"),
            ("EXP Systems LLC", "EXPSystems"),
            (
                "Sober Lemur S.a.s. di Vacondio Andrea",
                "SoberLemurSasdiVacondioAndrea",
            ),
            ("CCPKU", "CCPKU"),
            ("Jan Fiala", "JanFiala"),
            ("NagleCode, LLC", "NagleCode"),
            ("Moonchild Productions", "MoonchildProductions"),
            ("Moonchild Productions", "MoonchildProductions"),
            ("John MacFarlane", "JohnMacFarlane"),
            ("John MacFarlane", "JohnMacFarlane"),
            ("John MacFarlane", "JohnMacFarlane"),
            ("Paradox Interactive", "ParadoxInteractive"),
            ("Paragon Software GmbH", "ParagonSoftware"),
            ("Parsec Cloud Inc.", "ParsecCloud"),
            ("Francesco Sorge", "FrancescoSorge"),
            ("Giorgio Tani", "GiorgioTani"),
            ("Giorgio Tani", "GiorgioTani"),
            ("Persepolis Team", "PersepolisTeam"),
            ("Heiko Sommerfeldt", "HeikoSommerfeldt"),
            ("touchbyte GmbH", "touchbyte"),
            ("NGWIN", "NGWIN"),
            ("PicoTorrent contributors.", "PicoTorrentcontributors"),
            ("blupi.org", "blupiorg"),
            (
                "Sony Interactive Entertainment Network America LLC",
                "SonyInteractiveEntertainmentNetworkAmerica",
            ),
            ("Josef Nemec", "JosefNemec"),
            ("Plex, Inc.", "Plex"),
            ("Plex", "Plex"),
            ("Plex, Inc.", "Plex"),
            ("Plex, Inc.", "Plex"),
            ("Plex, Inc.", "Plex"),
            ("Plex, Inc.", "Plex"),
            ("www.pokerth.net", "wwwpokerthnet"),
            ("Postbox, Inc.", "Postbox"),
            (
                "PostgreSQL Global Development Group",
                "PostgreSQLGlobalDevelopmentGroup",
            ),
            (
                "PostgreSQL Global Development Group",
                "PostgreSQLGlobalDevelopmentGroup",
            ),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Ironman Software, LLC", "IronmanSoftware"),
            ("Microsoft", "Microsoft"),
            ("Kim Walisch", "KimWalisch"),
            ("Private Internet Access, Inc.", "PrivateInternetAccess"),
            ("Progress Software Corporation", "ProgressSoftware"),
            ("Microsoft Corporation", "Microsoft"),
            ("Proton Technologies AG", "ProtonTechnologies"),
            ("Proton Technologies AG", "ProtonTechnologies"),
            ("Simon Tatham", "SimonTatham"),
            ("Puppet Labs", "PuppetLabs"),
            ("Puppet Inc", "Puppet"),
            ("Puppet, Inc.", "Puppet"),
            ("Puppet Inc", "Puppet"),
            ("Miller Puckette", "MillerPuckette"),
            ("Lancaster University Physics", "LancasterUniversityPhysics"),
            ("Python Software Foundation", "PythonSoftwareFoundation"),
            ("Hubert Pham", "HubertPham"),
            ("Python Software Foundation", "PythonSoftwareFoundation"),
            ("Python Software Foundation", "PythonSoftwareFoundation"),
            ("Python Software Foundation", "PythonSoftwareFoundation"),
            ("Python Software Foundation", "PythonSoftwareFoundation"),
            ("Python Software Foundation", "PythonSoftwareFoundation"),
            ("Python Software Foundation", "PythonSoftwareFoundation"),
            ("Python Software Foundation", "PythonSoftwareFoundation"),
            ("Python Software Foundation", "PythonSoftwareFoundation"),
            ("QGIS Development Team", "QGISDevelopmentTeam"),
            ("QGIS Development Team", "QGISDevelopmentTeam"),
            ("QGIS Development Team", "QGISDevelopmentTeam"),
            ("Michael Hansen", "MichaelHansen"),
            ("Hanna Knutsson", "HannaKnutsson"),
            ("LarusStone", "LarusStone"),
            ("Module Art", "ModuleArt"),
            ("Paddy Xu", "PaddyXu"),
            ("Quicken", "Quicken"),
            ("R Core Team", "RCoreTeam"),
            ("R Core Team", "RCoreTeam"),
            ("R Core Team", "RCoreTeam"),
            ("RStudio", "RStudio"),
            ("Rambox", "Rambox"),
            ("Oleg Danilov", "OlegDanilov"),
            ("Raspberry Pi", "RaspberryPi"),
            ("rawtherapee.com", "rawtherapeecom"),
            ("Paul Rawnsley", "PaulRawnsley"),
            ("jklSoft", "jklSoft"),
            ("Devolutions inc.", "Devolutions"),
            ("Devolutions inc.", "Devolutions"),
            ("Remote Mouse", "RemoteMouse"),
            ("Baldur Karlsson", "BaldurKarlsson"),
            ("Antmicro", "Antmicro"),
            ("Responsively", "Responsively"),
            ("RetroShare Team", "RetroShareTeam"),
            ("VS Revo Group, Ltd.", "VSRevoGroup"),
            ("VS Revo Group, Ltd.", "VSRevoGroup"),
            ("3T Software Labs Ltd", "3TSoftwareLabs"),
            ("3T Software Labs Ltd", "3TSoftwareLabs"),
            ("3T Software Labs Ltd", "3TSoftwareLabs"),
            ("Rocket.Chat Support", "RocketChatSupport"),
            ("Punk Software", "PunkSoftware"),
            ("Artsoft Entertainment", "ArtsoftEntertainment"),
            ("MarkoBL", "MarkoBL"),
            ("code4ward GmbH", "code4ward"),
            ("Royal Apps GmbH", "RoyalApps"),
            ("The R Foundation", "TheRFoundation"),
            ("RubyInstaller Team", "RubyInstallerTeam"),
            ("RubyInstaller Team", "RubyInstallerTeam"),
            ("RubyInstaller Team", "RubyInstallerTeam"),
            ("Luke Haas", "LukeHaas"),
            ("Luke Haas", "LukeHaas"),
            ("The Rust Project Developers", "TheRustProjectDevelopers"),
            ("The Rust Project Developers", "TheRustProjectDevelopers"),
            ("The Rust Project Developers", "TheRustProjectDevelopers"),
            ("The Rust Project Developers", "TheRustProjectDevelopers"),
            ("The Rust Project Developers", "TheRustProjectDevelopers"),
            ("The Rust Project Developers", "TheRustProjectDevelopers"),
            ("The Rust Project Developers", "TheRustProjectDevelopers"),
            ("The Rust Project Developers", "TheRustProjectDevelopers"),
            ("The Rust Project Developers", "TheRustProjectDevelopers"),
            ("Topala Software Solutions", "TopalaSoftwareSolutions"),
            ("Ricardo Villalba", "RicardoVillalba"),
            ("Navimatics LLC", "Navimatics"),
            ("Dotz Softwares", "DotzSoftwares"),
            ("Samsung Electronics Co., Ltd.", "SamsungElectronics"),
            ("mircearoata", "mircearoata"),
            ("Scratch Foundation", "ScratchFoundation"),
            ("Nicke Manarin", "NickeManarin"),
            ("The Scribus Team", "TheScribusTeam"),
            ("The ScummVM Team", "TheScummVMTeam"),
            ("spikespaz", "spikespaz"),
            ("Sejda BV", "Sejda"),
            ("Datalust Pty Ltd", "Datalust"),
            ("Microsoft Corporation", "Microsoft"),
            ("ShareX Team", "ShareXTeam"),
            ("RandyRants.com", "RandyRantscom"),
            ("Meltytech, LLC", "Meltytech"),
            ("Sigil-Ebook", "SigilEbook"),
            ("Open Whisper Systems", "OpenWhisperSystems"),
            ("Open Whisper Systems", "OpenWhisperSystems"),
            ("Open Whisper Systems", "OpenWhisperSystems"),
            ("Open Whisper Systems", "OpenWhisperSystems"),
            ("Open Whisper Systems", "OpenWhisperSystems"),
            ("Open Whisper Systems", "OpenWhisperSystems"),
            ("Open Whisper Systems", "OpenWhisperSystems"),
            ("Approximatrix, LLC", "Approximatrix"),
            ("Ashley Stone", "AshleyStone"),
            ("Skype Technologies S.A.", "SkypeTechnologies"),
            ("Skype Technologies S.A.", "SkypeTechnologies"),
            ("Skype Technologies S.A.", "SkypeTechnologies"),
            ("Slack Technologies", "SlackTechnologies"),
            ("TechSmith Corporation", "TechSmith"),
            ("SnakeNest.com", "SnakeNestcom"),
            ("Cory Plotts", "CoryPlotts"),
            ("SmartBear Software", "SmartBearSoftware"),
            ("Sonic Pi", "SonicPi"),
            ("Sonos, Inc.", "Sonos"),
            ("Sonos, Inc.", "Sonos"),
            ("Antoine Aflalo", "AntoineAflalo"),
            ("Atlassian", "Atlassian"),
            ("SpeedCrunch", "SpeedCrunch"),
            ("Spek Project", "SpekProject"),
            ("Standard Notes", "StandardNotes"),
            ("Valve Corporation", "Valve"),
            ("http://www.sbcl.org", "wwwsbclorg"),
            ("SteelSeries ApS", "SteelSeries"),
            ("Stellarium team", "Stellariumteam"),
            ("Stellarium team", "Stellariumteam"),
            ("Stellarium team", "Stellariumteam"),
            ("strawberryperl.com project", "strawberryperlcomproject"),
            ("General Workings, Inc.", "GeneralWorkings"),
            ("General Workings, Inc.", "GeneralWorkings"),
            ("General Workings, Inc.", "GeneralWorkings"),
            ("Streamlink", "Streamlink"),
            ("Sebastian Meyer", "SebastianMeyer"),
            ("Jan Hovancik", "JanHovancik"),
            ("Jan Hovancik", "JanHovancik"),
            ("Jan Hovancik", "JanHovancik"),
            ("Stride", "Stride"),
            ("BrickLink Corporation", "BrickLink"),
            ("Sublime HQ Pty Ltd", "SublimeHQ"),
            ("Sublime HQ Pty Ltd", "SublimeHQ"),
            ("Krzysztof Kowalczyk", "KrzysztofKowalczyk"),
            ("SuperCollider Community", "SuperColliderCommunity"),
            ("SuperTuxKart", "SuperTuxKart"),
            ("Microsoft", "Microsoft"),
            ("SyncTrayzor", "SyncTrayzor"),
            ("Mister Group", "MisterGroup"),
            ("OpenVPN Technologies, Inc.", "OpenVPNTechnologies"),
            ("erengy", "erengy"),
            ("Tailscale Inc.", "Tailscale"),
            ("Tailscale Inc.", "Tailscale"),
            ("Taisei Project", "TaiseiProject"),
            ("Taskcade Inc.", "Taskcade"),
            ("Taskcade Inc.", "Taskcade"),
            ("\"Treasure Data, Inc\"", "TreasureData"),
            ("Treasure Data, Inc", "TreasureData"),
            ("Benito van der Zander", "BenitovanderZander"),
            ("TeX Users Group", "TeXUsersGroup"),
            ("TeamSpeak Systems GmbH", "TeamSpeakSystems"),
            ("TechPowerUp", "TechPowerUp"),
            ("Telegram FZ-LLC", "TelegramFZ"),
            ("Telegram FZ-LLC", "TelegramFZ"),
            ("Telegram FZ-LLC", "TelegramFZ"),
            ("Telegram FZ-LLC", "TelegramFZ"),
            ("Telegram FZ-LLC", "TelegramFZ"),
            ("Telegram FZ-LLC", "TelegramFZ"),
            ("Telegram FZ-LLC", "TelegramFZ"),
            ("Telegram FZ-LLC", "TelegramFZ"),
            ("Telegram FZ-LLC", "TelegramFZ"),
            ("TeraTerm Project", "TeraTermProject"),
            ("Eugene Pankov", "EugenePankov"),
            ("Eugene Pankov", "EugenePankov"),
            ("Eugene Pankov", "EugenePankov"),
            ("CompuPhase", "CompuPhase"),
            ("Tesseract-OCR community", "TesseractOCRcommunity"),
            ("Texmaker", "Texmaker"),
            ("Texnomic", "Texnomic"),
            ("RaMMicHaeL", "RaMMicHaeL"),
            ("Appest.com", "Appestcom"),
            ("GlavSoft LLC.", "GlavSoft"),
            ("TikzEdt", "TikzEdt"),
            ("mapeditor.org", "mapeditororg"),
            ("Nadeo", "Nadeo"),
            ("Toggl", "Toggl"),
            ("Toggl", "Toggl"),
            ("TortoiseGit", "TortoiseGit"),
            ("TortoiseSVN", "TortoiseSVN"),
            ("TortoiseSVN", "TortoiseSVN"),
            (
                "TranslucentTB Open Source Developers",
                "TranslucentTBOpenSourceDevelopers",
            ),
            ("Transmission Project", "TransmissionProject"),
            (
                "Yury Sidorov & Transmission Remote GUI working group",
                "YurySidorovTransmissionRemoteGUIworkinggroup",
            ),
            ("Binary Fortress Software", "BinaryFortressSoftware"),
            ("Binary Fortress Software", "BinaryFortressSoftware"),
            ("JAM Software", "JAMSoftware"),
            ("JAM Software", "JAMSoftware"),
            ("Trelby.org", "Trelbyorg"),
            ("Cerulean Studios, LLC", "CeruleanStudios"),
            ("TunnelBear", "TunnelBear"),
            ("New Breed Software", "NewBreedSoftware"),
            ("Inspect Element Inc.", "InspectElement"),
            ("Xander Frangos", "XanderFrangos"),
            ("Twitch Interactive, Inc.", "TwitchInteractive"),
            ("G DATA CyberDefense AG", "GDATACyberDefense"),
            ("SafelyRemove.com", "SafelyRemovecom"),
            ("Drew Naylor", "DrewNaylor"),
            ("Drew Naylor", "DrewNaylor"),
            ("Ultimaker B.V.", "Ultimaker"),
            ("Ultimaker B.V.", "Ultimaker"),
            ("Ultimaker B.V.", "Ultimaker"),
            ("uvnc bvba", "uvncbvba"),
            ("Reason Software Company Inc.", "ReasonSoftware"),
            ("Unified Intents AB", "UnifiedIntents"),
            ("Unity Technologies Inc.", "UnityTechnologies"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Ubisoft", "Ubisoft"),
            ("UMEZAWA Takeshi", "UMEZAWATakeshi"),
            ("VCV", "VCV"),
            ("VideoLAN", "VideoLAN"),
            ("VideoLAN", "VideoLAN"),
            ("VideoLAN", "VideoLAN"),
            ("VMware, Inc.", "VMware"),
            ("VMware, Inc.", "VMware"),
            ("VMware, Inc.", "VMware"),
            ("RealVNC Ltd", "RealVNC"),
            ("RealVNC Ltd", "RealVNC"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("marha@users.sourceforge.net", "marhauserssourceforgenet"),
            ("Bram Moolenaar et al.", "BramMoolenaaretal"),
            ("Virt Manager Project", "VirtManagerProject"),
            ("Vivaldi Technologies AS.", "VivaldiTechnologies"),
            ("Black Tree Gaming Ltd.", "BlackTreeGaming"),
            ("VoyagerX, Inc.", "VoyagerX"),
            ("LunarG, Inc.", "LunarG"),
            ("Warzone 2100 Project", "WarzoneProject"),
            ("Waterfox", "Waterfox"),
            ("Waterfox", "Waterfox"),
            ("Waterfox", "Waterfox"),
            ("Devolutions Inc.", "Devolutions"),
            ("Buds", "Buds"),
            ("Buds", "Buds"),
            ("Buds", "Buds"),
            ("Buds", "Buds"),
            (
                "Machine Learning Group, University of Waikato, Hamilton, NZ",
                "MachineLearningGroupUniversityofWaikatoHamiltonNZ",
            ),
            ("ImageWriter Developers", "ImageWriterDevelopers"),
            ("Sam Hocevar", "SamHocevar"),
            ("Timothy Johnson", "TimothyJohnson"),
            ("Timothy Johnson", "TimothyJohnson"),
            ("Timothy Johnson", "TimothyJohnson"),
            ("Navimatics LLC", "Navimatics"),
            ("HTTrack", "HTTrack"),
            ("Thingamahoochie Software", "ThingamahoochieSoftware"),
            ("win.rar GmbH", "winrar"),
            ("win.rar GmbH", "winrar"),
            ("Martin Prikryl", "MartinPrikryl"),
            ("Corel Corporation", "Corel"),
            ("Winamp SA", "Winamp"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Dynastream Innovations, Inc.", "DynastreamInnovations"),
            ("Silicon Labs Software", "SiliconLabsSoftware"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("Microsoft Corporation", "Microsoft"),
            ("WireGuard LLC", "WireGuard"),
            (
                "The Wireshark developer community, https://www.wireshark.org",
                "TheWiresharkdevelopercommunitywwwwiresharkorg",
            ),
            (
                "The Wireshark developer community, https://www.wireshark.org",
                "TheWiresharkdevelopercommunitywwwwiresharkorg",
            ),
            (
                "The Wireshark developer community, https://www.wireshark.org",
                "TheWiresharkdevelopercommunitywwwwiresharkorg",
            ),
            (
                "The Wireshark developer community, https://www.wireshark.org",
                "TheWiresharkdevelopercommunitywwwwiresharkorg",
            ),
            ("Antibody Software", "AntibodySoftware"),
            ("Antibody Software", "AntibodySoftware"),
            ("Antibody Software", "AntibodySoftware"),
            ("Antibody Software", "AntibodySoftware"),
            ("Antibody Software", "AntibodySoftware"),
            ("Automattic Inc.", "Automattic"),
            ("Automattic Inc.", "Automattic"),
            ("Automattic Inc.", "Automattic"),
            ("Automattic Inc.", "Automattic"),
            ("Rob Caelers & Raymond Penners", "RobCaelersRaymondPenners"),
            ("Writage", "Writage"),
            ("X2Go Project", "X2GoProject"),
            ("Bitnami", "Bitnami"),
            ("Christian Hohnstaedt", "ChristianHohnstaedt"),
            ("The TBOOX Open Source Group", "TheTBOOXOpenSourceGroup"),
            ("XMind Ltd.", "XMind"),
            ("Gougelet Pierre-e", "GougeletPierree"),
            ("Gougelet Pierre-e", "GougeletPierree"),
            ("Yarn Contributors", "YarnContributors"),
            (
                "Beijing Yinxiang Biji Technologies Co., Ltd.",
                "BeijingYinxiangBijiTechnologies",
            ),
            (
                "Beijing Yinxiang Biji Technologies Co., Ltd.",
                "BeijingYinxiangBijiTechnologies",
            ),
            (
                "Beijing Yinxiang Biji Technologies Co., Ltd.",
                "BeijingYinxiangBijiTechnologies",
            ),
            (
                "Beijing Yinxiang Biji Technologies Co., Ltd.",
                "BeijingYinxiangBijiTechnologies",
            ),
            (
                "Beijing Yinxiang Biji Technologies Co., Ltd.",
                "BeijingYinxiangBijiTechnologies",
            ),
            ("Adler Luiz", "AdlerLuiz"),
            ("Yubico AB", "Yubico"),
            ("Zentimo.com", "Zentimocom"),
            ("ZeroTier, Inc.", "ZeroTier"),
            ("Hendrik Erz", "HendrikErz"),
            ("Robin Stuart & BogDan Vatra", "RobinStuartBogDanVatra"),
            ("Zoom", "Zoom"),
            ("Zoom", "Zoom"),
            (
                "Corporation for Digital Scholarship",
                "CorporationforDigitalScholarship",
            ),
            ("Kandra Labs, Inc.", "KandraLabs"),
            ("Sangoma Technologies Corp.", "SangomaTechnologies"),
            ("Zygor Guides", "ZygorGuides"),
            ("Balena Inc.", "Balena"),
            ("Balena Inc.", "Balena"),
            ("Balena Inc.", "Balena"),
            ("Balena Inc.", "Balena"),
            ("Balena Inc.", "Balena"),
            ("Balena Inc.", "Balena"),
            ("Balena Inc.", "Balena"),
            ("Balena Inc.", "Balena"),
            ("Nathaniel Johns", "NathanielJohns"),
            ("Clement Tsang", "ClementTsang"),
            ("butterflow-ui @ github", "butterflowuigithub"),
            ("Kovid Goyal", "KovidGoyal"),
            ("Elliott Zheng", "ElliottZheng"),
            ("the darktable project", "thedarktableproject"),
            (
                "dnGrep Community Contributors",
                "dnGrepCommunityContributors",
            ),
            ("JGraph", "JGraph"),
            ("Hardcoded Software", "HardcodedSoftware"),
            ("eM Client Inc.", "eMClient"),
            ("Ebbflow.io", "Ebbflowio"),
            ("f.lux Software LLC", "fluxSoftware"),
            ("Kurata Sayuri", "KurataSayuri"),
            ("wereturtle", "wereturtle"),
            ("gnuplot development team", "gnuplotdevelopmentteam"),
            ("Stefans Tools", "StefansTools"),
            ("Drew Naylor", "DrewNaylor"),
            ("Drew Naylor", "DrewNaylor"),
            ("Drew Naylor", "DrewNaylor"),
            ("eVenture Limited", "eVenture"),
            ("eVenture Limited", "eVenture"),
            ("PurpleI2P", "PurpleI2P"),
            ("Vantage Linguistics", "VantageLinguistics"),
            ("Apple Inc.", "Apple"),
            ("KDE e.V.", "KDE"),
            ("KDE e.V.", "KDE"),
            ("Next Generation Software", "NextGenerationSoftware"),
            ("Maxima Team", "MaximaTeam"),
            ("Frank Skare (stax76)", "FrankSkare"),
            ("720kb", "720kb"),
            ("xiles", "xiles"),
            ("ownCloud GmbH", "ownCloud"),
            ("EasternGraphics", "EasternGraphics"),
            ("Laurent P. Ren© de Cotret", "LaurentPRendeCotret"),
            ("The pgAdmin Development Team", "ThepgAdminDevelopmentTeam"),
            ("The pgAdmin Development Team", "ThepgAdminDevelopmentTeam"),
            ("The pgAdmin Development Team", "ThepgAdminDevelopmentTeam"),
            ("The pgAdmin Development Team", "ThepgAdminDevelopmentTeam"),
            ("The qBittorrent project", "TheqBittorrentproject"),
            ("The qBittorrent project", "TheqBittorrentproject"),
            ("The qBittorrent project", "TheqBittorrentproject"),
            ("remote.it", "remoteit"),
            ("remote.it", "remoteit"),
            ("Lightbend, Inc.", "Lightbend"),
            ("Scilab Enterprises", "ScilabEnterprises"),
            ("The Sqlectron Team", "TheSqlectronTeam"),
            ("Jan Hovancik", "JanHovancik"),
            ("Oliver Schwendener", "OliverSchwendener"),
            ("Oliver Schwendener", "OliverSchwendener"),
            ("Oliver Schwendener", "OliverSchwendener"),
            ("Humanity", "Humanity"),
            ("Humanity", "Humanity"),
            ("Microsoft Corporation", "Microsoft"),
            ("Alipay.com Co., Ltd.", "Alipaycom"),
            (
                "百度在线网络技术（北京）有限公司",
                "百度在线网络技术有限公司",
            ),
            ("腾讯科技(深圳)有限公司", "腾讯科技有限公司"),
        ];
        for (index, (publisher, expected)) in cases.into_iter().enumerate() {
            let folded = icu_casemap::CaseMapper::new().fold_string(expected);
            let expected =
                icu_normalizer::ComposingNormalizerBorrowed::new_nfkc().normalize(&folded);
            assert_eq!(
                normalize_publisher(publisher),
                expected,
                "case {}: {publisher:?}",
                index + 1
            );
        }
    }

    #[rstest]
    #[case("Example ᴸᴸᶜ Tools", "example")]
    #[case("Example ᴵᴺᶜ Tools", "example")]
    #[case("℉", "F")]
    #[case("℡", "TEL")]
    #[case("Example (a\nb) Tools", "exampletools")]
    #[case("Example [a\nb] Tools", "exampletools")]
    #[case("Example \"a\nb\" Tools", "exampleabtools")]
    #[case("Example \"a\rb\" Tools", "exampleabtools")]
    #[case("Example \"a\u{85}b\" Tools", "exampleabtools")]
    #[case("Example \u{345}", "exampleι")]
    #[case("Av1.2 V3", "av")]
    #[case("a 1 2 b", "ab")]
    #[case("LLC Inc Tools", "llc")]
    #[case("a1.2v3", "a")]
    #[case("dataflare", "dataflare")]
    #[case("Dataflare", "dataflare")]
    #[case("Example LLC Tools", "example")]
    #[case("LLC Tools", "llctools")]
    #[case("Company", "company")]
    #[case("Example, Inc. Corporation", "example")]
    #[case("Some Publisher Corp", "somepublisher")]
    #[case("((Example))", "example")]
    #[case("Example (Outer (Inner))", "example")]
    #[case("Example [Other] Tools", "exampletools")]
    #[case("Example \"Other\" Tools", "exampletools")]
    #[case("Example version 2.1.7", "example")]
    #[case("Example v2", "example")]
    #[case("Example 123 Tools", "exampletools")]
    #[case("Example123", "example")]
    #[case("123Example", "123example")]
    #[case("https://example.com", "examplecom")]
    #[case("myhttps://example.com", "myhttpsexamplecom")]
    #[case("A.B.C. Tools", "abctools")]
    #[case("A/B/C Tools", "abctools")]
    #[case("Straße ＬＬＣ Tools", "strasse")]
    #[case("éé@@suffix", "éésuffix")]
    #[case("ééé@@suffix", "ééé")]
    #[case("😀a@@suffix", "a")]
    #[case("", "")]
    #[case("123", "")]
    fn publishers_match_winget_cli_rules(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(normalize_publisher(input), expected);
    }

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
