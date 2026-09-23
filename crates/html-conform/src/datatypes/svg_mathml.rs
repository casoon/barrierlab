//! `w:xml-name` and `w:svg-pathdata` — the two `w:*` datatypes the vendored
//! SVG 1.1/MathML 3 schema modules (`schema/svg11/`, `schema/mml3/`,
//! vendored by `xtask/vendor-svg-mathml.sh`) reference that the HTML5
//! schema's own modules never do (confirmed by a full-text `w:` grep across
//! every vendored `.rnc` file before implementing anything — see
//! `plan/DECISIONS.md`'s SVG/MathML entry).

/// `w:xml-name` (`nu.validator.datatype.XmlName`): XML 1.0's `Name`
/// production (§2.3) — `NameStartChar (NameChar)*` — using the *original*
/// XML 1.0 character classes (pre-XML-1.1/5th-edition simplification,
/// Unicode 2.0-era ranges), not a "modernized" reading. Every range below
/// is transcribed directly from `XmlName.java`'s `isNameStart`/
/// `isNameTrail` (verified complete: the extraction script's own range/
/// operator counts matched the source file's `&&`/`||` counts exactly
/// before this was written) — vnu-parity as the default, not a
/// hand-rolled approximation.
pub(crate) fn check_xml_name(value: &str) -> Result<(), String> {
    let mut chars = value.chars();
    match chars.next() {
        None => return Err("XML name must not be empty".to_string()),
        Some(first) if !is_xml_name_start_char(first) => {
            return Err(format!("{first:?} is not a valid XML name start character"));
        }
        Some(_) => {}
    }
    if let Some(bad) = chars.find(|&c| !is_xml_name_trail_char(c)) {
        return Err(format!("{bad:?} is not a valid XML name character"));
    }
    Ok(())
}

fn is_xml_name_start_char(c: char) -> bool {
    matches!(
        c,
        '\u{0041}'..='\u{005A}' | '\u{0061}'..='\u{007A}' | '\u{00C0}'..='\u{00D6}' | '\u{00D8}'..='\u{00F6}' |
        '\u{00F8}'..='\u{00FF}' | '\u{0100}'..='\u{0131}' | '\u{0134}'..='\u{013E}' | '\u{0141}'..='\u{0148}' |
        '\u{014A}'..='\u{017E}' | '\u{0180}'..='\u{01C3}' | '\u{01CD}'..='\u{01F0}' | '\u{01F4}'..='\u{01F5}' |
        '\u{01FA}'..='\u{0217}' | '\u{0250}'..='\u{02A8}' | '\u{02BB}'..='\u{02C1}' | '\u{0388}'..='\u{038A}' |
        '\u{038E}'..='\u{03A1}' | '\u{03A3}'..='\u{03CE}' | '\u{03D0}'..='\u{03D6}' | '\u{03E2}'..='\u{03F3}' |
        '\u{0401}'..='\u{040C}' | '\u{040E}'..='\u{044F}' | '\u{0451}'..='\u{045C}' | '\u{045E}'..='\u{0481}' |
        '\u{0490}'..='\u{04C4}' | '\u{04C7}'..='\u{04C8}' | '\u{04CB}'..='\u{04CC}' | '\u{04D0}'..='\u{04EB}' |
        '\u{04EE}'..='\u{04F5}' | '\u{04F8}'..='\u{04F9}' | '\u{0531}'..='\u{0556}' | '\u{0561}'..='\u{0586}' |
        '\u{05D0}'..='\u{05EA}' | '\u{05F0}'..='\u{05F2}' | '\u{0621}'..='\u{063A}' | '\u{0641}'..='\u{064A}' |
        '\u{0671}'..='\u{06B7}' | '\u{06BA}'..='\u{06BE}' | '\u{06C0}'..='\u{06CE}' | '\u{06D0}'..='\u{06D3}' |
        '\u{06E5}'..='\u{06E6}' | '\u{0905}'..='\u{0939}' | '\u{0958}'..='\u{0961}' | '\u{0985}'..='\u{098C}' |
        '\u{098F}'..='\u{0990}' | '\u{0993}'..='\u{09A8}' | '\u{09AA}'..='\u{09B0}' | '\u{09B6}'..='\u{09B9}' |
        '\u{09DC}'..='\u{09DD}' | '\u{09DF}'..='\u{09E1}' | '\u{09F0}'..='\u{09F1}' | '\u{0A05}'..='\u{0A0A}' |
        '\u{0A0F}'..='\u{0A10}' | '\u{0A13}'..='\u{0A28}' | '\u{0A2A}'..='\u{0A30}' | '\u{0A32}'..='\u{0A33}' |
        '\u{0A35}'..='\u{0A36}' | '\u{0A38}'..='\u{0A39}' | '\u{0A59}'..='\u{0A5C}' | '\u{0A72}'..='\u{0A74}' |
        '\u{0A85}'..='\u{0A8B}' | '\u{0A8F}'..='\u{0A91}' | '\u{0A93}'..='\u{0AA8}' | '\u{0AAA}'..='\u{0AB0}' |
        '\u{0AB2}'..='\u{0AB3}' | '\u{0AB5}'..='\u{0AB9}' | '\u{0B05}'..='\u{0B0C}' | '\u{0B0F}'..='\u{0B10}' |
        '\u{0B13}'..='\u{0B28}' | '\u{0B2A}'..='\u{0B30}' | '\u{0B32}'..='\u{0B33}' | '\u{0B36}'..='\u{0B39}' |
        '\u{0B5C}'..='\u{0B5D}' | '\u{0B5F}'..='\u{0B61}' | '\u{0B85}'..='\u{0B8A}' | '\u{0B8E}'..='\u{0B90}' |
        '\u{0B92}'..='\u{0B95}' | '\u{0B99}'..='\u{0B9A}' | '\u{0B9E}'..='\u{0B9F}' | '\u{0BA3}'..='\u{0BA4}' |
        '\u{0BA8}'..='\u{0BAA}' | '\u{0BAE}'..='\u{0BB5}' | '\u{0BB7}'..='\u{0BB9}' | '\u{0C05}'..='\u{0C0C}' |
        '\u{0C0E}'..='\u{0C10}' | '\u{0C12}'..='\u{0C28}' | '\u{0C2A}'..='\u{0C33}' | '\u{0C35}'..='\u{0C39}' |
        '\u{0C60}'..='\u{0C61}' | '\u{0C85}'..='\u{0C8C}' | '\u{0C8E}'..='\u{0C90}' | '\u{0C92}'..='\u{0CA8}' |
        '\u{0CAA}'..='\u{0CB3}' | '\u{0CB5}'..='\u{0CB9}' | '\u{0CE0}'..='\u{0CE1}' | '\u{0D05}'..='\u{0D0C}' |
        '\u{0D0E}'..='\u{0D10}' | '\u{0D12}'..='\u{0D28}' | '\u{0D2A}'..='\u{0D39}' | '\u{0D60}'..='\u{0D61}' |
        '\u{0E01}'..='\u{0E2E}' | '\u{0E32}'..='\u{0E33}' | '\u{0E40}'..='\u{0E45}' | '\u{0E81}'..='\u{0E82}' |
        '\u{0E87}'..='\u{0E88}' | '\u{0E94}'..='\u{0E97}' | '\u{0E99}'..='\u{0E9F}' | '\u{0EA1}'..='\u{0EA3}' |
        '\u{0EAA}'..='\u{0EAB}' | '\u{0EAD}'..='\u{0EAE}' | '\u{0EB2}'..='\u{0EB3}' | '\u{0EC0}'..='\u{0EC4}' |
        '\u{0F40}'..='\u{0F47}' | '\u{0F49}'..='\u{0F69}' | '\u{10A0}'..='\u{10C5}' | '\u{10D0}'..='\u{10F6}' |
        '\u{1102}'..='\u{1103}' | '\u{1105}'..='\u{1107}' | '\u{110B}'..='\u{110C}' | '\u{110E}'..='\u{1112}' |
        '\u{1154}'..='\u{1155}' | '\u{115F}'..='\u{1161}' | '\u{116D}'..='\u{116E}' | '\u{1172}'..='\u{1173}' |
        '\u{11AE}'..='\u{11AF}' | '\u{11B7}'..='\u{11B8}' | '\u{11BC}'..='\u{11C2}' | '\u{1E00}'..='\u{1E9B}' |
        '\u{1EA0}'..='\u{1EF9}' | '\u{1F00}'..='\u{1F15}' | '\u{1F18}'..='\u{1F1D}' | '\u{1F20}'..='\u{1F45}' |
        '\u{1F48}'..='\u{1F4D}' | '\u{1F50}'..='\u{1F57}' | '\u{1F5F}'..='\u{1F7D}' | '\u{1F80}'..='\u{1FB4}' |
        '\u{1FB6}'..='\u{1FBC}' | '\u{1FC2}'..='\u{1FC4}' | '\u{1FC6}'..='\u{1FCC}' | '\u{1FD0}'..='\u{1FD3}' |
        '\u{1FD6}'..='\u{1FDB}' | '\u{1FE0}'..='\u{1FEC}' | '\u{1FF2}'..='\u{1FF4}' | '\u{1FF6}'..='\u{1FFC}' |
        '\u{212A}'..='\u{212B}' | '\u{2180}'..='\u{2182}' | '\u{3041}'..='\u{3094}' | '\u{30A1}'..='\u{30FA}' |
        '\u{3105}'..='\u{312C}' | '\u{AC00}'..='\u{D7A3}' | '\u{4E00}'..='\u{9FA5}' | '\u{3021}'..='\u{3029}' |
        '\u{0386}' | '\u{038C}' | '\u{03DA}' | '\u{03DC}' |
        '\u{03DE}' | '\u{03E0}' | '\u{0559}' | '\u{06D5}' |
        '\u{093D}' | '\u{09B2}' | '\u{0A5E}' | '\u{0A8D}' |
        '\u{0ABD}' | '\u{0AE0}' | '\u{0B3D}' | '\u{0B9C}' |
        '\u{0CDE}' | '\u{0E30}' | '\u{0E84}' | '\u{0E8A}' |
        '\u{0E8D}' | '\u{0EA5}' | '\u{0EA7}' | '\u{0EB0}' |
        '\u{0EBD}' | '\u{1100}' | '\u{1109}' | '\u{113C}' |
        '\u{113E}' | '\u{1140}' | '\u{114C}' | '\u{114E}' |
        '\u{1150}' | '\u{1159}' | '\u{1163}' | '\u{1165}' |
        '\u{1167}' | '\u{1169}' | '\u{1175}' | '\u{119E}' |
        '\u{11A8}' | '\u{11AB}' | '\u{11BA}' | '\u{11EB}' |
        '\u{11F0}' | '\u{11F9}' | '\u{1F59}' | '\u{1F5B}' |
        '\u{1F5D}' | '\u{1FBE}' | '\u{2126}' | '\u{212E}' |
        '\u{3007}' | '_' | ':'
    )
}

fn is_xml_name_trail_char(c: char) -> bool {
    matches!(
        c,
        '\u{0030}'..='\u{0039}' | '\u{0660}'..='\u{0669}' | '\u{06F0}'..='\u{06F9}' | '\u{0966}'..='\u{096F}' |
        '\u{09E6}'..='\u{09EF}' | '\u{0A66}'..='\u{0A6F}' | '\u{0AE6}'..='\u{0AEF}' | '\u{0B66}'..='\u{0B6F}' |
        '\u{0BE7}'..='\u{0BEF}' | '\u{0C66}'..='\u{0C6F}' | '\u{0CE6}'..='\u{0CEF}' | '\u{0D66}'..='\u{0D6F}' |
        '\u{0E50}'..='\u{0E59}' | '\u{0ED0}'..='\u{0ED9}' | '\u{0F20}'..='\u{0F29}' | '\u{0041}'..='\u{005A}' |
        '\u{0061}'..='\u{007A}' | '\u{00C0}'..='\u{00D6}' | '\u{00D8}'..='\u{00F6}' | '\u{00F8}'..='\u{00FF}' |
        '\u{0100}'..='\u{0131}' | '\u{0134}'..='\u{013E}' | '\u{0141}'..='\u{0148}' | '\u{014A}'..='\u{017E}' |
        '\u{0180}'..='\u{01C3}' | '\u{01CD}'..='\u{01F0}' | '\u{01F4}'..='\u{01F5}' | '\u{01FA}'..='\u{0217}' |
        '\u{0250}'..='\u{02A8}' | '\u{02BB}'..='\u{02C1}' | '\u{0388}'..='\u{038A}' | '\u{038E}'..='\u{03A1}' |
        '\u{03A3}'..='\u{03CE}' | '\u{03D0}'..='\u{03D6}' | '\u{03E2}'..='\u{03F3}' | '\u{0401}'..='\u{040C}' |
        '\u{040E}'..='\u{044F}' | '\u{0451}'..='\u{045C}' | '\u{045E}'..='\u{0481}' | '\u{0490}'..='\u{04C4}' |
        '\u{04C7}'..='\u{04C8}' | '\u{04CB}'..='\u{04CC}' | '\u{04D0}'..='\u{04EB}' | '\u{04EE}'..='\u{04F5}' |
        '\u{04F8}'..='\u{04F9}' | '\u{0531}'..='\u{0556}' | '\u{0561}'..='\u{0586}' | '\u{05D0}'..='\u{05EA}' |
        '\u{05F0}'..='\u{05F2}' | '\u{0621}'..='\u{063A}' | '\u{0641}'..='\u{064A}' | '\u{0671}'..='\u{06B7}' |
        '\u{06BA}'..='\u{06BE}' | '\u{06C0}'..='\u{06CE}' | '\u{06D0}'..='\u{06D3}' | '\u{06E5}'..='\u{06E6}' |
        '\u{0905}'..='\u{0939}' | '\u{0958}'..='\u{0961}' | '\u{0985}'..='\u{098C}' | '\u{098F}'..='\u{0990}' |
        '\u{0993}'..='\u{09A8}' | '\u{09AA}'..='\u{09B0}' | '\u{09B6}'..='\u{09B9}' | '\u{09DC}'..='\u{09DD}' |
        '\u{09DF}'..='\u{09E1}' | '\u{09F0}'..='\u{09F1}' | '\u{0A05}'..='\u{0A0A}' | '\u{0A0F}'..='\u{0A10}' |
        '\u{0A13}'..='\u{0A28}' | '\u{0A2A}'..='\u{0A30}' | '\u{0A32}'..='\u{0A33}' | '\u{0A35}'..='\u{0A36}' |
        '\u{0A38}'..='\u{0A39}' | '\u{0A59}'..='\u{0A5C}' | '\u{0A72}'..='\u{0A74}' | '\u{0A85}'..='\u{0A8B}' |
        '\u{0A8F}'..='\u{0A91}' | '\u{0A93}'..='\u{0AA8}' | '\u{0AAA}'..='\u{0AB0}' | '\u{0AB2}'..='\u{0AB3}' |
        '\u{0AB5}'..='\u{0AB9}' | '\u{0B05}'..='\u{0B0C}' | '\u{0B0F}'..='\u{0B10}' | '\u{0B13}'..='\u{0B28}' |
        '\u{0B2A}'..='\u{0B30}' | '\u{0B32}'..='\u{0B33}' | '\u{0B36}'..='\u{0B39}' | '\u{0B5C}'..='\u{0B5D}' |
        '\u{0B5F}'..='\u{0B61}' | '\u{0B85}'..='\u{0B8A}' | '\u{0B8E}'..='\u{0B90}' | '\u{0B92}'..='\u{0B95}' |
        '\u{0B99}'..='\u{0B9A}' | '\u{0B9E}'..='\u{0B9F}' | '\u{0BA3}'..='\u{0BA4}' | '\u{0BA8}'..='\u{0BAA}' |
        '\u{0BAE}'..='\u{0BB5}' | '\u{0BB7}'..='\u{0BB9}' | '\u{0C05}'..='\u{0C0C}' | '\u{0C0E}'..='\u{0C10}' |
        '\u{0C12}'..='\u{0C28}' | '\u{0C2A}'..='\u{0C33}' | '\u{0C35}'..='\u{0C39}' | '\u{0C60}'..='\u{0C61}' |
        '\u{0C85}'..='\u{0C8C}' | '\u{0C8E}'..='\u{0C90}' | '\u{0C92}'..='\u{0CA8}' | '\u{0CAA}'..='\u{0CB3}' |
        '\u{0CB5}'..='\u{0CB9}' | '\u{0CE0}'..='\u{0CE1}' | '\u{0D05}'..='\u{0D0C}' | '\u{0D0E}'..='\u{0D10}' |
        '\u{0D12}'..='\u{0D28}' | '\u{0D2A}'..='\u{0D39}' | '\u{0D60}'..='\u{0D61}' | '\u{0E01}'..='\u{0E2E}' |
        '\u{0E32}'..='\u{0E33}' | '\u{0E40}'..='\u{0E45}' | '\u{0E81}'..='\u{0E82}' | '\u{0E87}'..='\u{0E88}' |
        '\u{0E94}'..='\u{0E97}' | '\u{0E99}'..='\u{0E9F}' | '\u{0EA1}'..='\u{0EA3}' | '\u{0EAA}'..='\u{0EAB}' |
        '\u{0EAD}'..='\u{0EAE}' | '\u{0EB2}'..='\u{0EB3}' | '\u{0EC0}'..='\u{0EC4}' | '\u{0F40}'..='\u{0F47}' |
        '\u{0F49}'..='\u{0F69}' | '\u{10A0}'..='\u{10C5}' | '\u{10D0}'..='\u{10F6}' | '\u{1102}'..='\u{1103}' |
        '\u{1105}'..='\u{1107}' | '\u{110B}'..='\u{110C}' | '\u{110E}'..='\u{1112}' | '\u{1154}'..='\u{1155}' |
        '\u{115F}'..='\u{1161}' | '\u{116D}'..='\u{116E}' | '\u{1172}'..='\u{1173}' | '\u{11AE}'..='\u{11AF}' |
        '\u{11B7}'..='\u{11B8}' | '\u{11BC}'..='\u{11C2}' | '\u{1E00}'..='\u{1E9B}' | '\u{1EA0}'..='\u{1EF9}' |
        '\u{1F00}'..='\u{1F15}' | '\u{1F18}'..='\u{1F1D}' | '\u{1F20}'..='\u{1F45}' | '\u{1F48}'..='\u{1F4D}' |
        '\u{1F50}'..='\u{1F57}' | '\u{1F5F}'..='\u{1F7D}' | '\u{1F80}'..='\u{1FB4}' | '\u{1FB6}'..='\u{1FBC}' |
        '\u{1FC2}'..='\u{1FC4}' | '\u{1FC6}'..='\u{1FCC}' | '\u{1FD0}'..='\u{1FD3}' | '\u{1FD6}'..='\u{1FDB}' |
        '\u{1FE0}'..='\u{1FEC}' | '\u{1FF2}'..='\u{1FF4}' | '\u{1FF6}'..='\u{1FFC}' | '\u{212A}'..='\u{212B}' |
        '\u{2180}'..='\u{2182}' | '\u{3041}'..='\u{3094}' | '\u{30A1}'..='\u{30FA}' | '\u{3105}'..='\u{312C}' |
        '\u{AC00}'..='\u{D7A3}' | '\u{4E00}'..='\u{9FA5}' | '\u{3021}'..='\u{3029}' | '\u{0300}'..='\u{0345}' |
        '\u{0360}'..='\u{0361}' | '\u{0483}'..='\u{0486}' | '\u{0591}'..='\u{05A1}' | '\u{05A3}'..='\u{05B9}' |
        '\u{05BB}'..='\u{05BD}' | '\u{05C1}'..='\u{05C2}' | '\u{064B}'..='\u{0652}' | '\u{06D6}'..='\u{06DC}' |
        '\u{06DD}'..='\u{06DF}' | '\u{06E0}'..='\u{06E4}' | '\u{06E7}'..='\u{06E8}' | '\u{06EA}'..='\u{06ED}' |
        '\u{0901}'..='\u{0903}' | '\u{093E}'..='\u{094C}' | '\u{0951}'..='\u{0954}' | '\u{0962}'..='\u{0963}' |
        '\u{0981}'..='\u{0983}' | '\u{09C0}'..='\u{09C4}' | '\u{09C7}'..='\u{09C8}' | '\u{09CB}'..='\u{09CD}' |
        '\u{09E2}'..='\u{09E3}' | '\u{0A40}'..='\u{0A42}' | '\u{0A47}'..='\u{0A48}' | '\u{0A4B}'..='\u{0A4D}' |
        '\u{0A70}'..='\u{0A71}' | '\u{0A81}'..='\u{0A83}' | '\u{0ABE}'..='\u{0AC5}' | '\u{0AC7}'..='\u{0AC9}' |
        '\u{0ACB}'..='\u{0ACD}' | '\u{0B01}'..='\u{0B03}' | '\u{0B3E}'..='\u{0B43}' | '\u{0B47}'..='\u{0B48}' |
        '\u{0B4B}'..='\u{0B4D}' | '\u{0B56}'..='\u{0B57}' | '\u{0B82}'..='\u{0B83}' | '\u{0BBE}'..='\u{0BC2}' |
        '\u{0BC6}'..='\u{0BC8}' | '\u{0BCA}'..='\u{0BCD}' | '\u{0C01}'..='\u{0C03}' | '\u{0C3E}'..='\u{0C44}' |
        '\u{0C46}'..='\u{0C48}' | '\u{0C4A}'..='\u{0C4D}' | '\u{0C55}'..='\u{0C56}' | '\u{0C82}'..='\u{0C83}' |
        '\u{0CBE}'..='\u{0CC4}' | '\u{0CC6}'..='\u{0CC8}' | '\u{0CCA}'..='\u{0CCD}' | '\u{0CD5}'..='\u{0CD6}' |
        '\u{0D02}'..='\u{0D03}' | '\u{0D3E}'..='\u{0D43}' | '\u{0D46}'..='\u{0D48}' | '\u{0D4A}'..='\u{0D4D}' |
        '\u{0E34}'..='\u{0E3A}' | '\u{0E47}'..='\u{0E4E}' | '\u{0EB4}'..='\u{0EB9}' | '\u{0EBB}'..='\u{0EBC}' |
        '\u{0EC8}'..='\u{0ECD}' | '\u{0F18}'..='\u{0F19}' | '\u{0F71}'..='\u{0F84}' | '\u{0F86}'..='\u{0F8B}' |
        '\u{0F90}'..='\u{0F95}' | '\u{0F99}'..='\u{0FAD}' | '\u{0FB1}'..='\u{0FB7}' | '\u{20D0}'..='\u{20DC}' |
        '\u{302A}'..='\u{302F}' | '\u{3031}'..='\u{3035}' | '\u{309D}'..='\u{309E}' | '\u{30FC}'..='\u{30FE}' |
        '\u{0386}' | '\u{038C}' | '\u{03DA}' | '\u{03DC}' |
        '\u{03DE}' | '\u{03E0}' | '\u{0559}' | '\u{06D5}' |
        '\u{093D}' | '\u{09B2}' | '\u{0A5E}' | '\u{0A8D}' |
        '\u{0ABD}' | '\u{0AE0}' | '\u{0B3D}' | '\u{0B9C}' |
        '\u{0CDE}' | '\u{0E30}' | '\u{0E84}' | '\u{0E8A}' |
        '\u{0E8D}' | '\u{0EA5}' | '\u{0EA7}' | '\u{0EB0}' |
        '\u{0EBD}' | '\u{1100}' | '\u{1109}' | '\u{113C}' |
        '\u{113E}' | '\u{1140}' | '\u{114C}' | '\u{114E}' |
        '\u{1150}' | '\u{1159}' | '\u{1163}' | '\u{1165}' |
        '\u{1167}' | '\u{1169}' | '\u{1175}' | '\u{119E}' |
        '\u{11A8}' | '\u{11AB}' | '\u{11BA}' | '\u{11EB}' |
        '\u{11F0}' | '\u{11F9}' | '\u{1F59}' | '\u{1F5B}' |
        '\u{1F5D}' | '\u{1FBE}' | '\u{2126}' | '\u{212E}' |
        '\u{3007}' | '\u{05BF}' | '\u{05C4}' | '\u{0670}' |
        '\u{093C}' | '\u{094D}' | '\u{09BC}' | '\u{09BE}' |
        '\u{09BF}' | '\u{09D7}' | '\u{0A02}' | '\u{0A3C}' |
        '\u{0A3E}' | '\u{0A3F}' | '\u{0ABC}' | '\u{0B3C}' |
        '\u{0BD7}' | '\u{0D57}' | '\u{0E31}' | '\u{0EB1}' |
        '\u{0F35}' | '\u{0F37}' | '\u{0F39}' | '\u{0F3E}' |
        '\u{0F3F}' | '\u{0F97}' | '\u{0FB9}' | '\u{20E1}' |
        '\u{3099}' | '\u{309A}' | '\u{00B7}' | '\u{02D0}' |
        '\u{02D1}' | '\u{0387}' | '\u{0640}' | '\u{0E46}' |
        '\u{0EC6}' | '\u{3005}' | '_' | ':' |
        '.' | '-'
    )
}

// ---------------------------------------------------------------------
// w:svg-pathdata
// ---------------------------------------------------------------------

/// `w:svg-pathdata` (`nu.validator.datatype.SvgPathData`): the `<path>`
/// element's `d` attribute mini-language (SVG 1.1 §8.3.1's normative
/// grammar for `path-data`).
///
/// **Scope note, unlike every other `w:*` type in this library:** vnu's
/// own implementation is a ~1500-line hand-written, Apache-Batik-derived
/// character-by-character state machine, with its own specific
/// leniencies vnu-parity would normally mean replicating exactly. That
/// port isn't done here — this instead implements the SVG 1.1 spec's own
/// published EBNF grammar directly (moveto/lineto/curveto/arc commands,
/// numbers, flags, comma-`wsp` separators), which should agree with vnu
/// on essentially every well-formed or clearly-malformed input, but may
/// diverge from vnu's exact edge-case leniency in ways neither this
/// crate's own test suite nor the vendored corpus can currently surface
/// (zero corpus fixtures exercise SVG path-data content at all — see
/// `plan/DECISIONS.md`'s SVG/MathML entry). A documented, deliberate
/// narrower scope, not a silently-incomplete implementation.
pub(crate) fn check_svg_pathdata(value: &str) -> Result<(), String> {
    let mut parser = PathParser {
        bytes: value.as_bytes(),
        pos: 0,
    };
    parser.skip_wsp();
    if parser.at_end() {
        // SVG 1.1 §8.3.1: "A path data segment (if there is one) must
        // begin with a moveto command" — but an *empty* `d` (or one that
        // is entirely whitespace) is explicitly conforming ("If a path
        // data segment is provided but is empty ... the ... path element
        // is disabled"). Same leniency vnu documents for its own parser.
        return Ok(());
    }
    parser.moveto_drawto_command_groups()?;
    parser.skip_wsp();
    if !parser.at_end() {
        return Err(format!(
            "unexpected trailing content in path data at byte {}",
            parser.pos
        ));
    }
    Ok(())
}

struct PathParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl PathParser<'_> {
    fn at_end(&self) -> bool {
        self.pos >= self.bytes.len()
    }
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }
    fn advance(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }
    fn is_wsp(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\r' | b'\n')
    }
    fn skip_wsp(&mut self) {
        while self.peek().is_some_and(Self::is_wsp) {
            self.pos += 1;
        }
    }
    /// `comma-wsp ::= (wsp+ ","? wsp*) | ("," wsp*)` — optional between
    /// arguments; never required (the classic SVG path-data compaction
    /// relies on this — e.g. `L.5.5` needs no separator at all), so this
    /// never fails.
    fn skip_comma_wsp(&mut self) {
        self.skip_wsp();
        if self.peek() == Some(b',') {
            self.pos += 1;
            self.skip_wsp();
        }
    }

    fn moveto_drawto_command_groups(&mut self) -> Result<(), String> {
        loop {
            self.moveto_drawto_command_group()?;
            self.skip_wsp();
            if self.at_end() || !matches!(self.peek(), Some(b'M' | b'm')) {
                return Ok(());
            }
        }
    }

    fn moveto_drawto_command_group(&mut self) -> Result<(), String> {
        self.moveto()?;
        loop {
            self.skip_wsp();
            match self.peek() {
                Some(command) if is_drawto_command_letter(command) => {
                    self.drawto_command()?;
                }
                _ => return Ok(()),
            }
        }
    }

    fn moveto(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(b'M' | b'm') => {}
            other => return Err(format!("expected moveto command, found {other:?}")),
        }
        self.skip_wsp();
        self.coordinate_pair()?;
        // A moveto's argument sequence, beyond the first pair, is treated
        // as implicit `lineto` commands (SVG 1.1 §8.3.2) — repeatedly
        // consume coordinate pairs until the next thing isn't one.
        loop {
            let checkpoint = self.pos;
            self.skip_comma_wsp();
            if self.looks_like_number_start() {
                if self.coordinate_pair().is_err() {
                    self.pos = checkpoint;
                    return Ok(());
                }
            } else {
                self.pos = checkpoint;
                return Ok(());
            }
        }
    }

    fn drawto_command(&mut self) -> Result<(), String> {
        let command = self.advance().expect("caller already peeked a command");
        self.skip_wsp();
        match command {
            b'Z' | b'z' => Ok(()),
            b'L' | b'l' => self.repeat_while_arguments_follow(Self::coordinate_pair),
            b'H' | b'h' => self.repeat_while_arguments_follow(Self::number_arg),
            b'V' | b'v' => self.repeat_while_arguments_follow(Self::number_arg),
            b'C' | b'c' => self.repeat_while_arguments_follow(|p| {
                p.coordinate_pair()?;
                p.skip_comma_wsp();
                p.coordinate_pair()?;
                p.skip_comma_wsp();
                p.coordinate_pair()
            }),
            b'S' | b's' => self.repeat_while_arguments_follow(|p| {
                p.coordinate_pair()?;
                p.skip_comma_wsp();
                p.coordinate_pair()
            }),
            b'Q' | b'q' => self.repeat_while_arguments_follow(|p| {
                p.coordinate_pair()?;
                p.skip_comma_wsp();
                p.coordinate_pair()
            }),
            b'T' | b't' => self.repeat_while_arguments_follow(Self::coordinate_pair),
            b'A' | b'a' => self.repeat_while_arguments_follow(Self::elliptical_arc_argument),
            other => Err(format!("unknown path command {:?}", other as char)),
        }
    }

    /// Runs `one_argument` once, then keeps running it (skipping an
    /// optional `comma-wsp` between repetitions) as long as another
    /// argument plausibly follows — implements every `*-argument-sequence`
    /// production, which is "one or more arguments for the same command
    /// letter", not "exactly one".
    fn repeat_while_arguments_follow(
        &mut self,
        mut one_argument: impl FnMut(&mut Self) -> Result<(), String>,
    ) -> Result<(), String> {
        one_argument(self)?;
        loop {
            let checkpoint = self.pos;
            self.skip_comma_wsp();
            if self.looks_like_number_start() {
                if one_argument(self).is_err() {
                    self.pos = checkpoint;
                    return Ok(());
                }
            } else {
                self.pos = checkpoint;
                return Ok(());
            }
        }
    }

    fn looks_like_number_start(&self) -> bool {
        matches!(self.peek(), Some(b'0'..=b'9' | b'+' | b'-' | b'.'))
    }

    fn coordinate_pair(&mut self) -> Result<(), String> {
        self.number_arg()?;
        self.skip_comma_wsp();
        self.number_arg()
    }

    fn number_arg(&mut self) -> Result<(), String> {
        self.number().map(|_| ())
    }

    /// `elliptical-arc-argument ::= nonnegative-number comma-wsp?
    /// nonnegative-number comma-wsp? number comma-wsp flag comma-wsp?
    /// flag comma-wsp? coordinate-pair`
    fn elliptical_arc_argument(&mut self) -> Result<(), String> {
        self.nonnegative_number()?;
        self.skip_comma_wsp();
        self.nonnegative_number()?;
        self.skip_comma_wsp();
        self.number_arg()?;
        self.skip_comma_wsp();
        self.flag()?;
        self.skip_comma_wsp();
        self.flag()?;
        self.skip_comma_wsp();
        self.coordinate_pair()
    }

    fn flag(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(b'0') | Some(b'1') => Ok(()),
            other => Err(format!("expected a flag (0 or 1), found {other:?}")),
        }
    }

    fn nonnegative_number(&mut self) -> Result<f64, String> {
        if self.peek() == Some(b'-') {
            return Err("expected a non-negative number".to_string());
        }
        self.number()
    }

    /// `number ::= sign? integer-constant | sign? floating-point-constant`
    fn number(&mut self) -> Result<f64, String> {
        let start = self.pos;
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.pos += 1;
        }
        let mut saw_digit = false;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
            saw_digit = true;
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
                saw_digit = true;
            }
        }
        if !saw_digit {
            self.pos = start;
            return Err("expected a number".to_string());
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            let exponent_start = self.pos;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            let mut exponent_digit = false;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
                exponent_digit = true;
            }
            if !exponent_digit {
                // Not actually an exponent (e.g. a following command
                // letter like `e`... never happens for path commands,
                // but be conservative) — back off and let the `e`/`E`
                // be re-tokenized by whatever comes next.
                self.pos = exponent_start;
            }
        }
        // SAFETY-free: `bytes[start..pos]` is always ASCII (digits,
        // sign, `.`, `e`/`E`), so this is always valid UTF-8.
        std::str::from_utf8(&self.bytes[start..self.pos])
            .expect("path-data numbers are always ASCII")
            .parse::<f64>()
            .map_err(|_| "malformed number in path data".to_string())
    }
}

fn is_drawto_command_letter(b: u8) -> bool {
    matches!(
        b,
        b'Z' | b'z'
            | b'L'
            | b'l'
            | b'H'
            | b'h'
            | b'V'
            | b'v'
            | b'C'
            | b'c'
            | b'S'
            | b's'
            | b'Q'
            | b'q'
            | b'T'
            | b't'
            | b'A'
            | b'a'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- check_xml_name --

    #[test]
    fn xml_name_accepts_simple_ascii_name() {
        assert!(check_xml_name("foo").is_ok());
        assert!(check_xml_name("foo-bar.baz").is_ok());
        assert!(check_xml_name("_leading-underscore").is_ok());
        assert!(check_xml_name("with:colon").is_ok());
    }

    #[test]
    fn xml_name_rejects_empty() {
        assert!(check_xml_name("").is_err());
    }

    #[test]
    fn xml_name_rejects_leading_digit() {
        assert!(check_xml_name("1foo").is_err());
    }

    #[test]
    fn xml_name_accepts_trailing_digit_and_hyphen() {
        assert!(check_xml_name("foo1").is_ok());
        assert!(check_xml_name("foo-1").is_ok());
    }

    #[test]
    fn xml_name_rejects_internal_whitespace() {
        assert!(check_xml_name("foo bar").is_err());
    }

    #[test]
    fn xml_name_rejects_leading_hyphen() {
        assert!(check_xml_name("-foo").is_err());
    }

    // -- check_svg_pathdata --

    #[test]
    fn svg_pathdata_accepts_empty() {
        assert!(check_svg_pathdata("").is_ok());
        assert!(check_svg_pathdata("   ").is_ok());
    }

    #[test]
    fn svg_pathdata_accepts_simple_moveto_lineto_closepath() {
        assert!(check_svg_pathdata("M10 10 L20 20 Z").is_ok());
        assert!(check_svg_pathdata("m10,10 l20,20 z").is_ok());
    }

    #[test]
    fn svg_pathdata_accepts_implicit_lineto_repetition_after_moveto() {
        assert!(check_svg_pathdata("M0 0 10 10 20 20").is_ok());
    }

    #[test]
    fn svg_pathdata_accepts_curves_and_arcs() {
        assert!(check_svg_pathdata("M0,0 C10,10 20,20 30,30").is_ok());
        assert!(check_svg_pathdata("M0,0 S10,10 20,20").is_ok());
        assert!(check_svg_pathdata("M0,0 Q10,10 20,20").is_ok());
        assert!(check_svg_pathdata("M0,0 T20,20").is_ok());
        assert!(check_svg_pathdata("M0,0 A5,5 0 0,1 10,10").is_ok());
    }

    #[test]
    fn svg_pathdata_accepts_numbers_without_separators() {
        // The classic SVG path-data compaction: no whitespace/comma
        // needed between a sign/decimal-point and the previous token.
        assert!(check_svg_pathdata("M0,0L.5.5").is_ok());
        assert!(check_svg_pathdata("M0,0l1-1").is_ok());
    }

    #[test]
    fn svg_pathdata_rejects_missing_moveto() {
        assert!(check_svg_pathdata("L10 10").is_err());
    }

    #[test]
    fn svg_pathdata_rejects_unknown_command() {
        assert!(check_svg_pathdata("M0 0 X10 10").is_err());
    }

    #[test]
    fn svg_pathdata_rejects_incomplete_arguments() {
        assert!(check_svg_pathdata("M0 0 L10").is_err());
    }

    #[test]
    fn svg_pathdata_rejects_bad_arc_flag() {
        assert!(check_svg_pathdata("M0,0 A5,5 0 2,1 10,10").is_err());
    }

    #[test]
    fn svg_pathdata_rejects_negative_arc_radius() {
        assert!(check_svg_pathdata("M0,0 A-5,5 0 0,1 10,10").is_err());
    }
}
