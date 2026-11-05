use std::collections::HashMap;
use std::sync::OnceLock;

use crate::util::chars::is_word;

const NAMED: &str = "amp:38,lt:60,gt:62,quot:34,apos:39,nbsp:160,shy:173,iexcl:161,cent:162,pound:163,curren:164,yen:165,sect:167,\
copy:169,laquo:171,not:172,reg:174,deg:176,plusmn:177,sup2:178,sup3:179,micro:181,para:182,middot:183,\
sup1:185,raquo:187,frac14:188,frac12:189,frac34:190,iquest:191,times:215,divide:247,ndash:8211,mdash:8212,\
lsquo:8216,rsquo:8217,sbquo:8218,ldquo:8220,rdquo:8221,bdquo:8222,dagger:8224,Dagger:8225,bull:8226,\
hellip:8230,permil:8240,prime:8242,Prime:8243,lsaquo:8249,rsaquo:8250,euro:8364,trade:8482,larr:8592,\
uarr:8593,rarr:8594,darr:8595,harr:8596,lArr:8656,uArr:8657,rArr:8658,dArr:8659,hArr:8660,forall:8704,\
part:8706,exist:8707,empty:8709,nabla:8711,isin:8712,notin:8713,ni:8715,prod:8719,sum:8721,minus:8722,\
radic:8730,infin:8734,and:8743,or:8744,cap:8745,cup:8746,int:8747,there4:8756,sim:8764,cong:8773,\
asymp:8776,ne:8800,equiv:8801,le:8804,ge:8805,sub:8834,sup:8835,sube:8838,supe:8839,oplus:8853,otimes:8855,\
perp:8869,sdot:8901,loz:9674,spades:9824,clubs:9827,hearts:9829,diams:9830,check:10003,cross:10007,\
AMP:38,LT:60,GT:62,QUOT:34,lsqb:91,rsqb:93,lcub:123,rcub:125,verbar:124,VerticalLine:124,midast:42,\
DiacriticalGrave:96,Hat:94,UnderBar:95,\
equals:61,colon:58,semi:59,num:35,percnt:37,lpar:40,rpar:41,lbrack:91,rbrack:93,lbrace:123,rbrace:125,\
vert:124,sol:47,bsol:92,quest:63,excl:33,ast:42,plus:43,comma:44,period:46,dollar:36,commat:64,grave:96,\
hat:94,lowbar:95,tilde:126,Alpha:913,Beta:914,Gamma:915,Delta:916,Epsilon:917,Zeta:918,Eta:919,Theta:920,\
Iota:921,Kappa:922,Lambda:923,Mu:924,Nu:925,Xi:926,Omicron:927,Pi:928,Rho:929,Sigma:931,Tau:932,\
Upsilon:933,Phi:934,Chi:935,Psi:936,Omega:937,alpha:945,beta:946,gamma:947,delta:948,epsilon:949,zeta:950,\
eta:951,theta:952,iota:953,kappa:954,lambda:955,mu:956,nu:957,xi:958,omicron:959,pi:960,rho:961,sigmaf:962,\
sigma:963,tau:964,upsilon:965,phi:966,chi:967,psi:968,omega:969";

fn named() -> &'static HashMap<&'static str, char> {
    static MAP: OnceLock<HashMap<&'static str, char>> = OnceLock::new();
    MAP.get_or_init(|| {
        NAMED
            .split(',')
            .map(|pair| {
                let (name, code) = pair.split_once(':').expect("name:code pair");
                (
                    name,
                    char::from_u32(code.parse().expect("code point")).expect("scalar value"),
                )
            })
            .collect()
    })
}

fn code_point(n: f64) -> Option<char> {
    if n > 0.0 && n <= 1_114_111.0 && !(55_296.0..=57_343.0).contains(&n) {
        char::from_u32(n as u32)
    } else {
        None
    }
}

const OPEN: [char; 2] = ['ﬂ', '°'];

fn find_open(chars: &[char], from: usize) -> Option<usize> {
    (from..chars.len().saturating_sub(1)).find(|&i| chars[i] == OPEN[0] && chars[i + 1] == OPEN[1])
}

fn restore_placeholders(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut last = 0;
    let mut from = find_open(&chars, 0);
    while let Some(start) = from {
        let Some(stop) = (start + 2..chars.len()).find(|&i| chars[i] == '¶') else {
            break;
        };
        if chars.get(stop + 1) == Some(&'ß') {
            let numeric = chars.get(start + 2) == Some(&'°');
            out.extend(&chars[last..start]);
            out.push_str(if numeric { "&#" } else { "&" });
            out.extend(&chars[start + if numeric { 3 } else { 2 }..stop]);
            out.push(';');
            last = stop + 2;
        }
        from = find_open(&chars, stop + 1);
    }
    if last == 0 {
        return text.to_owned();
    }
    out.extend(&chars[last..]);
    out
}

pub fn written_codes(text: &str) -> String {
    if !text.contains("¶ß") {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == OPEN[0] && chars.get(i + 1) == Some(&OPEN[1]) {
            let mut at = i + 2;
            if chars.get(at) == Some(&'°') {
                at += 1;
            }
            let word = at;
            while at < chars.len() && is_word(chars[at]) {
                at += 1;
            }
            if at > word && chars.get(at) == Some(&'¶') && chars.get(at + 1) == Some(&'ß') {
                out.push('#');
                out.extend(&chars[word..at]);
                out.push(';');
                i = at + 2;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn entity_at(rest: &str) -> Option<(char, usize)> {
    let body = &rest[1..];
    let digits = |s: &str, ok: fn(char) -> bool| s.find(|c| !ok(c)).unwrap_or(s.len());
    let value = |s: &str, radix: u32| {
        s.chars().fold(0.0_f64, |n, c| {
            n * f64::from(radix) + f64::from(c.to_digit(radix).unwrap_or(0))
        })
    };
    if let Some(after) = body.strip_prefix('#') {
        let n = digits(after, |c| c.is_ascii_digit());
        if n > 0 && after[n..].starts_with(';') {
            return code_point(value(&after[..n], 10)).map(|c| (c, 1 + 1 + n + 1));
        }
        if let Some(hex) = after.strip_prefix(['x', 'X']) {
            let n = digits(hex, |c| c.is_ascii_hexdigit());
            if n > 0 && hex[n..].starts_with(';') {
                return code_point(value(&hex[..n], 16)).map(|c| (c, 1 + 2 + n + 1));
            }
        }
        return None;
    }
    let n = digits(body, is_word);
    if n > 0 && body[n..].starts_with(';') {
        return named().get(&body[..n]).map(|&c| (c, 1 + n + 1));
    }
    None
}

pub fn decode_entities(text: &str) -> String {
    let text = if text.contains("¶ß") {
        restore_placeholders(text)
    } else {
        text.to_owned()
    };
    if !text.contains('&') {
        return text;
    }
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while let Some(found) = text[at..].find('&') {
        let start = at + found;
        out.push_str(&text[at..start]);
        match entity_at(&text[start..]) {
            Some((c, len)) => {
                out.push(c);
                at = start + len;
            }
            None => {
                out.push('&');
                at = start + 1;
            }
        }
    }
    out.push_str(&text[at..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_numeric_named_and_hex_entities() {
        assert_eq!(
            decode_entities("a &amp; b &#65; &#x42; &lt;"),
            "a & b A B <"
        );
        assert_eq!(
            decode_entities("&unknown; &#0; &#xD800; &#99999999;"),
            "&unknown; &#0; &#xD800; &#99999999;"
        );
        assert_eq!(decode_entities("&&amp;"), "&&");
    }

    #[test]
    fn restores_placeholders_once() {
        assert_eq!(decode_entities("ﬂ°quot¶ß"), "\"");
        assert_eq!(decode_entities("ﬂ°°35¶ß"), "#");
        assert_eq!(decode_entities("ﬂ°quot ¶"), "ﬂ°quot ¶");
    }

    #[test]
    fn writes_placeholders_back_as_codes() {
        assert_eq!(written_codes("a ﬂ°°35¶ß b ﬂ°quot¶ß"), "a #35; b #quot;");
        assert_eq!(written_codes("no codes"), "no codes");
    }
}
